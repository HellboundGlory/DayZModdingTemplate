use crate::enforce::{self, line_of, mask_strings, read_text, strip_comments, walk_tree};
use crate::family::{self, Family};
use crate::pytext;
use crate::repo;
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

const ALLOWLIST: &str = "tools/modded-allowlist.txt";
static PATCH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\bmodded[\s\x1c-\x1f]+class[\s\x1c-\x1f]+([A-Za-z_]\w*)").unwrap()
});

struct Source {
    mod_id: String,
    rel: String,
    text: String,
    patches: Vec<(String, usize)>,
}

struct Row {
    number: usize,
    mod_id: String,
    name: String,
}

pub fn run(args: &[String]) -> u8 {
    let parsed = match enforce::parse_options(
        args,
        "usage: checks check-modded [--root ROOT]",
        &["--root"],
        0,
    ) {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };
    let root = match parsed.value("--root") {
        Some(root) => PathBuf::from(root),
        None => repo::root(),
    };
    if !root.is_dir() {
        println!("cannot run: root not found: {}", root.display());
        return 2;
    }
    let family = match family::load_or_report(&root) {
        Ok(family) => family,
        Err(code) => return code,
    };
    if let Some(code) = enforce::skip_without_mods(&root) {
        return code;
    }
    let mut problems = Vec::new();
    let (files, rows, broken) = match load(&root, &family, &mut problems) {
        Ok(loaded) => loaded,
        Err(error) => {
            println!("cannot run: {error}");
            return 2;
        }
    };
    patch_problems(&files, &rows, &broken, &mut problems);
    stale_problems(&files, &rows, &mut problems);

    for problem in &problems {
        println!("{problem}");
    }
    let patches: usize = files.iter().map(|source| source.patches.len()).sum();
    if !problems.is_empty() {
        println!("FAILED: {} problems", problems.len());
        return 1;
    }
    let mods = files
        .iter()
        .filter(|source| !source.patches.is_empty())
        .map(|source| source.mod_id.as_str())
        .collect::<HashSet<_>>()
        .len();
    println!(
        "OK: {}, {}, {}",
        enforce::count(patches, "patch", "patches"),
        enforce::count(rows.len(), "row", "rows"),
        enforce::count(mods, "mod", "mods")
    );
    0
}

type Allowlist = HashSet<(String, String)>;
type Loaded = (Vec<Source>, Vec<Row>, Allowlist);

fn load(root: &Path, family: &Family, problems: &mut Vec<String>) -> Result<Loaded, String> {
    let files = sources(root)?;
    let (rows, broken) = allowlist(root, family, problems)?;
    Ok((files, rows, broken))
}

fn sources(root: &Path) -> Result<Vec<Source>, String> {
    let mods = root.join("mods");
    if !mods.is_dir() {
        return Err(format!("no mods under {}", root.display()));
    }
    let folders = sorted_names(&mods, &mods.display().to_string(), |path| path.is_dir())?;
    if folders.is_empty() {
        return Err(format!("no mods under {}", root.display()));
    }
    let mut found = Vec::new();
    for mod_id in folders {
        let mod_root = mods.join(&mod_id);
        let scripts = sorted_names(&mod_root, &mod_id, |_| true)?
            .into_iter()
            .filter(|name| name.to_lowercase() == "scripts");
        for name in scripts {
            let folder = mod_root.join(&name);
            for path in walk_tree(&folder, true)? {
                if !path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_lowercase()
                    .ends_with(".c")
                {
                    continue;
                }
                let inside = enforce::relative_path(&folder, &path);
                let rel = format!("mods/{mod_id}/{name}/{inside}");
                let text = read_text(&path, &rel)?;
                found.push(source(&mod_id, rel, text));
            }
        }
    }
    Ok(found)
}

// The names in a folder that pass the test, sorted; an unreadable folder is a cannot-run for the name
// the check printed.
fn sorted_names(
    folder: &Path,
    shown: &str,
    keep: impl Fn(&Path) -> bool,
) -> Result<Vec<String>, String> {
    let entries = fs::read_dir(folder).map_err(|error| {
        format!(
            "{shown}: cannot read: {}",
            enforce::py_os_error(&error, folder)
        )
    })?;
    let mut names: Vec<String> = entries
        .flatten()
        .filter(|entry| keep(&entry.path()))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    Ok(names)
}

fn source(mod_id: &str, rel: String, text: String) -> Source {
    let code = strip_comments(&text);
    let mask = mask_strings(&code);
    let patches = PATCH
        .captures_iter(&mask)
        .map(|found| {
            (
                found[1].to_string(),
                line_of(&code, found.get(0).map_or(0, |whole| whole.start())),
            )
        })
        .collect();
    Source {
        mod_id: mod_id.to_string(),
        rel,
        text,
        patches,
    }
}

// The owners a row may name: vanilla, each framework's word, then the family's word.
fn owners(family: &Family) -> Vec<String> {
    let mut owners = vec!["vanilla".to_string()];
    owners.extend(
        family
            .frameworks
            .iter()
            .map(|framework| framework.name.to_lowercase()),
    );
    owners.push(family.word());
    owners
}

fn allowlist(
    root: &Path,
    family: &Family,
    problems: &mut Vec<String>,
) -> Result<(Vec<Row>, Allowlist), String> {
    let path = root.join(ALLOWLIST);
    if !path.is_file() {
        return Err(format!("no allow list at {ALLOWLIST}"));
    }
    let text = read_text(&path, ALLOWLIST)?;
    let allowed = owners(family);
    let word = family.word();
    let prefix = family.prefix.as_str();
    let mut rows: Vec<Row> = Vec::new();
    let mut keys: HashSet<(String, String)> = HashSet::new();
    let mut broken = HashSet::new();
    for (index, line) in pytext::splitlines(&text).iter().enumerate() {
        let number = index + 1;
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let cells: Vec<&str> = line.split('\t').map(str::trim).collect();
        if cells.len() != 4 {
            problems.push(format!(
                "{ALLOWLIST}:{number}: a row needs 4 tab separated columns, not {}",
                cells.len()
            ));
            if cells.len() >= 2 {
                broken.insert((cells[0].to_string(), cells[1].to_string()));
            }
            continue;
        }
        let (mod_id, name, owner, reason) = (cells[0], cells[1], cells[2], cells[3]);
        if !allowed.iter().any(|known| known == owner) {
            problems.push(format!(
                "{ALLOWLIST}:{number}: {owner} is not an owner: {}",
                family::join_names(&allowed, "or")
            ));
        } else if owner == word && !name.starts_with(prefix) {
            problems.push(format!(
                "{ALLOWLIST}:{number}: {word} covers only a {prefix} class, not {name}"
            ));
        } else if name.starts_with(prefix) && owner != word {
            problems.push(format!(
                "{ALLOWLIST}:{number}: {name} can't have the owner {owner}"
            ));
        }
        if reason.is_empty() {
            problems.push(format!(
                "{ALLOWLIST}:{number}: the reason for {name} is empty"
            ));
        }
        let key = (mod_id.to_string(), name.to_string());
        if keys.contains(&key) {
            problems.push(format!(
                "{ALLOWLIST}:{number}: {name} in {mod_id} already has a row"
            ));
        } else {
            keys.insert(key);
            rows.push(Row {
                number,
                mod_id: mod_id.to_string(),
                name: name.to_string(),
            });
        }
    }
    Ok((rows, broken))
}

fn patch_problems(
    files: &[Source],
    rows: &[Row],
    broken: &HashSet<(String, String)>,
    problems: &mut Vec<String>,
) {
    let listed: HashSet<(&str, &str)> = rows
        .iter()
        .map(|row| (row.mod_id.as_str(), row.name.as_str()))
        .collect();
    for source in files {
        let Some(first) = source.patches.first() else {
            continue;
        };
        let base = Path::new(&source.rel)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        if source.patches.len() > 1 {
            problems.push(format!(
                "{}:{}: {base} holds {} modded class lines",
                source.rel,
                first.1,
                source.patches.len()
            ));
        }
        let stem = Path::new(&base)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        for (name, line) in &source.patches {
            if &stem != name {
                problems.push(format!(
                    "{}:{line}: {stem}.c is not named after {name}",
                    source.rel
                ));
            }
            let key = (source.mod_id.as_str(), name.as_str());
            if !listed.contains(&key) && !broken.contains(&(source.mod_id.clone(), name.clone())) {
                problems.push(format!(
                    "{}:{line}: {name} has no row in {ALLOWLIST}",
                    source.rel
                ));
            }
        }
        if let Some((line, message)) = header_problem(&source.text, first.1) {
            problems.push(format!("{}:{line}: {message}", source.rel));
        }
    }
}

// The four // lines above the first patch: Why on the first, Layer on the third.
fn header_problem(text: &str, before: usize) -> Option<(usize, &'static str)> {
    let lines: Vec<(usize, &str)> = pytext::splitlines(text)
        .into_iter()
        .take(before.saturating_sub(1))
        .enumerate()
        .map(|(index, line)| (index + 1, line))
        .filter(|(_, line)| !line.trim().is_empty())
        .collect();
    if lines.len() < 4 {
        return Some((1, "the header is shorter than four // lines"));
    }
    let lines = &lines[..4];
    for (number, line) in lines {
        if !line.trim_start().starts_with("//") {
            return Some((*number, "a header line is not a // comment"));
        }
    }
    if !lines[0].1.trim_start().starts_with("// Why:") {
        return Some((
            lines[0].0,
            "the first header line does not start with // Why:",
        ));
    }
    if !lines[2].1.trim_start().starts_with("// Layer:") {
        return Some((
            lines[2].0,
            "the third header line does not start with // Layer:",
        ));
    }
    None
}

fn stale_problems(files: &[Source], rows: &[Row], problems: &mut Vec<String>) {
    let patched: HashSet<(&str, &str)> = files
        .iter()
        .flat_map(|source| {
            source
                .patches
                .iter()
                .map(move |(name, _)| (source.mod_id.as_str(), name.as_str()))
        })
        .collect();
    for row in rows {
        if !patched.contains(&(row.mod_id.as_str(), row.name.as_str())) {
            problems.push(format!(
                "{ALLOWLIST}:{}: {} in {} matches no patch",
                row.number, row.name, row.mod_id
            ));
        }
    }
}
