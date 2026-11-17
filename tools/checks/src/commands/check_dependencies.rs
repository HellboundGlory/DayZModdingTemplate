use crate::enforce::{self, line_of, read_text, strip_comments_keep_lines, walk_linked};
use crate::family::{self, join_names, Family};
use regex::Regex;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

static PATCHES: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bclass[\s\x1c-\x1f]+CfgPatches\b").unwrap());
static PATCH_CLASS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bclass[\s\x1c-\x1f]+([A-Za-z_]\w*)").unwrap());
static ADDONS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\brequiredAddons[\s\x1c-\x1f]*\[[\s\x1c-\x1f]*\][\s\x1c-\x1f]*(?:\+)?[\s\x1c-\x1f]*=[\s\x1c-\x1f]*\{").unwrap()
});
static STRING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#""([^"]*)""#).unwrap());
static QUOTED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#""[^"]*""#).unwrap());

struct Config {
    path: PathBuf,
    rel: String,
    kind: &'static str,
    mod_id: String,
    is_root: bool,
}

struct Parsed {
    classes: Vec<(String, usize)>,
    addons: Vec<(String, usize)>,
    addon_line: Option<usize>,
}

pub fn run(args: &[String]) -> u8 {
    let parsed = match enforce::parse_options(
        args,
        "usage: checks check-dependencies [--root ROOT]",
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
    let rule = family.rule();
    let (files, stray) = match config_files(&root) {
        Ok(found) => found,
        Err(error) => {
            println!("cannot run: {error}");
            return 2;
        }
    };
    let mut parsed = Vec::new();
    for config in &files {
        match parse_config(&config.path, &config.rel) {
            Ok(result) => parsed.push(result),
            Err(error) => {
                println!("cannot run: {error}");
                return 2;
            }
        }
    }
    let mod_ids: BTreeSet<String> = files
        .iter()
        .filter(|config| config.kind == "mod")
        .map(|config| config.mod_id.to_lowercase())
        .collect();
    let known: BTreeSet<String> = files.iter().map(|config| config.mod_id.clone()).collect();
    let mut problems: Vec<String> = stray
        .iter()
        .map(|rel| format!("{rel}: test folder does not end in Tests: {rule}"))
        .collect();
    for (config, result) in files.iter().zip(&parsed) {
        problems.extend(ownership_problems(config, &result.classes, &known, &rule));
        problems.extend(addon_problems(config, result, &mod_ids, &family, &rule));
    }

    if let Some(code) = super::failed(&problems) {
        return code;
    }
    println!("OK: {} configs checked", files.len());
    0
}

fn config_files(root: &Path) -> Result<(Vec<Config>, Vec<String>), String> {
    let mut found = false;
    let mut mods = 0;
    let mut result = Vec::new();
    let mut stray = Vec::new();
    for (parent, is_test) in [("mods", false), ("tests", true)] {
        let directory = root.join(parent);
        if !directory.is_dir() {
            continue;
        }
        found = true;
        for name in enforce::list_dir(&directory)? {
            let folder = directory.join(&name);
            if !folder.is_dir() {
                continue;
            }
            let rel = format!("{parent}/{name}/config.cpp");
            let path = folder.join("config.cpp");
            if is_test && !name.ends_with("Tests") {
                if path.is_file() {
                    stray.push(rel);
                }
                continue;
            }
            if !is_test {
                mods += 1;
            }
            if !path.is_file() {
                return Err(format!("{rel}: missing"));
            }
            let kind = if is_test { "test" } else { "mod" };
            result.push(Config {
                path: path.clone(),
                rel,
                kind,
                mod_id: name.clone(),
                is_root: true,
            });
            for nested in walk_linked(&folder, Some(0))? {
                if nested == path
                    || nested
                        .file_name()
                        .is_none_or(|file| file.to_string_lossy().to_lowercase() != "config.cpp")
                {
                    continue;
                }
                let inside = enforce::relative_path(root, &nested);
                result.push(Config {
                    path: nested,
                    rel: inside,
                    kind,
                    mod_id: name.clone(),
                    is_root: false,
                });
            }
        }
    }
    if !found {
        return Err(format!("no mods/ or tests/ under {}", root.display()));
    }
    if mods == 0 {
        return Err(format!("no mods under {}", root.display()));
    }
    Ok((result, stray))
}

// The config's text without the braces of a string: a string has no escapes here.
fn mask_strings(text: &str) -> String {
    QUOTED
        .replace_all(text, |found: &regex::Captures| " ".repeat(found[0].len()))
        .into_owned()
}

// The index of the brace that closes the one at start, skipping strings, or None.
fn matching_brace(text: &str, start: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0i64;
    let mut index = start;
    while index < bytes.len() {
        match bytes[index] {
            b'"' => {
                index += 1;
                while index < bytes.len() && bytes[index] != b'"' {
                    index += 1;
                }
                index += 1;
                continue;
            }
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
        index += 1;
    }
    None
}

// The block that opens after a position, with no semicolon before its brace: its start and end.
fn block_after(text: &str, position: usize) -> Option<(usize, usize)> {
    let start = text[position..].find('{').map(|at| position + at)?;
    if text[position..start].contains(';') {
        return None;
    }
    matching_brace(text, start).map(|end| (start, end))
}

// The brace depth at a position, counting from start, with strings skipped.
fn depth_at(text: &str, start: usize, position: usize) -> i64 {
    let bytes = text.as_bytes();
    let mut depth = 0i64;
    let mut index = start;
    while index < position {
        match bytes[index] {
            b'"' => {
                index += 1;
                while index < bytes.len() && bytes[index] != b'"' {
                    index += 1;
                }
            }
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ => {}
        }
        index += 1;
    }
    depth
}

// The matches of a pattern that start in [start, end), as (start, end, group 1 span).
type Span = (usize, usize);
type Match = (usize, usize, Option<Span>);

fn matches_within(pattern: &Regex, masked: &str, start: usize, end: usize) -> Vec<Match> {
    let mut found = Vec::new();
    let visible = &masked[..end];
    let mut at = start;
    while at <= visible.len() {
        let Some(captures) = pattern.captures_at(visible, at) else {
            break;
        };
        let whole = captures.get(0).expect("the whole match");
        found.push((
            whole.start(),
            whole.end(),
            captures.get(1).map(|group| (group.start(), group.end())),
        ));
        at = whole.end().max(at + 1);
    }
    found
}

fn check_quoted(rel: &str, body: &str) -> Result<(), String> {
    if STRING
        .replace_all(body, "")
        .trim_matches(|c| " \t\r\n,".contains(c))
        .is_empty()
    {
        return Ok(());
    }
    Err(format!(
        "{rel}: requiredAddons has a token that is not a quoted string"
    ))
}

fn parse_config(path: &Path, rel: &str) -> Result<Parsed, String> {
    let text = strip_comments_keep_lines(&read_text(path, rel)?, true);
    let masked = mask_strings(&text);
    let patches = matches_within(&PATCHES, &masked, 0, masked.len());
    if patches.is_empty() {
        return Err(format!("{rel}: no CfgPatches"));
    }
    let mut classes = Vec::new();
    let mut addons = Vec::new();
    let mut addon_line = None;
    for (_, patch_end, _) in patches {
        let Some(span) = block_after(&text, patch_end) else {
            return Err(format!("{rel}: CfgPatches has no block"));
        };
        for (block_start, block_end, name_span) in
            matches_within(&PATCH_CLASS, &masked, span.0, span.1)
        {
            if depth_at(&text, span.0, block_start) != 1 {
                continue;
            }
            let (name_start, name_end) = name_span.unwrap_or((block_start, block_end));
            let name = text[name_start..name_end].to_string();
            let Some(class_span) = block_after(&text, block_end) else {
                return Err(format!("{rel}: patch class {name} has no block"));
            };
            let mut found = false;
            for (assign_start, assign_end, _) in
                matches_within(&ADDONS, &masked, class_span.0, class_span.1)
            {
                if depth_at(&text, class_span.0, assign_start) != 1 {
                    continue;
                }
                let Some(end) = matching_brace(&text, assign_end - 1) else {
                    return Err(format!("{rel}: requiredAddons has no list"));
                };
                found = true;
                if addon_line.is_none() {
                    addon_line = Some(line_of(&text, assign_start));
                }
                let body = &text[assign_end..end];
                check_quoted(rel, body)?;
                for item in STRING.captures_iter(body) {
                    let group = item.get(1).expect("the quoted text");
                    addons.push((
                        group.as_str().to_string(),
                        line_of(&text, assign_end + group.start()),
                    ));
                }
            }
            if !found {
                return Err(format!("{rel}: patch class {name} has no requiredAddons"));
            }
            classes.push((name, line_of(&text, name_start)));
        }
    }
    if classes.is_empty() {
        return Err(format!("{rel}: CfgPatches holds no class"));
    }
    Ok(Parsed {
        classes,
        addons,
        addon_line,
    })
}

fn problem(rule: &str, rel: &str, line: usize, message: &str) -> String {
    format!("{rel}:{line}: {message}: {rule}")
}

// The mod or test whose name a class name starts with, longest name first.
fn owner_of(identifier: &str, known: &BTreeSet<String>) -> Option<String> {
    let lowered = identifier.to_lowercase();
    let mut names: Vec<&String> = known.iter().collect();
    names.sort_by(|a, b| {
        b.chars()
            .count()
            .cmp(&a.chars().count())
            .then_with(|| a.cmp(b))
    });
    names
        .into_iter()
        .find(|name| lowered.starts_with(&name.to_lowercase()))
        .cloned()
}

fn ownership_problems(
    config: &Config,
    classes: &[(String, usize)],
    known: &BTreeSet<String>,
    rule: &str,
) -> Vec<String> {
    let mut allowed = BTreeSet::new();
    allowed.insert(config.mod_id.to_lowercase());
    if config.kind == "test" {
        allowed.insert(config.mod_id[..config.mod_id.len() - "Tests".len()].to_lowercase());
    }
    let mut problems = Vec::new();
    for (name, line) in classes {
        if let Some(owner) = owner_of(name, known) {
            if !allowed.contains(&owner.to_lowercase()) {
                problems.push(problem(
                    rule,
                    &config.rel,
                    *line,
                    &format!("patch class {name} belongs to {owner}"),
                ));
            }
        }
    }
    if config.is_root
        && !classes
            .iter()
            .any(|(name, _)| name.to_lowercase() == config.mod_id.to_lowercase())
    {
        problems.push(problem(
            rule,
            &config.rel,
            classes[0].1,
            &format!("CfgPatches has no class {}", config.mod_id),
        ));
    }
    problems
}

// A framework's addons, one of which the core must list.
fn missing_core_addons(core: &str, addons: &[String]) -> String {
    match addons {
        [only] => format!("{core} does not list {only}"),
        _ => format!("{core} lists neither {}", join_names(addons, "nor")),
    }
}

fn addon_problems(
    config: &Config,
    parsed: &Parsed,
    mod_ids: &BTreeSet<String>,
    family: &Family,
    rule: &str,
) -> Vec<String> {
    let listed: BTreeSet<String> = parsed
        .addons
        .iter()
        .map(|(name, _)| name.to_lowercase())
        .collect();
    let core = family.core.to_lowercase();
    let base: BTreeSet<String> = family
        .frameworks
        .iter()
        .flat_map(|framework| framework.addons.iter().map(|addon| addon.to_lowercase()))
        .collect();
    let target = (config.kind == "test")
        .then(|| config.mod_id[..config.mod_id.len() - "Tests".len()].to_string());
    let parent = target.as_ref().map(|name| name.to_lowercase());
    let allowed = |name: &str| {
        let lowered = name.to_lowercase();
        lowered.starts_with("dz_")
            || base.contains(&lowered)
            || lowered == core
            || parent.as_deref() == Some(lowered.as_str())
    };
    let mut problems = Vec::new();
    for (name, at) in &parsed.addons {
        if allowed(name) {
            continue;
        }
        if mod_ids.contains(&name.to_lowercase()) {
            problems.push(problem(
                rule,
                &config.rel,
                *at,
                &format!("{} lists sibling {name}", config.mod_id),
            ));
        } else {
            problems.push(problem(
                rule,
                &config.rel,
                *at,
                &format!("{} lists unknown addon {name}", config.mod_id),
            ));
        }
    }
    if !config.is_root {
        return problems;
    }
    let addon_line = parsed.addon_line.unwrap_or(1);
    if config.kind == "mod" && config.mod_id.to_lowercase() == core {
        for framework in &family.frameworks {
            if !framework
                .addons
                .iter()
                .any(|addon| listed.contains(&addon.to_lowercase()))
            {
                let message = missing_core_addons(&family.core, &framework.addons);
                problems.push(problem(rule, &config.rel, addon_line, &message));
            }
        }
    } else if config.kind == "mod" {
        if !listed.contains(&core) {
            problems.push(problem(
                rule,
                &config.rel,
                addon_line,
                &format!("{} does not list {}", config.mod_id, family.core),
            ));
        }
    } else {
        let target = target.unwrap_or_default();
        if !listed.contains(&target.to_lowercase()) {
            problems.push(problem(
                rule,
                &config.rel,
                addon_line,
                &format!("{} does not list {target}", config.mod_id),
            ));
        }
        if !mod_ids.contains(&target.to_lowercase()) {
            problems.push(problem(
                rule,
                &config.rel,
                addon_line,
                &format!("{} targets {target}, which is not in mods/", config.mod_id),
            ));
        }
    }
    problems
}
