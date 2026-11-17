use crate::enforce::{
    self, line_of, py_os_error, read_text, strip_comments_keep_lines, walk_linked,
};
use crate::family::{self, Family};
use regex::Regex;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

const RULE: &str = "AGENTS rule 1: a reference is never a source";
const GAME_FAMILIES: [&str; 2] = ["Land_", "StaticObj_"];
static CLASS_DECLARATION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^[ \t]*(?:(modded)[\s\x1c-\x1f]+)?class[\s\x1c-\x1f]+([A-Za-z_]\w*)").unwrap()
});

pub fn run(args: &[String]) -> u8 {
    let parsed = match enforce::parse_options(
        args,
        "usage: checks check-reference [--root ROOT] [--reference REFERENCE] [--vanilla VANILLA]",
        &["--root", "--reference", "--vanilla"],
        0,
    ) {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };
    let root = PathBuf::from(parsed.value("--root").unwrap_or("."));
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
    match mod_folders(&root) {
        Ok(mods) if !mods.is_empty() => {}
        Ok(_) => {
            println!("cannot run: no mods under {}", root.display());
            return 2;
        }
        Err(error) => {
            println!("cannot run: {error}");
            return 2;
        }
    }
    if parsed.value("--reference").is_none_or(str::is_empty)
        && family.reference.is_none()
        && parsed.value("--vanilla").is_none()
    {
        println!(
            "skipped: no reference folder: pass --reference or set reference in tools/family.conf"
        );
        return 0;
    }
    let (names, files) = match load(
        &root,
        &family,
        parsed.value("--reference"),
        parsed.value("--vanilla"),
    ) {
        Ok(found) => found,
        Err(error) => {
            println!("cannot run: {error}");
            return 2;
        }
    };
    let matcher = Regex::new(&format!(
        r"\b(?:{})\b",
        names_longest_first(&names)
            .iter()
            .map(|name| regex::escape(name))
            .collect::<Vec<_>>()
            .join("|")
    ))
    .unwrap();

    let mut problems = Vec::new();
    for (path, rel) in &files {
        let text = match read_text(path, rel) {
            Ok(text) => strip_comments_keep_lines(
                &text,
                path.file_name()
                    .is_some_and(|name| name.to_string_lossy().to_lowercase() == "config.cpp"),
            ),
            Err(error) => {
                println!("cannot run: {error}");
                return 2;
            }
        };
        for found in matcher.find_iter(&text) {
            problems.push(format!(
                "{rel}:{}: uses reference name {}: {RULE}",
                line_of(&text, found.start()),
                found.as_str()
            ));
        }
    }

    if let Some(code) = super::failed(&problems) {
        return code;
    }
    println!(
        "OK: {} reference names, {} files checked",
        names.len(),
        files.len()
    );
    0
}

// Longest names first, so the matcher tries a longer name before one that is its prefix.
fn names_longest_first(names: &BTreeSet<String>) -> Vec<String> {
    let mut sorted: Vec<String> = names.iter().cloned().collect();
    sorted.sort_by(|a, b| {
        b.chars()
            .count()
            .cmp(&a.chars().count())
            .then_with(|| a.cmp(b))
    });
    sorted
}

type Loaded = (BTreeSet<String>, Vec<(PathBuf, String)>);

fn load(
    root: &Path,
    family: &Family,
    reference: Option<&str>,
    vanilla: Option<&str>,
) -> Result<Loaded, String> {
    let reference_dir = folder(
        reference,
        family.reference.as_deref(),
        "--reference",
        "reference",
    )?;
    let vanilla_dir = folder(vanilla, family.vanilla.as_deref(), "--vanilla", "vanilla")?;
    let mut names: BTreeSet<String> = class_names(&reference_dir, "--reference")?
        .into_iter()
        .filter(|name| !GAME_FAMILIES.iter().any(|prefix| name.starts_with(prefix)))
        .collect();
    for name in class_names(&vanilla_dir, "--vanilla")? {
        names.remove(&name);
    }
    if names.is_empty() {
        return Err("no reference names collected".to_string());
    }
    Ok((names, repo_files(root)?))
}

// The flag wins over the family's folder from tools/family.conf.
fn folder(
    flag: Option<&str>,
    configured: Option<&Path>,
    option: &str,
    key: &str,
) -> Result<PathBuf, String> {
    let path = match (flag.filter(|value| !value.is_empty()), configured) {
        (Some(value), _) => PathBuf::from(value),
        (None, Some(path)) => path.to_path_buf(),
        (None, None) => {
            return Err(format!(
                "{option} is missing: pass {option} or set {key} in tools/family.conf"
            ))
        }
    };
    if !path.is_dir() {
        return Err(format!(
            "{option} folder not found: {} (pass {option} or set {key} in tools/family.conf)",
            path.display()
        ));
    }
    Ok(path)
}

fn class_names(directory: &Path, option: &str) -> Result<BTreeSet<String>, String> {
    let files: Vec<PathBuf> = walk_linked(directory, None)?
        .into_iter()
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().to_lowercase().ends_with(".c"))
        })
        .collect();
    if files.is_empty() {
        return Err(format!(
            "no .c files under {option}: {}",
            directory.display()
        ));
    }
    let mut names = BTreeSet::new();
    for path in files {
        let bytes = fs::read(&path).map_err(|error| {
            format!(
                "{}: cannot read: {}",
                path.display(),
                py_os_error(&error, &path)
            )
        })?;
        let text = strip_comments_keep_lines(
            &String::from_utf8_lossy(&bytes)
                .replace("\r\n", "\n")
                .replace('\r', "\n"),
            false,
        );
        for found in CLASS_DECLARATION.captures_iter(&text) {
            if found.get(1).is_none() {
                names.insert(found[2].to_string());
            }
        }
    }
    Ok(names)
}

fn mod_folders(root: &Path) -> Result<Vec<String>, String> {
    let directory = root.join("mods");
    if !directory.is_dir() {
        return Ok(Vec::new());
    }
    let entries = fs::read_dir(&directory).map_err(|error| {
        format!(
            "{}: cannot read: {}",
            directory.display(),
            py_os_error(&error, &directory)
        )
    })?;
    Ok(entries
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect())
}

fn scope_of(rel: &str) -> bool {
    let parts: Vec<&str> = rel.split('/').collect();
    (parts.len() > 2 && parts[0] == "mods")
        || (parts.len() > 2 && parts[0] == "tests" && parts[1].ends_with("Tests"))
}

fn repo_files(root: &Path) -> Result<Vec<(PathBuf, String)>, String> {
    Ok(enforce::mod_source_files(root)?
        .into_iter()
        .filter(|(_, rel)| scope_of(rel))
        .collect())
}
