use crate::enforce::{self, line_of, read_text, strip_comments, walk_tree};
use crate::family::{self, Family};
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

const ROOT_TAG: &str = "modded_inputs";
static INPUTS_LINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\binputs[\s\x1c-\x1f]*=[\s\x1c-\x1f]*"([^"]*)"([ \t]*;)?"#).unwrap()
});
static GET_INPUT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\bGetInputByName[\s\x1c-\x1f]*\([\s\x1c-\x1f]*"([^"]*)"[\s\x1c-\x1f]*\)"#)
        .unwrap()
});
static RAW_KEY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(?:KeyState|KeyCode)\b").unwrap());

struct Inputs {
    mod_id: String,
    rel: String,
    actions: Vec<String>,
    sorted: BTreeSet<String>,
    presets: BTreeSet<String>,
    groups: Vec<String>,
    locs: Vec<String>,
}

pub fn run(args: &[String]) -> u8 {
    let parsed =
        match enforce::parse_options_range(args, "usage: checks check-inputs [root]", &[], 0, 1) {
            Ok(parsed) => parsed,
            Err(code) => return code,
        };
    let root = PathBuf::from(parsed.positional.first().map_or(".", String::as_str));
    if !root.is_dir() {
        println!("cannot run: root not found: {}", root.display());
        return 2;
    }
    let mods_dir = root.join("mods");
    if !mods_dir.is_dir() {
        println!("cannot run: no mods under {}", root.display());
        return 2;
    }
    let family = match family::load_or_report(&root) {
        Ok(family) => family,
        Err(code) => return code,
    };
    let mut problems = Vec::new();
    let parsed_inputs = read_mods(&mods_dir, &root, &mut problems);
    let declared = inputs_problems(&parsed_inputs, &mods_dir, &root, &family, &mut problems);
    let files = match scripts(&mods_dir, &root, &mut problems) {
        Ok(files) => files,
        Err(error) => {
            println!("cannot run: {error}");
            return 2;
        }
    };
    script_problems(&files, &declared, &family, &mut problems);

    if let Some(code) = super::failed(&problems) {
        return code;
    }
    println!(
        "OK: {} inputs files, {} actions, {} scripts",
        parsed_inputs.len(),
        declared.len(),
        files.len()
    );
    0
}

// The path with . and .. collapsed, as os.path.normpath gives it.
fn normpath(path: &str) -> String {
    let absolute = path.starts_with('/');
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.last().is_some_and(|last| *last != "..") {
                    parts.pop();
                } else if !absolute {
                    parts.push("..");
                }
            }
            other => parts.push(other),
        }
    }
    let joined = parts.join("/");
    match (absolute, joined.is_empty()) {
        (true, _) => format!("/{joined}"),
        (false, true) => ".".to_string(),
        (false, false) => joined,
    }
}

fn abspath(path: &Path) -> String {
    let text = path.display().to_string();
    if text.starts_with('/') {
        return normpath(&text);
    }
    let cwd = std::env::current_dir()
        .map(|dir| dir.display().to_string())
        .unwrap_or_default();
    normpath(&format!("{cwd}/{text}"))
}

// os.path.relpath(path, start), with both made absolute first.
fn relpath(path: &Path, start: &Path) -> String {
    let target = abspath(path);
    let base = abspath(start);
    let target: Vec<&str> = target.split('/').filter(|part| !part.is_empty()).collect();
    let base: Vec<&str> = base.split('/').filter(|part| !part.is_empty()).collect();
    let common = target.iter().zip(&base).take_while(|(a, b)| a == b).count();
    let mut parts: Vec<&str> = vec![".."; base.len() - common];
    parts.extend(&target[common..]);
    if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    }
}

fn read_mods(mods_dir: &Path, root: &Path, problems: &mut Vec<String>) -> Vec<Inputs> {
    let folders = match enforce::list_dir(mods_dir) {
        Ok(names) => names
            .into_iter()
            .filter(|name| mods_dir.join(name).is_dir())
            .collect::<Vec<_>>(),
        Err(error) => {
            problems.push(format!("cannot read {}: {error}", relpath(mods_dir, root)));
            return Vec::new();
        }
    };
    let mut parsed = Vec::new();
    for mod_id in folders {
        let mod_root = mods_dir.join(&mod_id);
        let config = mod_root.join("config.cpp");
        if !config.is_file() {
            continue;
        }
        let config_rel = relpath(&config, root);
        let text = match read_text(&config, &config_rel) {
            Ok(text) => strip_comments(&text),
            Err(error) => {
                problems.push(error);
                continue;
            }
        };
        let Some(line) = INPUTS_LINE.captures(&text) else {
            continue;
        };
        if line.get(2).is_none() {
            problems.push(format!("{config_rel}: the inputs line has no ; after it"));
            continue;
        }
        let named = line[1].to_string();
        let named_slash = named.replace('\\', "/");
        if !named_slash.starts_with(&format!("{mod_id}/")) {
            problems.push(format!(
                "{config_rel}: inputs names {named}, which doesn't start with {mod_id}/"
            ));
            continue;
        }
        let path_text = normpath(&format!(
            "{}/{}",
            mod_root.display(),
            &named_slash[mod_id.len() + 1..]
        ));
        if !inside(&path_text, &mod_root) {
            problems.push(format!(
                "{config_rel}: inputs names {named}, which is outside mods/{mod_id}"
            ));
            continue;
        }
        let path = PathBuf::from(&path_text);
        if !path.is_file() {
            problems.push(format!(
                "{config_rel}: inputs names {named} which is not there"
            ));
            continue;
        }
        let rel = relpath(&path, root);
        if let Some(found) = parse(&path, &mod_id, &rel, problems) {
            parsed.push(found);
        }
    }
    parsed
}

fn inside(path: &str, folder: &Path) -> bool {
    let path = abspath(Path::new(path));
    let folder = abspath(folder);
    path == folder || path.starts_with(&format!("{folder}/"))
}

fn parse(path: &Path, mod_id: &str, rel: &str, problems: &mut Vec<String>) -> Option<Inputs> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            problems.push(format!(
                "{rel}: cannot read: {}",
                enforce::py_os_error(&error, path)
            ));
            return None;
        }
    };
    let element = match xml::parse(&bytes) {
        Ok(element) => element,
        Err(error) => {
            problems.push(format!("{rel}: is not valid XML: {error}"));
            return None;
        }
    };
    if element.tag != ROOT_TAG {
        problems.push(format!("{rel}: root is {}, not {ROOT_TAG}", element.tag));
        return None;
    }
    let mut found = Inputs {
        mod_id: mod_id.to_string(),
        rel: rel.to_string(),
        actions: Vec::new(),
        sorted: BTreeSet::new(),
        presets: BTreeSet::new(),
        groups: Vec::new(),
        locs: Vec::new(),
    };
    for inputs in element.children_named("inputs") {
        for block in inputs.children_named("actions") {
            for action in block.children_named("input") {
                let name = action.attr("name").unwrap_or("").to_string();
                let loc = action.attr("loc").unwrap_or("").to_string();
                if loc.is_empty() {
                    problems.push(format!("{rel}: action {name} has no loc"));
                }
                found.actions.push(name);
                found.locs.push(loc);
            }
        }
        for group in inputs.children_named("sorting") {
            let name = group.attr("name").unwrap_or("").to_string();
            let loc = group.attr("loc").unwrap_or("").to_string();
            found.groups.push(name.clone());
            found.locs.push(loc.clone());
            if loc.is_empty() {
                problems.push(format!("{rel}: sorting {name} has no loc"));
            }
            for action in group.children_named("input") {
                found
                    .sorted
                    .insert(action.attr("name").unwrap_or("").to_string());
            }
        }
    }
    for preset in element.children_named("preset") {
        for action in preset.children_named("input") {
            found
                .presets
                .insert(action.attr("name").unwrap_or("").to_string());
        }
    }
    Some(found)
}

// The csv module's default field size limit.
const CSV_FIELD_LIMIT: usize = 131_072;

// The keys of a stringtable: the first column of each row, stripped, or None when it cannot be read.
fn stringtable_keys(path: &Path, rel: &str, problems: &mut Vec<String>) -> BTreeSet<String> {
    if !path.is_file() {
        return BTreeSet::new();
    }
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            problems.push(format!(
                "{rel}: cannot read: {}",
                enforce::py_os_error(&error, path)
            ));
            return BTreeSet::new();
        }
    };
    let Ok(text) = std::str::from_utf8(&bytes) else {
        problems.push(format!("{rel}: cannot read: not UTF-8"));
        return BTreeSet::new();
    };
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let rows = csv::rows(text);
    if rows
        .iter()
        .flatten()
        .any(|field| field.chars().count() > CSV_FIELD_LIMIT)
    {
        problems.push(format!(
            "{rel}: cannot read: _csv.Error: field larger than field limit ({CSV_FIELD_LIMIT})"
        ));
        return BTreeSet::new();
    }
    rows.into_iter()
        .filter_map(|row| row.first().map(|first| first.trim().to_string()))
        .filter(|key| !key.is_empty())
        .collect()
}

fn inputs_problems(
    parsed: &[Inputs],
    mods_dir: &Path,
    root: &Path,
    family: &Family,
    problems: &mut Vec<String>,
) -> BTreeSet<String> {
    let mods: BTreeSet<&str> = parsed.iter().map(|entry| entry.mod_id.as_str()).collect();
    let mut keys: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for mod_id in mods {
        let path = mods_dir.join(mod_id).join("stringtable.csv");
        let rel = relpath(&path, root);
        keys.insert(mod_id.to_string(), stringtable_keys(&path, &rel, problems));
    }
    let core_path = mods_dir.join(&family.core).join("stringtable.csv");
    let core = stringtable_keys(&core_path, &relpath(&core_path, root), problems);
    let word = family.word();
    let mut seen: BTreeMap<String, String> = BTreeMap::new();

    for entry in parsed {
        let actions: BTreeSet<&str> = entry.actions.iter().map(String::as_str).collect();
        if entry.actions.is_empty() {
            problems.push(format!("{}: declares no actions", entry.rel));
        }
        for name in &entry.sorted {
            if !actions.contains(name.as_str()) {
                problems.push(format!(
                    "{}: sorting names {name}, which no action declares",
                    entry.rel
                ));
            }
        }
        for name in &entry.presets {
            if !actions.contains(name.as_str()) {
                problems.push(format!(
                    "{}: preset names {name}, which no action declares",
                    entry.rel
                ));
            }
        }
        for name in &entry.actions {
            if !name.starts_with(&format!("UA{}", entry.mod_id)) {
                problems.push(format!(
                    "{}: action {name} is not named UA{}",
                    entry.rel, entry.mod_id
                ));
            }
            if let Some(other) = seen.get(name) {
                if other != &entry.rel {
                    problems.push(format!("{}: {name} is also declared in {other}", entry.rel));
                }
            }
            seen.entry(name.clone())
                .or_insert_with(|| entry.rel.clone());
            if !entry.sorted.contains(name) {
                problems.push(format!(
                    "{}: action {name} is not in a sorting block",
                    entry.rel
                ));
            }
        }
        for group in &entry.groups {
            if *group != word {
                problems.push(format!(
                    "{}: sorting name is {group}, not {word}",
                    entry.rel
                ));
            }
        }
        let own = keys.get(&entry.mod_id).cloned().unwrap_or_default();
        for loc in &entry.locs {
            if !loc.is_empty() && !own.contains(loc) && !core.contains(loc) {
                problems.push(format!("{}: {loc} is in no stringtable", entry.rel));
            }
        }
    }
    seen.into_keys().collect()
}

fn scripts(
    mods_dir: &Path,
    root: &Path,
    problems: &mut Vec<String>,
) -> Result<Vec<(String, String)>, String> {
    let mut found = Vec::new();
    for mod_id in enforce::list_dir(mods_dir)? {
        let mod_root = mods_dir.join(&mod_id);
        if !mod_root.is_dir() {
            continue;
        }
        for path in walk_tree(&mod_root, false)? {
            if !path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase()
                .ends_with(".c")
            {
                continue;
            }
            let rel = relpath(&path, root);
            match read_text(&path, &rel) {
                Ok(text) => found.push((rel, strip_comments(&text))),
                Err(error) => problems.push(error),
            }
        }
    }
    Ok(found)
}

fn script_problems(
    files: &[(String, String)],
    declared: &BTreeSet<String>,
    family: &Family,
    problems: &mut Vec<String>,
) {
    let allowed = format!("mods/{0}/Scripts/5_Mission/{0}Keys.c", family.core);
    let action_prefix = format!("UA{}", family.prefix);
    for (rel, text) in files {
        for found in RAW_KEY.find_iter(text) {
            if *rel == allowed {
                continue;
            }
            problems.push(format!(
                "{rel}:{}: {} is not allowed: a player-facing key is rebindable",
                line_of(text, found.start()),
                found.as_str()
            ));
        }
        for found in GET_INPUT.captures_iter(text) {
            let name = &found[1];
            if name.starts_with(&action_prefix) && !declared.contains(name) {
                let at = found.get(0).map_or(0, |whole| whole.start());
                problems.push(format!(
                    "{rel}:{}: GetInputByName names {name}, which no inputs file declares",
                    line_of(text, at)
                ));
            }
        }
    }
}

mod csv {
    // The rows of a comma-separated file read as Python's csv reader does with its default dialect:
    // quoted fields may hold commas and line breaks, a doubled quote is a quote, and a blank line is an empty row.
    pub fn rows(text: &str) -> Vec<Vec<String>> {
        let chars: Vec<char> = text.chars().collect();
        let mut rows = Vec::new();
        let mut row: Vec<String> = Vec::new();
        let mut field = String::new();
        let mut state = State::StartRecord;
        let mut index = 0;
        while index < chars.len() {
            let c = chars[index];
            index += 1;
            state = match state {
                State::StartRecord => match c {
                    '\n' | '\r' => {
                        rows.push(Vec::new());
                        if c == '\r' && chars.get(index) == Some(&'\n') {
                            index += 1;
                        }
                        State::StartRecord
                    }
                    _ => start_field(c, &mut field),
                },
                State::StartField => match c {
                    '"' => State::Quoted,
                    ',' => {
                        row.push(std::mem::take(&mut field));
                        State::StartField
                    }
                    '\n' | '\r' => {
                        end_record(
                            &mut rows,
                            &mut row,
                            &mut field,
                            c,
                            chars.get(index),
                            &mut index,
                        );
                        State::StartRecord
                    }
                    _ => {
                        field.push(c);
                        State::InField
                    }
                },
                State::InField => match c {
                    ',' => {
                        row.push(std::mem::take(&mut field));
                        State::StartField
                    }
                    '\n' | '\r' => {
                        end_record(
                            &mut rows,
                            &mut row,
                            &mut field,
                            c,
                            chars.get(index),
                            &mut index,
                        );
                        State::StartRecord
                    }
                    _ => {
                        field.push(c);
                        State::InField
                    }
                },
                State::Quoted => match c {
                    '"' => State::QuoteInQuoted,
                    _ => {
                        field.push(c);
                        State::Quoted
                    }
                },
                State::QuoteInQuoted => match c {
                    '"' => {
                        field.push('"');
                        State::Quoted
                    }
                    ',' => {
                        row.push(std::mem::take(&mut field));
                        State::StartField
                    }
                    '\n' | '\r' => {
                        end_record(
                            &mut rows,
                            &mut row,
                            &mut field,
                            c,
                            chars.get(index),
                            &mut index,
                        );
                        State::StartRecord
                    }
                    _ => {
                        field.push(c);
                        State::InField
                    }
                },
            };
        }
        match state {
            State::StartRecord => {}
            State::StartField => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
            }
            _ => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
            }
        }
        rows
    }

    #[derive(Clone, Copy)]
    enum State {
        StartRecord,
        StartField,
        InField,
        Quoted,
        QuoteInQuoted,
    }

    fn start_field(c: char, field: &mut String) -> State {
        if c == '"' {
            State::Quoted
        } else if c == ',' {
            State::StartField
        } else {
            field.push(c);
            State::InField
        }
    }

    fn end_record(
        rows: &mut Vec<Vec<String>>,
        row: &mut Vec<String>,
        field: &mut String,
        c: char,
        next: Option<&char>,
        index: &mut usize,
    ) {
        row.push(std::mem::take(field));
        rows.push(std::mem::take(row));
        if c == '\r' && next == Some(&'\n') {
            *index += 1;
        }
    }
}

// A reader for the XML the inputs files use: the elements, their attributes and the well-formedness
// rules that decide whether a file parses. Messages and positions follow the expat library that
// ElementTree uses for the common errors.
mod xml {
    pub struct Element {
        pub tag: String,
        attributes: Vec<(String, String)>,
        children: Vec<Element>,
    }

    impl Element {
        pub fn attr(&self, name: &str) -> Option<&str> {
            self.attributes
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value.as_str())
        }

        pub fn children_named<'a>(
            &'a self,
            name: &'a str,
        ) -> impl Iterator<Item = &'a Element> + 'a {
            self.children.iter().filter(move |child| child.tag == name)
        }
    }

    type Found<T> = Result<T, (String, usize)>;

    // The element tree, or the error text with its line and column.
    pub fn parse(bytes: &[u8]) -> Result<Element, String> {
        let text = match std::str::from_utf8(bytes) {
            Ok(text) => text,
            Err(error) => {
                return Err(located(
                    "not well-formed (invalid token)",
                    bytes,
                    error.valid_up_to(),
                ));
            }
        };
        let mut parser = Parser {
            text,
            bytes: text.as_bytes(),
            pos: 0,
            scopes: Vec::new(),
            ascii: false,
        };
        let result = parser.document();
        // expat reports the first bad character it meets, so it wins over a later error.
        let forbidden = text
            .char_indices()
            .find(|(_, c)| !valid_char(*c as u32))
            .map(|(index, _)| (index, "not well-formed (invalid token)"));
        let body = text.len() - text.trim_start_matches('\u{feff}').len();
        let not_ascii = parser
            .ascii
            .then(|| text[body..].bytes().position(|byte| !byte.is_ascii()))
            .flatten()
            .map(|index| (body + index, "not ASCII"));
        let bad = match (forbidden, not_ascii) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        match (result, bad) {
            (Err((_, at)), Some((bad, message))) if bad < at => Err(located(message, bytes, bad)),
            (Ok(_), Some((bad, message))) => Err(located(message, bytes, bad)),
            (result, _) => result.map_err(|(message, at)| located(&message, bytes, at)),
        }
    }

    fn located(message: &str, bytes: &[u8], at: usize) -> String {
        let line = 1 + bytes[..at].iter().filter(|&&byte| byte == b'\n').count();
        let line_start = bytes[..at]
            .iter()
            .rposition(|&byte| byte == b'\n')
            .map_or(0, |index| index + 1);
        // expat counts characters, not bytes, in a column
        let column = bytes[line_start..at]
            .iter()
            .filter(|&&byte| byte & 0xC0 != 0x80)
            .count();
        format!("{message}: line {line}, column {column}")
    }

    struct Parser<'a> {
        text: &'a str,
        bytes: &'a [u8],
        pos: usize,
        scopes: Vec<Vec<(String, String)>>,
        ascii: bool,
    }

    fn is_name_start(byte: u8) -> bool {
        byte.is_ascii_alphabetic() || byte == b'_' || byte >= 0x80
    }

    fn is_name_char(byte: u8) -> bool {
        is_name_start(byte) || byte.is_ascii_digit() || byte == b'-' || byte == b'.' || byte == b':'
    }

    fn is_token_char(byte: u8) -> bool {
        byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')
    }

    fn is_space(byte: u8) -> bool {
        matches!(byte, b' ' | b'\t' | b'\r' | b'\n')
    }

    fn valid_char(code: u32) -> bool {
        matches!(code, 0x9 | 0xA | 0xD | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF)
    }

    impl<'a> Parser<'a> {
        fn at(&self, offset: usize) -> Option<u8> {
            self.bytes.get(self.pos + offset).copied()
        }

        fn starts(&self, prefix: &str) -> bool {
            self.bytes[self.pos..].starts_with(prefix.as_bytes())
        }

        fn skip_space(&mut self) -> usize {
            let start = self.pos;
            while self.at(0).is_some_and(is_space) {
                self.pos += 1;
            }
            self.pos - start
        }

        // The error for a token cut short by the end of the input is "unclosed token" at its start.
        fn cut_short(&self, at: usize, error: (String, usize)) -> (String, usize) {
            if self.pos >= self.bytes.len() {
                ("unclosed token".to_string(), at)
            } else {
                error
            }
        }

        fn name(&mut self) -> Found<String> {
            let start = self.pos;
            if !self.at(0).is_some_and(is_name_start) {
                return Err(("not well-formed (invalid token)".to_string(), self.pos));
            }
            while self.at(0).is_some_and(is_name_char) {
                self.pos += 1;
            }
            if self.text[start..self.pos].ends_with(':') {
                return Err(("not well-formed (invalid token)".to_string(), self.pos));
            }
            Ok(self.text[start..self.pos].to_string())
        }

        fn document(&mut self) -> Found<Element> {
            if self.bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
                self.pos = 3;
            }
            let declaration_at = self.pos;
            loop {
                self.skip_space();
                if self.starts("<?") {
                    self.processing_instruction(declaration_at)?;
                } else if self.starts("<!--") {
                    self.comment()?;
                } else if self.starts("<!DOCTYPE") {
                    self.doctype()?;
                } else if self.at(0) == Some(b'<') {
                    break;
                } else if self.pos >= self.bytes.len() {
                    return Err(("no element found".to_string(), self.pos));
                } else {
                    return Err(("syntax error".to_string(), self.pos));
                }
            }
            let root = self.element()?;
            loop {
                self.skip_space();
                if self.pos >= self.bytes.len() {
                    return Ok(root);
                } else if self.starts("<?") {
                    self.processing_instruction(usize::MAX)?;
                } else if self.starts("<!--") {
                    self.comment()?;
                } else if self.at(0) == Some(b'<') && self.at(1).is_none() {
                    return Err(("unclosed token".to_string(), self.pos));
                } else if self.at(0) == Some(b'<') && !self.at(1).is_some_and(is_name_start) {
                    return Err(("not well-formed (invalid token)".to_string(), self.pos + 1));
                } else {
                    return Err(("junk after document element".to_string(), self.pos));
                }
            }
        }

        fn processing_instruction(&mut self, declaration_at: usize) -> Found<()> {
            let start = self.pos;
            self.pos += 2;
            let target = self.name()?;
            if target.eq_ignore_ascii_case("xml") && start != declaration_at {
                return Err((
                    "XML or text declaration not at start of entity".to_string(),
                    start,
                ));
            }
            if target == "xml" {
                return self.declaration(start);
            }
            match self.at(0) {
                None => return Err(("unclosed token".to_string(), start)),
                Some(byte) if !is_space(byte) && !self.starts("?>") => {
                    return Err(("not well-formed (invalid token)".to_string(), self.pos));
                }
                _ => {}
            }
            match self.text[self.pos..].find("?>") {
                Some(end) => {
                    self.pos += end + 2;
                    Ok(())
                }
                None => Err(("unclosed token".to_string(), start)),
            }
        }

        // Inputs files are UTF-8; a declared US-ASCII file is checked for ASCII once parsed.
        fn encoding(&mut self, name: &str, at: usize) -> Found<()> {
            match name.to_ascii_lowercase().as_str() {
                "utf-8" | "utf8" => Ok(()),
                "us-ascii" | "ascii" => {
                    self.ascii = true;
                    Ok(())
                }
                _ => Err((
                    format!("unsupported encoding {name} (inputs files are UTF-8)"),
                    at,
                )),
            }
        }

        // The version, then the optional encoding and standalone attributes, of an XML declaration.
        // Without its closing "?>" the declaration is unclosed; otherwise the first fault is reported.
        fn declaration(&mut self, start: usize) -> Found<()> {
            if !self.text[self.pos..].contains("?>") {
                return Err(("unclosed token".to_string(), start));
            }
            let malformed = |at: usize| Err(("XML declaration not well-formed".to_string(), at));
            let mut seen: Vec<&str> = Vec::new();
            loop {
                let spaced = self.skip_space() > 0;
                if self.starts("?>") {
                    if seen.first() != Some(&"version") {
                        return malformed(self.pos);
                    }
                    self.pos += 2;
                    return Ok(());
                }
                if !spaced {
                    return malformed(self.pos);
                }
                let at = self.pos;
                if !self.at(0).is_some_and(|byte| byte.is_ascii_alphabetic()) {
                    return malformed(at);
                }
                let name_start = self.pos;
                while self.at(0).is_some_and(is_token_char) {
                    self.pos += 1;
                }
                let expected = match &self.text[name_start..self.pos] {
                    "version" if seen.is_empty() => Some("version"),
                    "encoding" if seen == ["version"] => Some("encoding"),
                    "standalone" if matches!(seen.last(), Some(&"version") | Some(&"encoding")) => {
                        Some("standalone")
                    }
                    _ => None,
                };
                if self.at(0).is_some_and(is_space) {
                    self.skip_space();
                    if self.at(0) != Some(b'=') {
                        return malformed(self.pos);
                    }
                } else if self.at(0) != Some(b'=') {
                    return malformed(name_start);
                }
                self.pos += 1;
                self.skip_space();
                let quote = match self.at(0) {
                    Some(quote @ (b'"' | b'\'')) => quote,
                    _ => return malformed(self.pos),
                };
                self.pos += 1;
                let value_start = self.pos;
                while self.at(0).is_some_and(is_token_char) {
                    self.pos += 1;
                }
                if self.at(0) != Some(quote) {
                    return malformed(self.pos);
                }
                let value = &self.text[value_start..self.pos];
                self.pos += 1;
                let Some(expected) = expected else {
                    return malformed(at);
                };
                let valid = match expected {
                    "version" => value.strip_prefix("1.").is_some_and(|minor| {
                        !minor.is_empty() && minor.bytes().all(|byte| byte.is_ascii_digit())
                    }),
                    "standalone" => value == "yes" || value == "no",
                    _ => value
                        .as_bytes()
                        .first()
                        .is_some_and(u8::is_ascii_alphabetic),
                };
                if !valid {
                    return malformed(value_start);
                }
                if expected == "encoding" {
                    self.encoding(value, value_start)?;
                }
                seen.push(expected);
            }
        }

        fn comment(&mut self) -> Found<()> {
            let start = self.pos;
            self.pos += 4;
            let rest = &self.text[self.pos..];
            match rest.find("--") {
                None => Err(("unclosed token".to_string(), start)),
                Some(index) if rest[index..].starts_with("-->") => {
                    self.pos += index + 3;
                    Ok(())
                }
                Some(index) => Err((
                    "not well-formed (invalid token)".to_string(),
                    self.pos + index + 2,
                )),
            }
        }

        fn doctype(&mut self) -> Found<()> {
            let start = self.pos;
            let mut depth = 0;
            self.pos += "<!DOCTYPE".len();
            while let Some(byte) = self.at(0) {
                self.pos += 1;
                match byte {
                    b'[' => depth += 1,
                    b']' => depth -= 1,
                    b'>' if depth == 0 => return Ok(()),
                    _ => {}
                }
            }
            Err(("unclosed token".to_string(), start))
        }

        // The value of a reference at the current '&', as the character it stands for.
        fn reference(&mut self) -> Found<char> {
            let start = self.pos;
            self.pos += 1;
            if self.at(0) == Some(b'#') {
                self.pos += 1;
                let hex = self.at(0) == Some(b'x');
                if hex {
                    self.pos += 1;
                }
                let digits_start = self.pos;
                while self.at(0).is_some_and(|byte| {
                    if hex {
                        byte.is_ascii_hexdigit()
                    } else {
                        byte.is_ascii_digit()
                    }
                }) {
                    self.pos += 1;
                }
                let digits = &self.text[digits_start..self.pos];
                if digits.is_empty() || self.at(0) != Some(b';') {
                    return Err(self.cut_short(
                        start,
                        ("not well-formed (invalid token)".to_string(), self.pos),
                    ));
                }
                self.pos += 1;
                let code =
                    u32::from_str_radix(digits, if hex { 16 } else { 10 }).unwrap_or(u32::MAX);
                return match char::from_u32(code) {
                    Some(value) if valid_char(code) => Ok(value),
                    _ => Err(("reference to invalid character number".to_string(), start)),
                };
            }
            let name = self.name().map_err(|error| self.cut_short(start, error))?;
            if self.at(0) != Some(b';') {
                return Err(self.cut_short(
                    start,
                    ("not well-formed (invalid token)".to_string(), self.pos),
                ));
            }
            self.pos += 1;
            match name.as_str() {
                "lt" => Ok('<'),
                "gt" => Ok('>'),
                "amp" => Ok('&'),
                "quot" => Ok('"'),
                "apos" => Ok('\''),
                _ => Err(("undefined entity".to_string(), start)),
            }
        }

        // A reference error in a start tag is reported at the tag, not at the reference.
        fn attribute_value(&mut self, quote: u8, tag: usize) -> Found<String> {
            let mut value = String::new();
            loop {
                match self.at(0) {
                    None => return Err(("unclosed token".to_string(), tag)),
                    Some(byte) if byte == quote => {
                        self.pos += 1;
                        return Ok(value);
                    }
                    Some(b'<') => {
                        return Err(("not well-formed (invalid token)".to_string(), self.pos))
                    }
                    Some(b'&') => {
                        let character = self.reference().map_err(|(message, at)| {
                            if message.starts_with("not well-formed") {
                                (message, at)
                            } else {
                                (message, tag)
                            }
                        })?;
                        value.push(character);
                    }
                    Some(b'\r') if self.at(1) == Some(b'\n') => self.pos += 1,
                    Some(b'\t' | b'\n' | b'\r') => {
                        value.push(' ');
                        self.pos += 1;
                    }
                    Some(byte) if byte < 0x20 => {
                        return Err(("not well-formed (invalid token)".to_string(), self.pos))
                    }
                    Some(_) => {
                        let character = self.text[self.pos..].chars().next().unwrap_or('\u{fffd}');
                        value.push(character);
                        self.pos += character.len_utf8();
                    }
                }
            }
        }

        fn lookup(&self, prefix: &str) -> Option<String> {
            self.scopes
                .iter()
                .rev()
                .flat_map(|scope| scope.iter().rev())
                .find(|(name, _)| name == prefix)
                .map(|(_, uri)| uri.clone())
        }

        // The tag an element name stands for: "{uri}local" in a namespace, the bare name otherwise.
        fn expanded(&self, name: &str, at: usize, attribute: bool) -> Found<String> {
            let (prefix, local) = match name.split_once(':') {
                Some((prefix, local)) => (prefix, local),
                None => ("", name),
            };
            if prefix == "xml" {
                return Ok(format!("{{http://www.w3.org/XML/1998/namespace}}{local}"));
            }
            if prefix.is_empty() && attribute {
                return Ok(local.to_string());
            }
            match self.lookup(prefix) {
                Some(uri) if !uri.is_empty() => Ok(format!("{{{uri}}}{local}")),
                _ if prefix.is_empty() => Ok(local.to_string()),
                _ => Err(("unbound prefix".to_string(), at)),
            }
        }

        fn element(&mut self) -> Found<Element> {
            let start = self.pos;
            self.pos += 1;
            let raw_name = self.name()?;
            let mut raw_attributes: Vec<(String, String, usize)> = Vec::new();
            let empty = loop {
                let spaced = self.skip_space() > 0;
                match self.at(0) {
                    None => return Err(("unclosed token".to_string(), start)),
                    Some(b'/') => {
                        match self.at(1) {
                            Some(b'>') => {}
                            None => return Err(("unclosed token".to_string(), start)),
                            Some(_) => {
                                return Err((
                                    "not well-formed (invalid token)".to_string(),
                                    self.pos + 1,
                                ))
                            }
                        }
                        self.pos += 2;
                        break true;
                    }
                    Some(b'>') => {
                        self.pos += 1;
                        break false;
                    }
                    Some(byte) if is_name_start(byte) => {
                        if !spaced {
                            return Err(("not well-formed (invalid token)".to_string(), self.pos));
                        }
                        let at = self.pos;
                        let name = self.name()?;
                        self.skip_space();
                        match self.at(0) {
                            Some(b'=') => {}
                            None => return Err(("unclosed token".to_string(), start)),
                            Some(_) => {
                                return Err((
                                    "not well-formed (invalid token)".to_string(),
                                    self.pos,
                                ))
                            }
                        }
                        self.pos += 1;
                        self.skip_space();
                        let quote = match self.at(0) {
                            Some(quote @ (b'"' | b'\'')) => quote,
                            None => return Err(("unclosed token".to_string(), start)),
                            _ => {
                                return Err((
                                    "not well-formed (invalid token)".to_string(),
                                    self.pos,
                                ))
                            }
                        };
                        self.pos += 1;
                        let value = self.attribute_value(quote, start)?;
                        if raw_attributes
                            .iter()
                            .any(|(existing, _, _)| existing == &name)
                        {
                            return Err(("duplicate attribute".to_string(), at));
                        }
                        raw_attributes.push((name, value, at));
                    }
                    Some(_) => {
                        return Err(("not well-formed (invalid token)".to_string(), self.pos))
                    }
                }
            };
            let declared: Vec<(String, String)> = raw_attributes
                .iter()
                .filter_map(|(name, value, _)| {
                    if name == "xmlns" {
                        Some((String::new(), value.clone()))
                    } else {
                        name.strip_prefix("xmlns:")
                            .map(|prefix| (prefix.to_string(), value.clone()))
                    }
                })
                .collect();
            self.scopes.push(declared);
            let result = self.body(start, &raw_name, empty, &raw_attributes);
            self.scopes.pop();
            result
        }

        // The children of an element and its resolved names, with the element's namespaces in scope.
        fn body(
            &mut self,
            start: usize,
            raw_name: &str,
            empty: bool,
            raw_attributes: &[(String, String, usize)],
        ) -> Found<Element> {
            let children = if empty {
                Vec::new()
            } else {
                self.children(raw_name)?
            };
            let mut attributes: Vec<(String, String)> = Vec::new();
            for (name, value, at) in raw_attributes {
                if name == "xmlns" || name.starts_with("xmlns:") {
                    continue;
                }
                let key = self.expanded(name, *at, true)?;
                if attributes.iter().any(|(existing, _)| existing == &key) {
                    return Err(("duplicate attribute".to_string(), *at));
                }
                attributes.push((key, value.clone()));
            }
            Ok(Element {
                tag: self.expanded(raw_name, start, false)?,
                attributes,
                children,
            })
        }

        // The content up to and including the end tag that matches the element's start tag.
        fn children(&mut self, raw_name: &str) -> Found<Vec<Element>> {
            let mut children = Vec::new();
            loop {
                match self.at(0) {
                    None => return Err(("no element found".to_string(), self.pos)),
                    Some(b'<') if self.at(1) == Some(b'/') => {
                        let at = self.pos;
                        self.pos += 2;
                        let name_at = self.pos;
                        let closing = self.name().map_err(|error| self.cut_short(at, error))?;
                        self.skip_space();
                        match self.at(0) {
                            Some(b'>') => {}
                            None => return Err(("unclosed token".to_string(), at)),
                            Some(_) => {
                                return Err((
                                    "not well-formed (invalid token)".to_string(),
                                    self.pos,
                                ))
                            }
                        }
                        self.pos += 1;
                        if closing != raw_name {
                            return Err(("mismatched tag".to_string(), name_at));
                        }
                        return Ok(children);
                    }
                    Some(b'<') if self.starts("<!--") => self.comment()?,
                    Some(b'<') if self.starts("<?") => self.processing_instruction(usize::MAX)?,
                    Some(b'<') if self.starts("<![CDATA[") => {
                        match self.text[self.pos..].find("]]>") {
                            Some(end) => self.pos += end + 3,
                            None => {
                                return Err((
                                    "unclosed CDATA section".to_string(),
                                    self.bytes.len(),
                                ))
                            }
                        }
                    }
                    Some(b'<') if self.at(1) == Some(b'!') => {
                        return Err(("not well-formed (invalid token)".to_string(), self.pos + 2));
                    }
                    Some(b'<') => children.push(self.element()?),
                    Some(b'&') => {
                        self.reference()?;
                    }
                    Some(_) => {
                        if self.starts("]]>") {
                            return Err((
                                "not well-formed (invalid token)".to_string(),
                                self.pos + 2,
                            ));
                        }
                        let character = self.text[self.pos..].chars().next().unwrap_or('\u{fffd}');
                        if (character as u32) < 0x20 && !matches!(character, '\t' | '\n' | '\r') {
                            return Err(("not well-formed (invalid token)".to_string(), self.pos));
                        }
                        self.pos += character.len_utf8();
                    }
                }
            }
        }
    }
}
