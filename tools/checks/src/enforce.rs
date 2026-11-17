// Reads Enforce Script sources as text: comments are stripped, and a mask marks string contents so
// that checks match code only.

use regex::{Captures, Regex};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::LazyLock;

// The core mod's RPC class: its Register calls name the handlers.
pub fn rpc_register(core: &str) -> Regex {
    Regex::new(&format!(r"{}RPC\s*\.\s*Register\s*\(", regex::escape(core))).unwrap()
}
pub static IDENTIFIER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[A-Za-z_]\w*$").unwrap());
pub static QUALIFIED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z_]\w*(?:\s*\.\s*[A-Za-z_]\w*)+$").unwrap());
pub static STRING_LITERAL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^"(?:\\.|[^"\\])*"$"#).unwrap());

pub fn check_mf() -> Result<(), String> {
    // /usr/bin/mf is METAFONT and waits at a prompt, so it gets no stdin to read.
    let out = Command::new("mf")
        .arg("--version")
        .stdin(Stdio::null())
        .output()
        .map_err(|_| "ModFoundry mf not found on PATH (/usr/bin/mf is METAFONT)".to_string())?;
    let text = String::from_utf8_lossy(&out.stdout);
    let mut chars = text.trim().chars();
    let ok = chars.by_ref().take(3).collect::<String>() == "mf "
        && chars.next().is_some_and(|c| c.is_ascii_digit());
    if ok {
        Ok(())
    } else {
        Err("ModFoundry mf not found first on PATH (/usr/bin/mf is METAFONT)".to_string())
    }
}

// Reads options the way argparse does: "--" ends them, a unique prefix of an option name is that
// option, --name=value works, and -h or --help prints the usage whatever else is given.
pub struct Arguments {
    values: HashMap<String, String>,
    pub positional: Vec<String>,
}

impl Arguments {
    pub fn value(&self, option: &str) -> Option<&str> {
        self.values.get(option).map(String::as_str)
    }
}

fn is_option(arg: &str, long: &str) -> bool {
    arg.len() >= 3 && long.starts_with(arg)
}

pub fn parse_options(
    args: &[String],
    usage: &str,
    options: &[&str],
    positionals: usize,
) -> Result<Arguments, u8> {
    parse_options_range(args, usage, options, positionals, positionals)
}

// parse_options for a command whose positional arguments may be left out: min to max of them.
pub fn parse_options_range(
    args: &[String],
    usage: &str,
    options: &[&str],
    min: usize,
    max: usize,
) -> Result<Arguments, u8> {
    let mut parsed = Arguments {
        values: HashMap::new(),
        positional: Vec::new(),
    };
    let mut unknown = Vec::new();
    let mut help = false;
    let mut ended = false;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        let name = arg.split('=').next().unwrap_or("");
        if ended || !arg.starts_with('-') || arg == "-" {
            parsed.positional.push(arg.clone());
        } else if arg == "--" {
            ended = true;
        } else if arg == "-h" || is_option(name, "--help") {
            help = true;
        } else if let Some(option) = options.iter().find(|option| is_option(name, option)) {
            let value = match arg.split_once('=') {
                Some((_, value)) => value.to_string(),
                None => match rest.next() {
                    Some(value) => value.clone(),
                    None => {
                        eprintln!("{usage}\nerror: argument {option}: expected one argument");
                        return Err(2);
                    }
                },
            };
            parsed.values.insert(option.to_string(), value);
        } else {
            unknown.push(arg.clone());
        }
    }
    if help {
        println!("{usage}");
        return Err(0);
    }
    if !unknown.is_empty() {
        eprintln!(
            "{usage}\nerror: unrecognized arguments: {}",
            unknown.join(" ")
        );
        return Err(2);
    }
    if parsed.positional.len() < min || parsed.positional.len() > max {
        let expected = if min == max {
            min.to_string()
        } else {
            format!("{min} to {max}")
        };
        eprintln!("{usage}\nerror: expected {expected} positional arguments");
        return Err(2);
    }
    Ok(parsed)
}

pub fn read_text(path: &Path, rel: &str) -> Result<String, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("{rel}: cannot read: {}", py_os_error(&error, path)))?;
    let text = String::from_utf8_lossy(&bytes);
    Ok(text.replace("\r\n", "\n").replace('\r', "\n"))
}

static DECIMAL_DIGIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\p{Nd}$").unwrap());

// The text with every Unicode decimal digit written as its ASCII digit, as Python's int() reads them.
// Digits come in runs of ten code points from a zero, so a digit's value is its offset from the
// zero that starts its run.
pub fn ascii_digits(text: &str) -> String {
    text.chars()
        .map(|c| {
            if is_decimal(c) {
                char::from_digit(decimal_value(c), 10).unwrap()
            } else {
                c
            }
        })
        .collect()
}

fn is_decimal(c: char) -> bool {
    DECIMAL_DIGIT.is_match(c.encode_utf8(&mut [0; 4]))
}

fn decimal_value(c: char) -> u32 {
    let mut zero = c as u32;
    while zero > 0 && char::from_u32(zero - 1).is_some_and(is_decimal) {
        zero -= 1;
    }
    (c as u32 - zero) % 10
}

static INT_LITERAL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^-?\d+$").unwrap());

// A name a message or a constant resolves to: a string, or a number kept as its decimal text.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Name {
    Text(String),
    Number(String),
}

// The constants of a mod by name; a name given two values maps to None.
pub type Names = HashMap<String, Option<Name>>;

// The name an argument stands for: a string or integer literal, or a constant named directly or as
// the last part of a qualified name.
pub fn resolve_name(argument: &str, names: Option<&Names>) -> Option<Name> {
    let argument = argument.trim();
    if STRING_LITERAL.is_match(argument) {
        return Some(Name::Text(argument[1..argument.len() - 1].to_string()));
    }
    if INT_LITERAL.is_match(argument) {
        return Some(Name::Number(int_text(&ascii_digits(argument))));
    }
    if IDENTIFIER.is_match(argument) {
        return lookup(names, argument);
    }
    if QUALIFIED.is_match(argument) {
        return lookup(names, argument.rsplit('.').next().unwrap_or("").trim());
    }
    None
}

fn lookup(names: Option<&Names>, name: &str) -> Option<Name> {
    names.and_then(|names| names.get(name)).cloned().flatten()
}

pub fn after_word_or_dot(prev: Option<char>, _next: Option<char>) -> bool {
    !prev.is_some_and(|c| is_word(c) || c == '.')
}

pub fn bare_word(prev: Option<char>, next: Option<char>) -> bool {
    !prev.is_some_and(is_word) && !next.is_some_and(is_word)
}

pub fn anywhere(_prev: Option<char>, _next: Option<char>) -> bool {
    true
}

// A decimal integer of any size, with leading zeros dropped and a zero with no sign, so it compares as a value.
pub fn int_text(digits: &str) -> String {
    let (sign, digits) = match digits.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", digits),
    };
    let trimmed = digits.trim_start_matches('0');
    if trimmed.is_empty() {
        "0".to_string()
    } else {
        format!("{sign}{trimmed}")
    }
}

pub fn line_of(text: &str, position: usize) -> usize {
    text.as_bytes()[..position]
        .iter()
        .filter(|&&byte| byte == b'\n')
        .count()
        + 1
}

// A string literal ends at its closing quote; a backslash escapes the next character.
// Strings in comments-stripped code may span lines; masked strings may not escape a newline.
fn string_end(bytes: &[u8], start: usize, across_lines: bool) -> Option<usize> {
    let mut at = start + 1;
    while at < bytes.len() {
        match bytes[at] {
            b'\\' => {
                if at + 1 >= bytes.len() || (!across_lines && bytes[at + 1] == b'\n') {
                    return None;
                }
                at += 2;
            }
            b'"' => return Some(at + 1),
            _ => at += 1,
        }
    }
    None
}

pub fn strip_comments(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'"' {
            if let Some(end) = string_end(bytes, at, true) {
                out.extend_from_slice(&bytes[at..end]);
                at = end;
                continue;
            }
        } else if bytes[at..].starts_with(b"//") {
            let end = bytes[at..]
                .iter()
                .position(|&byte| byte == b'\n')
                .map_or(bytes.len(), |p| at + p);
            out.extend(bytes[at..end].iter().map(|&byte| blank(byte)));
            at = end;
            continue;
        } else if bytes[at..].starts_with(b"/*") {
            if let Some(close) = find_bytes(bytes, at + 2, b"*/") {
                out.extend(bytes[at..close + 2].iter().map(|&byte| blank(byte)));
                at = close + 2;
                continue;
            }
        }
        out.push(bytes[at]);
        at += 1;
    }
    String::from_utf8(out).unwrap()
}

fn blank(byte: u8) -> u8 {
    if byte == b'\n' {
        b'\n'
    } else {
        b' '
    }
}

fn find_bytes(bytes: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    bytes[from..]
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|p| from + p)
}

pub fn mask_strings(code: &str) -> String {
    mask_string_literals(code, false)
}

// Also masks a string that a backslash carries over a line break.
pub fn mask_strings_across_lines(code: &str) -> String {
    mask_string_literals(code, true)
}

fn mask_string_literals(code: &str, across_lines: bool) -> String {
    let bytes = code.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'"' {
            if let Some(end) = string_end(bytes, at, across_lines) {
                out.extend(std::iter::repeat_n(b' ', end - at));
                at = end;
                continue;
            }
        }
        out.push(bytes[at]);
        at += 1;
    }
    String::from_utf8(out).unwrap()
}

pub fn block(mask: &str, open_at: usize) -> Option<usize> {
    matching(mask, open_at, b'{', b'}')
}

pub fn paren(mask: &str, open_at: usize) -> Option<usize> {
    matching(mask, open_at, b'(', b')')
}

fn matching(mask: &str, open_at: usize, open: u8, close: u8) -> Option<usize> {
    let mut depth = 0;
    for (index, &byte) in mask.as_bytes().iter().enumerate().skip(open_at) {
        if byte == open {
            depth += 1;
        } else if byte == close {
            depth -= 1;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

pub fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

pub fn prev_char(text: &str, at: usize) -> Option<char> {
    text[..at].chars().next_back()
}

pub fn next_char(text: &str, at: usize) -> Option<char> {
    text[at..].chars().next()
}

pub fn count(number: usize, one: &str, many: &str) -> String {
    format!("{number} {}", if number == 1 { one } else { many })
}

// str(OSError) as Python prints it: "[Errno 13] Permission denied: '/path'".
pub fn py_os_error(error: &std::io::Error, path: &Path) -> String {
    let code = error.raw_os_error().unwrap_or(0);
    let text = error.to_string();
    let strerror = match text.rfind(" (os error ") {
        Some(at) => &text[..at],
        None => text.as_str(),
    };
    format!("[Errno {code}] {strerror}: '{}'", path.display())
}

fn walk_error(error: &std::io::Error, path: &Path) -> String {
    format!("cannot read directory: {}", py_os_error(error, path))
}

// The files under a folder, the way the checks' own walk reads them: symbolic links to folders are
// entered, a folder reached again through its own links is a cannot-run, and a folder named server
// is skipped when it sits at depth skip_server_at.
type Pending = (PathBuf, usize, Vec<(u64, u64)>);

pub fn walk_linked(start: &Path, skip_server_at: Option<usize>) -> Result<Vec<PathBuf>, String> {
    use std::os::unix::fs::MetadataExt;
    let mut found = Vec::new();
    let mut stack: Vec<Pending> = vec![(start.to_path_buf(), 0, Vec::new())];
    while let Some((current, depth, ancestors)) = stack.pop() {
        let meta = fs::metadata(&current).map_err(|error| walk_error(&error, &current))?;
        let key = (meta.dev(), meta.ino());
        if ancestors.contains(&key) {
            return Err(format!("{}: link cycle", current.display()));
        }
        let mut chain = ancestors;
        chain.push(key);
        let entries = match fs::read_dir(&current) {
            Ok(entries) => sorted_entries(entries),
            Err(error) => return Err(walk_error(&error, &current)),
        };
        let mut children = Vec::new();
        for path in entries {
            let target = match fs::metadata(&path) {
                Ok(target) => Some(target),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => return Err(walk_error(&error, &path)),
            };
            match target {
                Some(target) if target.is_dir() => {
                    let name = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_lowercase();
                    if skip_server_at == Some(depth) && name == "server" {
                        continue;
                    }
                    children.push((path, depth + 1, chain.clone()));
                }
                Some(target) if target.is_file() => found.push(path),
                _ => {}
            }
        }
        stack.extend(children.into_iter().rev());
    }
    Ok(found)
}

// os.walk: folders are not entered through links. A folder that cannot be read is an error when
// raise is set and is skipped otherwise.
pub fn walk_tree(start: &Path, raise: bool) -> Result<Vec<PathBuf>, String> {
    let mut found = Vec::new();
    let mut stack = vec![start.to_path_buf()];
    while let Some(current) = stack.pop() {
        let entries = match fs::read_dir(&current) {
            Ok(entries) => sorted_entries(entries),
            Err(error) if raise => return Err(walk_error(&error, &current)),
            Err(_) => continue,
        };
        let mut folders = Vec::new();
        for path in entries {
            let is_dir = fs::metadata(&path).is_ok_and(|target| target.is_dir());
            if is_dir {
                let is_link =
                    fs::symlink_metadata(&path).is_ok_and(|meta| meta.file_type().is_symlink());
                if !is_link {
                    folders.push(path);
                }
            } else {
                found.push(path);
            }
        }
        stack.extend(folders.into_iter().rev());
    }
    Ok(found)
}

// The entries of a folder, sorted by name.
fn sorted_entries(entries: fs::ReadDir) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    paths.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    paths
}

// The names in a folder, sorted; an unreadable folder is a cannot-run naming it.
pub fn list_dir(path: &Path) -> Result<Vec<String>, String> {
    let entries = fs::read_dir(path).map_err(|error| {
        format!(
            "{}: cannot read: {}",
            path.display(),
            py_os_error(&error, path)
        )
    })?;
    let mut names: Vec<String> = entries
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    Ok(names)
}

// For a root whose mods folder is there but holds no mod: prints the skip line and gives exit 0.
// A missing folder is not a skip. An entry that cannot be read counts as a mod, so the check reports it.
pub fn skip_without_mods(root: &Path) -> Option<u8> {
    let mods = root.join("mods");
    let has_mod = fs::read_dir(&mods).ok()?.any(|entry| match entry {
        Err(_) => true,
        Ok(entry) => match entry.file_type() {
            Ok(kind) if kind.is_symlink() => entry.path().is_dir(),
            Ok(kind) => kind.is_dir(),
            Err(_) => true,
        },
    });
    if has_mod {
        return None;
    }
    println!("skipped: no mods under {} yet", mods.display());
    Some(0)
}

// A path under root as root-relative text with forward slashes; a path outside root is kept whole.
pub fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

// The .c, .layout and config.cpp files under mods/ and tests/, as the path and its root-relative name.
pub fn mod_source_files(root: &Path) -> Result<Vec<(PathBuf, String)>, String> {
    let mut found = Vec::new();
    for parent in ["mods", "tests"] {
        let directory = root.join(parent);
        if !directory.is_dir() {
            continue;
        }
        for path in walk_linked(&directory, Some(1))? {
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            if name.ends_with(".c") || name.ends_with(".layout") || name == "config.cpp" {
                let rel = format!("{parent}/{}", relative_path(&directory, &path));
                found.push((path, rel));
            }
        }
    }
    Ok(found)
}

// Comments are removed and their newlines kept, so the lines still count the same; strings are kept.
// With config set, a string ends at the next quote, with no escapes.
pub fn strip_comments_keep_lines(text: &str, config: bool) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'"' {
            let end = if config {
                find_bytes(bytes, at + 1, b"\"").map(|close| close + 1)
            } else {
                string_end(bytes, at, true)
            };
            if let Some(end) = end {
                out.extend_from_slice(&bytes[at..end]);
                at = end;
                continue;
            }
        } else if bytes[at..].starts_with(b"//") {
            at = bytes[at..]
                .iter()
                .position(|&byte| byte == b'\n')
                .map_or(bytes.len(), |p| at + p);
            continue;
        } else if bytes[at..].starts_with(b"/*") {
            if let Some(close) = find_bytes(bytes, at + 2, b"*/") {
                out.extend(std::iter::repeat_n(
                    b'\n',
                    bytes[at..close + 2].iter().filter(|&&b| b == b'\n').count(),
                ));
                at = close + 2;
                continue;
            }
        }
        out.push(bytes[at]);
        at += 1;
    }
    String::from_utf8(out).unwrap()
}

// The arguments of the call whose open paren is at open_at, split at commas outside (), [] and {}
// Text comes from code and delimiters from mask, with nothing trimmed.
pub fn call_args(code: &str, mask: &str, open_at: usize) -> Vec<String> {
    let mut args = Vec::new();
    let mut depth = [0usize; 3];
    let mut start = open_at + 1;
    for (index, byte) in mask.bytes().enumerate().skip(open_at + 1) {
        match byte {
            b'(' => depth[0] += 1,
            b'[' => depth[1] += 1,
            b'{' => depth[2] += 1,
            b')' | b']' | b'}' => {
                let slot = match byte {
                    b')' => 0,
                    b']' => 1,
                    _ => 2,
                };
                if depth[slot] == 0 {
                    if byte == b')' {
                        args.push(code[start..index].to_string());
                        return args;
                    }
                    continue;
                }
                depth[slot] -= 1;
            }
            b',' if depth.iter().all(|&open| open == 0) => {
                args.push(code[start..index].to_string());
                start = index + 1;
            }
            _ => {}
        }
    }
    args.push(code[start..].to_string());
    args
}

// The matches of re from start to end, as captures, that pass ok(char before, char after).
// The regex crate has no lookaround, so lookbehind and lookahead run here: a rejected match
// moves the search one character on, and text past end counts as the end of the string.
pub fn bounded_captures<'t>(
    re: &Regex,
    text: &'t str,
    start: usize,
    end: usize,
    ok: impl Fn(Option<char>, Option<char>) -> bool,
) -> Vec<Captures<'t>> {
    let haystack = &text[..end];
    let mut found = Vec::new();
    let mut at = start;
    while let Some(caps) = re.captures_at(haystack, at) {
        let whole = caps.get(0).expect("group 0 always matches");
        if ok(
            prev_char(text, whole.start()),
            next_char(haystack, whole.end()),
        ) {
            at = whole.end();
            found.push(caps);
        } else {
            at = whole.start()
                + haystack[whole.start()..]
                    .chars()
                    .next()
                    .map_or(1, char::len_utf8);
        }
    }
    found
}

// A character outside ASCII, or one of the separators 0x1c to 0x1f. Python's \s and strip() count the
// separators as whitespace, and the word classes of the two languages differ for other characters.
pub fn odd_char(c: char) -> bool {
    !c.is_ascii() || ('\u{1c}'..='\u{1f}').contains(&c)
}

// The 1-based lines of the code that hold an odd character outside its masked strings and comments.
// The lines come from the code, because the mask blanks a newline inside a string.
pub fn odd_lines(code: &str, mask: &str) -> Vec<usize> {
    let mut lines: Vec<usize> = Vec::new();
    for (at, c) in mask.char_indices() {
        if odd_char(c) && !(at == 0 && c == '\u{feff}') {
            let line = line_of(code, at);
            if lines.last() != Some(&line) {
                lines.push(line);
            }
        }
    }
    lines
}

// One Enforce file under a mod's Scripts folder: the text with comments removed and the mask of its
// strings. Both keep the same byte offsets.
pub struct ScriptText {
    pub rel: String,
    pub code: String,
    pub mask: String,
}

pub struct Scripts {
    pub texts: Vec<ScriptText>,
}

// The .c files of every mod's Scripts folder, in os.walk order: each mod by name, each Scripts folder
// in turn, then a folder's files by name and its subfolders by name, with no link entered. Nothing is
// outlined, so broken code is read as text. With strict_mod_stat unset, a mod whose folder cannot be
// examined is skipped, as os.path.isdir does. Strings are masked across a backslash line break when
// across_lines is set.
pub fn script_texts(
    mods_root: &Path,
    shown: &str,
    strict_mod_stat: bool,
    across_lines: bool,
) -> Result<Scripts, String> {
    if !mods_root.is_dir() {
        return Err(format!("no mods under {shown}"));
    }
    let names = listing(mods_root).map_err(|error| {
        format!(
            "{}: cannot read: {}",
            mods_root.display(),
            py_os_error(&error, mods_root)
        )
    })?;
    let mut mods = Vec::new();
    for name in names {
        let path = mods_root.join(&name);
        match fs::metadata(&path) {
            Ok(meta) if meta.is_dir() => mods.push(name),
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) if strict_mod_stat => {
                return Err(format!(
                    "{}: cannot read: {}",
                    path.display(),
                    py_os_error(&error, &path)
                ))
            }
            Err(_) => {}
        }
    }
    if mods.is_empty() {
        return Err(format!("no mods under {shown}"));
    }
    let mut texts = Vec::new();
    for mod_id in mods {
        let mod_root = mods_root.join(&mod_id);
        let names = listing(&mod_root).map_err(|error| {
            format!("{mod_id}: cannot read: {}", py_os_error(&error, &mod_root))
        })?;
        let scripts: Vec<String> = names
            .into_iter()
            .filter(|name| name.to_lowercase() == "scripts")
            .collect();
        for name in scripts {
            let shown_folder = format!("mods/{mod_id}/{name}");
            read_scripts(
                &mod_root.join(&name),
                &shown_folder,
                across_lines,
                &mut texts,
            )?;
        }
    }
    Ok(Scripts { texts })
}

// The sorted names in a folder.
fn listing(folder: &Path) -> Result<Vec<String>, std::io::Error> {
    let mut names: Vec<String> = fs::read_dir(folder)?
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    Ok(names)
}

fn read_scripts(
    folder: &Path,
    shown: &str,
    across_lines: bool,
    found: &mut Vec<ScriptText>,
) -> Result<(), String> {
    let entries = fs::read_dir(folder).map_err(|error| walk_error(&error, folder))?;
    let mut files = Vec::new();
    let mut folders = Vec::new();
    for entry in entries {
        let path = entry.map_err(|error| walk_error(&error, folder))?.path();
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        if path.is_dir() {
            folders.push((name, path));
        } else {
            files.push((name, path));
        }
    }
    files.sort();
    folders.sort();
    for (name, path) in files {
        if name.to_lowercase().ends_with(".c") {
            let rel = format!("{shown}/{name}");
            let code = strip_comments(&read_text(&path, &rel)?);
            let mask = if across_lines {
                mask_strings_across_lines(&code)
            } else {
                mask_strings(&code)
            };
            found.push(ScriptText { rel, code, mask });
        }
    }
    for (name, path) in folders {
        let is_link = fs::symlink_metadata(&path).is_ok_and(|meta| meta.file_type().is_symlink());
        if !is_link {
            read_scripts(&path, &format!("{shown}/{name}"), across_lines, found)?;
        }
    }
    Ok(())
}
