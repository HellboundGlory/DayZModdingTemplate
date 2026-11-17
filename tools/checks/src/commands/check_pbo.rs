use std::process::Command;
use std::sync::LazyLock;

use regex::Regex;

use crate::cli::{self, Opt, Spec};
use crate::enforce;
use crate::pytext;

const SPEC: Spec = Spec {
    command: "check-pbo",
    usage: "[-h] [--list-file LIST_FILE] [mod_root]",
    options: &[Opt {
        name: "list-file",
        values: 1,
    }],
    positional: Some(("mod_root", false)),
};

const NEVER_SHIP: [&str; 4] = ["docs", "tools", "tests", "test"];

static PACKED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[+-]\s+(\d+)  (.+?)  \[[^\]]*\]$").unwrap());
static SUMMARY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(\d+) of (\d+) files packed, \d+ bytes$").unwrap());

pub fn run(args: &[String]) -> u8 {
    let parsed = match cli::parse(&SPEC, args) {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };
    let mod_root = parsed.positionals.first().filter(|path| !path.is_empty());
    let list_file = parsed.value("list-file").filter(|path| !path.is_empty());
    match (mod_root, list_file) {
        (None, None) => cli::fail(&SPEC, "give a mod root or --list-file"),
        (Some(_), Some(_)) => cli::fail(&SPEC, "give a mod root or --list-file, not both"),
        (None, Some(path)) => {
            let bytes = match std::fs::read(path) {
                Ok(bytes) => bytes,
                Err(error) => {
                    eprintln!("cannot read {path}: {}", pytext::os_error(&error, path));
                    return 2;
                }
            };
            let text = String::from_utf8_lossy(&bytes);
            let (paths, warnings) = parse_packed(pytext::file_lines(&text));
            report(paths, &warnings, path)
        }
        (Some(mod_root), None) => {
            if let Err(note) = enforce::check_mf() {
                println!("skipped: {note}");
                return 0;
            }
            let Some(output) = mf_stdout(mod_root) else {
                return 2;
            };
            let (paths, warnings) = parse_packed(pytext::splitlines(&output));
            report(paths, &warnings, mod_root)
        }
    }
}

// None when the lines are not the output of `mf build --list`, or the summary does not add up.
fn parse_packed<'a>(
    lines: impl IntoIterator<Item = &'a str>,
) -> (Option<Vec<String>>, Vec<String>) {
    let mut paths = Vec::new();
    let mut warnings = Vec::new();
    let mut listed = 0usize;
    let mut totals: Option<(Option<usize>, Option<usize>)> = None;
    for raw in lines {
        let line = pytext::rstrip(raw);
        if line.is_empty() || line.starts_with("using saved settings:") {
            continue;
        }
        if line.starts_with("warning:") {
            warnings.push(line.to_string());
        } else if line.starts_with(['+', '-']) {
            let Some(captures) = PACKED.captures(line) else {
                return (None, warnings);
            };
            listed += 1;
            if line.starts_with('+') {
                paths.push(captures[2].to_string());
            }
        } else {
            let Some(captures) = SUMMARY.captures(line) else {
                return (None, warnings);
            };
            totals = Some((captures[1].parse().ok(), captures[2].parse().ok()));
        }
    }
    let complete = matches!(totals, Some((packed, files)) if files == Some(listed) && packed == Some(paths.len()));
    (complete.then_some(paths), warnings)
}

fn mf_stdout(mod_root: &str) -> Option<String> {
    use std::os::unix::process::ExitStatusExt;
    match Command::new("mf")
        .args(["build", mod_root, "--list"])
        .output()
    {
        Err(error) => {
            eprintln!("cannot run mf: {}", pytext::os_error(&error, "mf"));
            None
        }
        Ok(done) if !done.status.success() => {
            let code = done
                .status
                .code()
                .unwrap_or_else(|| -done.status.signal().unwrap_or(0));
            let stderr = String::from_utf8_lossy(&done.stderr);
            eprintln!("mf failed with exit {code}: {}", pytext::strip(&stderr));
            None
        }
        Ok(done) => Some(String::from_utf8_lossy(&done.stdout).into_owned()),
    }
}

fn is_config(path: &str) -> bool {
    path.replace('\\', "/").to_lowercase() == "config.cpp"
}

fn violations(paths: &[String]) -> Vec<String> {
    let mut problems = Vec::new();
    for path in paths {
        let normalized = path.replace('\\', "/");
        let segments: Vec<&str> = normalized.split('/').collect();
        let first = segments[0].to_lowercase();
        let banned = segments
            .iter()
            .find(|name| NEVER_SHIP.contains(&name.to_lowercase().as_str()));
        if let Some(banned) = banned {
            problems.push(format!("{path}: {banned}/ never ships"));
        } else if first == "server" {
            problems.push(format!("{path}: server/ never ships"));
        } else if first == "mods" {
            problems.push(format!("{path}: mods/ never ships"));
        } else if path.to_lowercase().ends_with(".md") {
            problems.push(format!("{path}: markdown never ships"));
        }
    }
    problems
}

fn report(paths: Option<Vec<String>>, warnings: &[String], source: &str) -> u8 {
    for warning in warnings {
        println!("{warning}");
    }
    let Some(paths) = paths else {
        eprintln!("cannot parse {source}: not `mf build --list` output");
        return 2;
    };
    if paths.is_empty() {
        eprintln!("cannot run {source}: no packed paths");
        return 2;
    }
    if !paths.iter().any(|path| is_config(path)) {
        eprintln!("cannot run {source}: no config.cpp packed");
        return 2;
    }
    let problems = violations(&paths);
    for problem in &problems {
        println!("{problem}");
    }
    if !problems.is_empty() {
        println!(
            "FAILED: {} that must never ship",
            enforce::count(problems.len(), "packed path", "packed paths")
        );
        return 1;
    }
    println!(
        "OK: {}, none inside docs, tools, tests or mods at any depth, no server/ at the root, and no markdown",
        enforce::count(paths.len(), "packed path", "packed paths")
    );
    0
}
