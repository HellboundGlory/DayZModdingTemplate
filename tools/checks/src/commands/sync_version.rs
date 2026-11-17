use std::cmp::Ordering;
use std::ffi::OsString;
use std::fs;
use std::io::{ErrorKind, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use crate::cli::{self, Opt, Spec};
use crate::enforce;
use crate::pytext;

const SPEC: Spec = Spec {
    command: "sync-version",
    usage: "[-h] [--root ROOT] [--check | --set X.Y.Z | --bump MOD PART]",
    options: &[
        Opt {
            name: "root",
            values: 1,
        },
        Opt {
            name: "check",
            values: 0,
        },
        Opt {
            name: "set",
            values: 1,
        },
        Opt {
            name: "bump",
            values: 2,
        },
    ],
    positional: None,
};

const VERSION: &str = "VERSION";
const MODS: &str = "mods";
const TESTS: &str = "tests";
const TEST_SUFFIX: &str = "Tests";
const PARTS: [&str; 2] = ["minor", "patch"];

const NUMBER: &str = r"(?:0|[1-9][0-9]*)";

static VERSION_SHAPE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"^{NUMBER}\.{NUMBER}\.{NUMBER}$")).unwrap());
static FAMILY_FILE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"^({NUMBER}\.{NUMBER}\.{NUMBER})\r?\n?$")).unwrap());
static VERSION_LINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?m)^([ \t]*version[ \t]*=[ \t]*")([^"\n]*)("[ \t]*;[ \t]*)\r?$"#).unwrap()
});
static ANY_VERSION_LINE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^[ \t]*[vV][eE][rR][sSſ][iIıİ][oO][nN][ \t]*=").unwrap());
static STRING_OR_COMMENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#""[^"\n]*"|//[^\n]*|/\*(?s:.*?)\*/"#).unwrap());

// One number of a version, kept as digits so any size compares and counts the same as Python's int.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Num(String);

impl Num {
    fn zero() -> Num {
        Num("0".to_string())
    }

    fn is_zero(&self) -> bool {
        self.0 == "0"
    }

    fn next(&self) -> Num {
        let mut digits: Vec<u8> = self.0.bytes().rev().collect();
        for digit in digits.iter_mut() {
            if *digit == b'9' {
                *digit = b'0';
            } else {
                *digit += 1;
                return Num(String::from_utf8(digits.into_iter().rev().collect()).unwrap());
            }
        }
        digits.push(b'1');
        Num(String::from_utf8(digits.into_iter().rev().collect()).unwrap())
    }
}

impl Ord for Num {
    fn cmp(&self, other: &Num) -> Ordering {
        (self.0.len(), &self.0).cmp(&(other.0.len(), &other.0))
    }
}

impl PartialOrd for Num {
    fn partial_cmp(&self, other: &Num) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

type Version = [Num; 3];

struct VersionLine {
    rel: String,
    path: PathBuf,
    text: String,
    start: usize,
    end: usize,
    version: Version,
}

struct Entry {
    mod_id: String,
    is_test: bool,
    mod_line: VersionLine,
    config: VersionLine,
}

impl Entry {
    fn version(&self) -> &Version {
        &self.mod_line.version
    }
}

pub fn run(args: &[String]) -> u8 {
    let parsed = match cli::parse(&SPEC, args) {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };
    let modes = [
        parsed.flag("check"),
        parsed.value("set").is_some(),
        parsed.pair("bump").is_some(),
    ];
    if modes.iter().filter(|mode| **mode).count() > 1 {
        return cli::fail(
            &SPEC,
            "argument --check, --set, --bump: not allowed with each other",
        );
    }
    let root_text = parsed.value("root").unwrap_or(".");
    let root = Path::new(root_text);
    if !root.is_dir() {
        println!("cannot run: root not found: {root_text}");
        return 2;
    }
    match sync(root, root_text, &parsed) {
        Ok(code) => code,
        Err(message) => {
            println!("cannot run: {message}");
            2
        }
    }
}

fn sync(root: &Path, root_text: &str, parsed: &cli::Parsed) -> Result<u8, String> {
    let family = read_family(root)?;
    let entries = mod_entries(root)?;
    let writing = parsed.value("set").is_some() || parsed.pair("bump").is_some();
    if !writing {
        if let Some(code) = enforce::skip_without_mods(root) {
            return Ok(code);
        }
    }
    if !entries.iter().any(|entry| !entry.is_test) {
        return Err(format!("no {MODS} under {root_text}"));
    }
    if let Some(text) = parsed.value("set") {
        return set_family(text, &family, &entries, root);
    }
    if let Some((mod_id, part)) = parsed.pair("bump") {
        return bump(mod_id, part, &family, &entries);
    }
    let problems = check_versions(&entries, &family);
    for problem in &problems {
        println!("{problem}");
    }
    if !problems.is_empty() {
        println!("FAILED: {} problems", problems.len());
        return Ok(1);
    }
    println!(
        "OK: {} mod files, {VERSION} {}",
        entries.len(),
        show(&family)
    );
    Ok(0)
}

fn show(version: &Version) -> String {
    version
        .iter()
        .map(|part| part.0.as_str())
        .collect::<Vec<_>>()
        .join(".")
}

fn parse_version(text: &str, whence: &str) -> Result<Version, String> {
    if !VERSION_SHAPE.is_match(text) {
        return Err(format!("{whence}: not x.y.z: {}", pytext::repr(text)));
    }
    let mut parts = text.split('.').map(|part| Num(part.to_string()));
    Ok([
        parts.next().unwrap(),
        parts.next().unwrap(),
        parts.next().unwrap(),
    ])
}

// A missing file, an unreadable one and bad UTF-8 each get their own message.
fn read_source(path: &Path, rel: &str) -> Result<String, String> {
    pytext::read_utf8(path)
        .map_err(|error| error.describe(rel, &pytext::escaped(path.as_os_str().as_bytes())))
}

fn read_family(root: &Path) -> Result<Version, String> {
    let text = read_source(&root.join(VERSION), VERSION)?;
    let Some(captures) = FAMILY_FILE.captures(&text) else {
        return Err(format!("{VERSION}: not x.y.z: {}", pytext::repr(&text)));
    };
    parse_version(&captures[1], VERSION)
}

// Directory names in Python's order, which compares code points; only directories count.
fn folder_names(directory: &Path) -> Result<Vec<OsString>, String> {
    if !directory.is_dir() {
        return Ok(Vec::new());
    }
    let shown = pytext::escaped(directory.as_os_str().as_bytes());
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("{shown}: cannot read: {}", pytext::os_error(&error, &shown)))?;
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| {
            format!("{shown}: cannot read: {}", pytext::os_error(&error, &shown))
        })?;
        names.push(entry.file_name());
    }
    names.sort_by_key(|name| pytext::name_points(name.as_bytes()));
    Ok(names
        .into_iter()
        .filter(|name| directory.join(name).is_dir())
        .collect())
}

fn mask_comments(text: &str) -> String {
    STRING_OR_COMMENT
        .replace_all(text, |captures: &regex::Captures| {
            let found = &captures[0];
            if found.starts_with('"') {
                found.to_string()
            } else {
                found
                    .bytes()
                    .map(|byte| if byte == b'\n' { '\n' } else { ' ' })
                    .collect::<String>()
            }
        })
        .into_owned()
}

fn read_version_line(path: &Path, rel: &str) -> Result<VersionLine, String> {
    let text = read_source(path, rel)?;
    let masked = mask_comments(&text);
    let count = ANY_VERSION_LINE.find_iter(&masked).count();
    if count == 0 {
        return Err(format!("{rel} has no version line"));
    }
    if count > 1 {
        return Err(format!("{rel} has {count} version lines"));
    }
    let Some(captures) = VERSION_LINE.captures(&masked) else {
        return Err(format!("{rel}: version line is not version = \"x.y.z\";"));
    };
    let number = captures.get(2).unwrap();
    let version = parse_version(number.as_str(), rel)?;
    Ok(VersionLine {
        rel: rel.to_string(),
        path: path.to_path_buf(),
        text,
        start: number.start(),
        end: number.end(),
        version,
    })
}

fn read_entry(root: &Path, folder: &str, name: &OsString, is_test: bool) -> Result<Entry, String> {
    let rel_dir = Path::new(folder).join(name);
    let rel_of = |file: &str| pytext::escaped(rel_dir.join(file).as_os_str().as_bytes());
    let mod_line = read_version_line(&root.join(rel_dir.join("mod.cpp")), &rel_of("mod.cpp"))?;
    let config = read_version_line(
        &root.join(rel_dir.join("config.cpp")),
        &rel_of("config.cpp"),
    )?;
    let name = pytext::escaped(name.as_bytes());
    let mod_id = if is_test {
        name.strip_suffix(TEST_SUFFIX).unwrap_or(&name).to_string()
    } else {
        name
    };
    Ok(Entry {
        mod_id,
        is_test,
        mod_line,
        config,
    })
}

fn mod_entries(root: &Path) -> Result<Vec<Entry>, String> {
    let mut entries = Vec::new();
    for name in folder_names(&root.join(MODS))? {
        entries.push(read_entry(root, MODS, &name, false)?);
    }
    for name in folder_names(&root.join(TESTS))? {
        if pytext::escaped(name.as_bytes()).ends_with(TEST_SUFFIX) {
            entries.push(read_entry(root, TESTS, &name, true)?);
        }
    }
    for entry in entries.iter().filter(|entry| entry.is_test) {
        let has_mod = entries
            .iter()
            .any(|other| !other.is_test && other.mod_id == entry.mod_id);
        if !has_mod {
            return Err(format!(
                "{}: no {} in {MODS}",
                entry.mod_line.rel, entry.mod_id
            ));
        }
    }
    Ok(entries)
}

fn check_versions(entries: &[Entry], family: &Version) -> Vec<String> {
    let mut problems = Vec::new();
    for entry in entries {
        if entry.config.version != *entry.version() {
            problems.push(format!(
                "{}: version {}, differs from {} at {}",
                entry.config.rel,
                show(&entry.config.version),
                entry.mod_line.rel,
                show(entry.version())
            ));
        }
        let rel = &entry.mod_line.rel;
        if entry.is_test {
            let parent = entries
                .iter()
                .find(|other| !other.is_test && other.mod_id == entry.mod_id);
            let parent = &parent.unwrap().mod_line.version;
            if entry.version() != parent {
                problems.push(format!(
                    "{rel}: version {}, differs from {} at {}",
                    show(entry.version()),
                    entry.mod_id,
                    show(parent)
                ));
            }
        } else if family[0].is_zero() {
            if entry.version() != family {
                problems.push(format!(
                    "{rel}: version {}, expected {}",
                    show(entry.version()),
                    show(family)
                ));
            }
        } else if entry.version()[0] != family[0] {
            problems.push(format!(
                "{rel}: version {}, major differs from {VERSION} {}",
                show(entry.version()),
                show(family)
            ));
        } else if entry.version() < family {
            problems.push(format!(
                "{rel}: version {}, below {VERSION} {}",
                show(entry.version()),
                show(family)
            ));
        }
    }
    problems
}

fn replace_version(record: &VersionLine, version: &Version) -> String {
    format!(
        "{}{}{}",
        &record.text[..record.start],
        show(version),
        &record.text[record.end..]
    )
}

fn entry_changes<'a>(
    entries: impl IntoIterator<Item = &'a Entry>,
    version: &Version,
) -> Vec<(PathBuf, String)> {
    let mut changes = Vec::new();
    for entry in entries {
        changes.push((
            entry.mod_line.path.clone(),
            replace_version(&entry.mod_line, version),
        ));
        changes.push((
            entry.config.path.clone(),
            replace_version(&entry.config, version),
        ));
    }
    changes
}

// Every new text is staged in its own folder first; the replaces follow only once all staging worked.
fn write_all(changes: &[(PathBuf, String)]) -> Result<(), String> {
    let mut staged = Vec::new();
    let result = stage_all(changes, &mut staged).and_then(|()| replace_all(&staged));
    for (temp, _) in &staged {
        let _ = fs::remove_file(temp);
    }
    result
}

fn stage_all(
    changes: &[(PathBuf, String)],
    staged: &mut Vec<(PathBuf, PathBuf)>,
) -> Result<(), String> {
    for (path, text) in changes {
        let wrap = |error: std::io::Error, shown: &str| {
            format!("cannot write: {}", pytext::os_error(&error, shown))
        };
        let (temp, mut handle) = create_temp(path.parent().unwrap_or(Path::new(".")))
            .map_err(|(temp, error)| wrap(error, &pytext::escaped(temp.as_os_str().as_bytes())))?;
        staged.push((temp.clone(), path.clone()));
        let shown_temp = pytext::escaped(temp.as_os_str().as_bytes());
        handle
            .write_all(text.as_bytes())
            .map_err(|error| wrap(error, &shown_temp))?;
        let mode = fs::metadata(path)
            .map_err(|error| wrap(error, &pytext::escaped(path.as_os_str().as_bytes())))?
            .permissions()
            .mode()
            & 0o7777;
        fs::set_permissions(&temp, fs::Permissions::from_mode(mode))
            .map_err(|error| wrap(error, &shown_temp))?;
    }
    Ok(())
}

fn create_temp(dir: &Path) -> Result<(PathBuf, fs::File), (PathBuf, std::io::Error)> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let mut attempts = 0;
    loop {
        let temp = dir.join(format!(
            ".sync_version-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temp)
        {
            Ok(handle) => return Ok((temp, handle)),
            Err(error) if error.kind() == ErrorKind::AlreadyExists && attempts < 100 => {
                attempts += 1
            }
            Err(error) => return Err((temp, error)),
        }
    }
}

fn replace_all(staged: &[(PathBuf, PathBuf)]) -> Result<(), String> {
    for (temp, path) in staged {
        let shown = format!(
            "{} -> {}",
            pytext::repr(&pytext::escaped(temp.as_os_str().as_bytes())),
            pytext::repr(&pytext::escaped(path.as_os_str().as_bytes()))
        );
        fs::rename(temp, path)
            .map_err(|error| format!("cannot write: {}", pytext::os_error_text(&error, &shown)))?;
    }
    Ok(())
}

fn family_release_allowed(family: &Version, wanted: &Version) -> bool {
    if wanted[0].is_zero() {
        return family[0].is_zero();
    }
    wanted[1].is_zero() && wanted[2].is_zero() && wanted[0] > family[0]
}

fn set_family(text: &str, family: &Version, entries: &[Entry], root: &Path) -> Result<u8, String> {
    let wanted = match parse_version(text, "--set") {
        Ok(wanted) => wanted,
        Err(error) => {
            println!("refused: {error}");
            return Ok(1);
        }
    };
    if !family_release_allowed(family, &wanted) {
        println!(
            "refused: --set {} needs 0.y.z or a higher x.0.0 than {}",
            show(&wanted),
            show(family)
        );
        return Ok(1);
    }
    let mut changes = entry_changes(entries, &wanted);
    changes.push((root.join(VERSION), format!("{}\n", show(&wanted))));
    write_all(&changes)?;
    println!("set {} mod files to {}", entries.len(), show(&wanted));
    Ok(0)
}

fn bump(mod_id: &str, part: &str, family: &Version, entries: &[Entry]) -> Result<u8, String> {
    if !PARTS.contains(&part) {
        println!("refused: --bump takes minor or patch, not {part}");
        return Ok(1);
    }
    let Some(target) = entries
        .iter()
        .find(|entry| !entry.is_test && entry.mod_id == mod_id)
    else {
        return Err(format!("no {MODS}/{mod_id}"));
    };
    if family[0].is_zero() {
        println!(
            "refused: --bump needs a 1.0.0 release, {VERSION} is {}",
            show(family)
        );
        return Ok(1);
    }
    if let Some(first) = check_versions(entries, family).first() {
        println!("refused: --bump on a family that fails the check: {first}");
        return Ok(1);
    }
    let [major, minor, patch] = target.version();
    let wanted = if part == "minor" {
        [major.clone(), minor.next(), Num::zero()]
    } else {
        [major.clone(), minor.clone(), patch.next()]
    };
    let moved: Vec<&Entry> = entries
        .iter()
        .filter(|entry| entry.mod_id == mod_id)
        .collect();
    write_all(&entry_changes(moved, &wanted))?;
    println!("bumped {mod_id} to {}", show(&wanted));
    Ok(0)
}
