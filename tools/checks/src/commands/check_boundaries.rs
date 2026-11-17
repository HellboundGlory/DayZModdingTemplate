use crate::enforce::{self, read_text, strip_comments_keep_lines};
use crate::family;
use regex::Regex;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

static BACKTICK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"`([^`\n]+)`").unwrap());

struct Scope {
    kind: &'static str,
    mod_id: String,
}

struct Source {
    path: PathBuf,
    rel: String,
    scope: Scope,
}

pub fn run(args: &[String]) -> u8 {
    let parsed = match enforce::parse_options(
        args,
        "usage: checks check-boundaries [--root ROOT]",
        &["--root"],
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
    let (count, problems) = match check(&root, &family) {
        Ok(found) => found,
        Err(error) => {
            println!("cannot run: {error}");
            return 2;
        }
    };
    if let Some(code) = super::failed(&problems) {
        return code;
    }
    println!("OK: {count} files checked");
    0
}

fn check(root: &Path, family: &family::Family) -> Result<(usize, Vec<String>), String> {
    if mod_folders(root)?.is_empty() {
        return Err(format!("no mods under {}", root.display()));
    }
    let known = owners(root)?;
    let upper_known: BTreeMap<String, String> = known
        .iter()
        .map(|name| (upper_key(name, &family.prefix), name.clone()))
        .collect();
    let exceptions = contracts(root, &family.core)?;
    let files = source_files(root)?;
    let rule = family.rule();

    let mut problems = Vec::new();
    for source in &files {
        let text = strip_comments_keep_lines(
            &read_text(&source.path, &source.rel)?,
            source
                .path
                .file_name()
                .is_some_and(|name| name.to_string_lossy().to_lowercase() == "config.cpp"),
        );
        for (at, identifier) in family_identifiers(&text, &family.prefix) {
            if exceptions.contains(&identifier) {
                continue;
            }
            let Some(owner) = owner_of(&identifier, &known) else {
                continue;
            };
            if allowed(&owner, &source.scope, &family.core) {
                continue;
            }
            problems.push(format!(
                "{}:{}: {identifier} belongs to {owner}: {rule}",
                source.rel,
                enforce::line_of(&text, at)
            ));
        }
        for (at, identifier) in upper_identifiers(&text, &family.prefix) {
            if exceptions.contains(&identifier) {
                continue;
            }
            let Some(owner) = upper_owner(&identifier, &upper_known) else {
                continue;
            };
            if allowed(&owner, &source.scope, &family.core) {
                continue;
            }
            problems.push(format!(
                "{}:{}: {identifier} belongs to {owner}: {rule}",
                source.rel,
                enforce::line_of(&text, at)
            ));
        }
    }
    Ok((files.len(), problems))
}

fn mod_folders(root: &Path) -> Result<Vec<String>, String> {
    let directory = root.join("mods");
    if !directory.is_dir() {
        return Ok(Vec::new());
    }
    Ok(enforce::list_dir(&directory)?
        .into_iter()
        .filter(|name| directory.join(name).is_dir())
        .collect())
}

// Mod and test folder names, longest first, so the longest matching name owns an identifier.
fn owners(root: &Path) -> Result<Vec<String>, String> {
    let mut names = mod_folders(root)?;
    let directory = root.join("tests");
    if directory.is_dir() {
        names.extend(
            enforce::list_dir(&directory)?
                .into_iter()
                .filter(|name| name.ends_with("Tests") && directory.join(name).is_dir()),
        );
    }
    names.sort_by(|a, b| {
        b.chars()
            .count()
            .cmp(&a.chars().count())
            .then_with(|| a.cmp(b))
    });
    Ok(names)
}

fn upper_key(name: &str, prefix: &str) -> String {
    let rest = name.strip_prefix(prefix).unwrap_or(name);
    format!("{}_{}", prefix.to_uppercase(), rest.to_uppercase())
}

fn owner_of(identifier: &str, known: &[String]) -> Option<String> {
    known
        .iter()
        .find(|name| identifier.starts_with(name.as_str()))
        .cloned()
}

fn upper_owner(identifier: &str, upper_known: &BTreeMap<String, String>) -> Option<String> {
    let mut keys: Vec<&String> = upper_known.keys().collect();
    keys.sort_by(|a, b| {
        b.chars()
            .count()
            .cmp(&a.chars().count())
            .then_with(|| a.cmp(b))
    });
    keys.into_iter()
        .find(|key| identifier == key.as_str() || identifier.starts_with(&format!("{key}_")))
        .map(|key| upper_known[key].clone())
}

fn allowed(owner: &str, scope: &Scope, core: &str) -> bool {
    if owner == core {
        return true;
    }
    if scope.kind == "mod" {
        return owner == scope.mod_id;
    }
    owner == scope.mod_id || scope.mod_id.strip_suffix("Tests") == Some(owner)
}

fn scope_of(rel: &str) -> Option<Scope> {
    let parts: Vec<&str> = rel.split('/').collect();
    if parts.len() > 2 && parts[0] == "mods" {
        return Some(Scope {
            kind: "mod",
            mod_id: parts[1].to_string(),
        });
    }
    if parts.len() > 2 && parts[0] == "tests" && parts[1].ends_with("Tests") {
        return Some(Scope {
            kind: "test",
            mod_id: parts[1].to_string(),
        });
    }
    None
}

fn source_files(root: &Path) -> Result<Vec<Source>, String> {
    let mut found = Vec::new();
    for (path, rel) in enforce::mod_source_files(root)? {
        if let Some(scope) = scope_of(&rel) {
            found.push(Source { path, rel, scope });
        }
    }
    Ok(found)
}

fn contracts(root: &Path, core: &str) -> Result<HashSet<String>, String> {
    let rel = format!("mods/{core}/docs/CONTRACTS.md");
    let path = root.join(&rel);
    if !path.is_file() {
        return Ok(HashSet::new());
    }
    let text = read_text(&path, &rel)?;
    Ok(BACKTICK
        .captures_iter(&text)
        .map(|found| found[1].to_string())
        .collect())
}

fn is_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

// Each run of ASCII letters, digits and underscores that starts with the family prefix.
fn family_identifiers(text: &str, prefix: &str) -> Vec<(usize, String)> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        if !is_word(bytes[at]) {
            at += 1;
            continue;
        }
        let start = at;
        while at < bytes.len() && is_word(bytes[at]) {
            at += 1;
        }
        if text[start..at].starts_with(prefix) {
            found.push((start, text[start..at].to_string()));
        }
    }
    found
}

// <PREFIX>_ followed by upper-case letters, digits and underscores, not preceded by a letter or digit.
fn upper_identifiers(text: &str, prefix: &str) -> Vec<(usize, String)> {
    let marker = format!("{}_", prefix.to_uppercase());
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let preceded = at > 0 && bytes[at - 1].is_ascii_alphanumeric();
        if !preceded && bytes[at..].starts_with(marker.as_bytes()) {
            let mut end = at + marker.len();
            while end < bytes.len()
                && (bytes[end].is_ascii_uppercase()
                    || bytes[end].is_ascii_digit()
                    || bytes[end] == b'_')
            {
                end += 1;
            }
            if end > at + marker.len() {
                found.push((at, text[at..end].to_string()));
                at = end;
                continue;
            }
        }
        at += 1;
    }
    found
}
