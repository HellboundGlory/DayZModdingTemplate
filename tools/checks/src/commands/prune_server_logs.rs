use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::{Duration, SystemTime};

use regex::Regex;

use crate::cli::{self, Opt, Spec};
use crate::digits;
use crate::family;
use crate::pytext;
use crate::repo;

const SPEC: Spec = Spec {
    command: "prune-server-logs",
    usage: "[-h] [--keep KEEP] [--root ROOT] profiles",
    options: &[
        Opt {
            name: "keep",
            values: 1,
        },
        Opt {
            name: "root",
            values: 1,
        },
    ],
    positional: Some(("profiles", true)),
};

const DEFAULT_KEEP: i64 = 2;
const BOOT_FOLDERS: [&str; 2] = ["EventManagerLog", "WebApiLog"];
// Each server start writes one RPT, and its other files carry a stamp a moment either side of it.
const SAME_RUN_SECONDS: i64 = 60;
// A server running beside a self test is still writing its own logs.
const STILL_WRITTEN: Duration = Duration::from_secs(15 * 60);

// Python's \w: letters, numbers and underscore, which Rust's \w does not match exactly.
static RUN_FILE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(DayZ[\p{L}\p{N}_]+|script|error|crash|ErrorMessage_[\p{L}\p{N}_]+|FROZEN_[\p{L}\p{N}_]+|EventManagerLog|WebApiLog)_(\d{4}-\d{2}-\d{2}_\d{2}-\d{2}-\d{2})\.(RPT|ADM|log|mdmp)\n?$",
    )
    .unwrap()
});
fn check_folder(core: &str) -> Regex {
    Regex::new(&format!(
        r"^{}Tests[A-Z][\p{{L}}\p{{N}}_]*\n?$",
        regex::escape(core)
    ))
    .unwrap()
}

type Failure = (PathBuf, io::Error);

struct Run {
    path: PathBuf,
    stamp: i64,
    extension: String,
}

pub fn run(args: &[String]) -> u8 {
    let parsed = match cli::parse(&SPEC, args) {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };
    let profiles = PathBuf::from(&parsed.positionals[0]);
    let keep = match parsed.value("keep") {
        None => DEFAULT_KEEP,
        Some(text) => match python_int(text) {
            Some(keep) => keep,
            None => {
                let message = format!("argument --keep: invalid int value: {}", pytext::repr(text));
                return cli::fail(&SPEC, &message);
            }
        },
    };
    if keep < 1 {
        return cli::fail(&SPEC, "--keep must be 1 or more");
    }
    if !profiles.is_dir() {
        return 0;
    }
    let root = parsed.value("root").map_or_else(repo::root, PathBuf::from);
    let family = match family::load_or_report(&root) {
        Ok(family) => family,
        Err(code) => return code,
    };
    match prune(&profiles, keep, &check_folder(&family.core)) {
        Ok((removed, folders)) => {
            println!("server logs: removed {removed} old files and {folders} check folders");
            0
        }
        Err(message) => {
            eprintln!("prune-server-logs: {message}");
            1
        }
    }
}

fn prune(profiles: &Path, keep: i64, check: &Regex) -> Result<(usize, usize), String> {
    let mut files = Vec::new();
    if let Some(limit) = oldest_kept(&dated_files(profiles)?, keep)? {
        let mut folders = vec![profiles.to_path_buf()];
        folders.extend(BOOT_FOLDERS.iter().map(|name| profiles.join(name)));
        for folder in folders {
            if !folder.is_dir() || is_link(&folder) {
                continue;
            }
            for run in dated_files(&folder)? {
                if run.stamp < limit && is_settled(&run.path)? {
                    files.push(run.path);
                }
            }
        }
    }
    let mut check_folders = Vec::new();
    for name in names_in(profiles)? {
        let path = profiles.join(&name);
        if check.is_match(&name) && path.is_dir() && !is_link(&path) {
            check_folders.push(path);
        }
    }
    // Every path is settled before the first removal, so a failure above leaves all files in place.
    let removed: usize = files
        .iter()
        .map(|path| {
            report_removal(
                path,
                fs::remove_file(path).map_err(|error| (path.clone(), error)),
            )
        })
        .sum();
    let folders: usize = check_folders
        .iter()
        .map(|path| report_removal(path, remove_tree(path)))
        .sum();
    Ok((removed, folders))
}

fn names_in(folder: &Path) -> Result<Vec<String>, String> {
    let entries = fs::read_dir(folder)
        .map_err(|error| pytext::os_error(&error, &folder.to_string_lossy()))?;
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| pytext::os_error(&error, &folder.to_string_lossy()))?;
        if let Ok(name) = entry.file_name().into_string() {
            names.push(name);
        }
    }
    names.sort();
    Ok(names)
}

fn dated_files(folder: &Path) -> Result<Vec<Run>, String> {
    let mut runs = Vec::new();
    for name in names_in(folder)? {
        let path = folder.join(&name);
        if !path.is_file() {
            continue;
        }
        let Some(captures) = RUN_FILE.captures(&name) else {
            continue;
        };
        if let Some(stamp) = stamp_seconds(&captures[2])? {
            runs.push(Run {
                path,
                stamp,
                extension: captures[3].to_string(),
            });
        }
    }
    Ok(runs)
}

fn oldest_kept(runs: &[Run], keep: i64) -> Result<Option<i64>, String> {
    let mut starts: Vec<i64> = runs
        .iter()
        .filter(|run| run.extension == "RPT")
        .map(|run| run.stamp)
        .collect();
    if starts.is_empty() {
        return Ok(None);
    }
    starts.sort();
    let kept = (keep as usize).min(starts.len());
    let limit = starts[starts.len() - kept] - SAME_RUN_SECONDS;
    if limit < days_from_civil(1, 1, 1) * 86_400 {
        return Err("date value out of range".to_string());
    }
    Ok(Some(limit))
}

fn is_settled(path: &Path) -> Result<bool, String> {
    let modified = fs::metadata(path)
        .and_then(|meta| meta.modified())
        .map_err(|error| pytext::os_error(&error, &path.to_string_lossy()))?;
    Ok(SystemTime::now()
        .duration_since(modified)
        .is_ok_and(|age| age > STILL_WRITTEN))
}

fn is_link(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_symlink())
}

fn report_removal(shown: &Path, result: Result<(), Failure>) -> usize {
    match result {
        Ok(()) => 1,
        Err((path, error)) => {
            let shown = shown.to_string_lossy();
            println!(
                "server logs: could not remove {shown}: {}",
                pytext::os_error(&error, &path.to_string_lossy())
            );
            0
        }
    }
}

// Stops at the first failure. A file that cannot be unlinked is reported by its folder, and a folder by its own path, as Python's rmtree reports them.
fn remove_tree(path: &Path) -> Result<(), Failure> {
    let entries = fs::read_dir(path).map_err(|error| (path.to_path_buf(), error))?;
    for entry in entries {
        let entry = entry.map_err(|error| (path.to_path_buf(), error))?;
        let child = entry.path();
        if entry
            .file_type()
            .map_err(|error| (child.clone(), error))?
            .is_dir()
        {
            remove_tree(&child)?;
        } else {
            fs::remove_file(&child).map_err(|error| (path.to_path_buf(), error))?;
        }
    }
    fs::remove_dir(path).map_err(|error| (path.to_path_buf(), error))
}

// The stamp as seconds from year 1, or None where strptime would refuse it. Each digit is read by value,
// but strptime's patterns only take a Unicode digit where they say \d, so a field's leading digit must be ASCII.
fn stamp_seconds(text: &str) -> Result<Option<i64>, String> {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() != 19 {
        return Ok(None);
    }
    let unreadable = || format!("cannot read the stamp {text}");
    let mut numbers = [0i64; 6];
    for (slot, range) in [0..4, 5..7, 8..10, 11..13, 14..16, 17..19]
        .into_iter()
        .enumerate()
    {
        for c in &chars[range] {
            let digit = digits::value(*c).ok_or_else(unreadable)?;
            numbers[slot] = numbers[slot] * 10 + i64::from(digit);
        }
    }
    let ascii =
        |c: char, low: u8, high: u8| c.is_ascii_digit() && (low..=high).contains(&(c as u8 - b'0'));
    let month_fits =
        (chars[5] == '1' && ascii(chars[6], 0, 2)) || (chars[5] == '0' && ascii(chars[6], 1, 9));
    let day_fits = match chars[8] {
        '3' => ascii(chars[9], 0, 1),
        '1' | '2' => true,
        '0' => ascii(chars[9], 1, 9),
        _ => false,
    };
    let hour_fits = match chars[11] {
        '0' | '1' => true,
        '2' => ascii(chars[12], 0, 3),
        _ => false,
    };
    let fits =
        month_fits && day_fits && hour_fits && ascii(chars[14], 0, 5) && ascii(chars[17], 0, 5);
    let [year, month, day, hour, minute, second] = numbers;
    let valid = fits
        && year >= 1
        && (1..=12).contains(&month)
        && (1..=days_in_month(year, month)).contains(&day)
        && hour <= 23
        && minute <= 59
        && second <= 59;
    Ok(
        valid
            .then(|| ((days_from_civil(year, month, day) * 24 + hour) * 60 + minute) * 60 + second),
    )
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let shifted_month = (month + 9) % 12;
    let day_of_year = (153 * shifted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era
}

// int() for the --keep value: surrounding spaces, a sign, and single underscores between digits.
fn python_int(text: &str) -> Option<i64> {
    let trimmed = pytext::strip(text);
    let (negative, digits) = match trimmed.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, trimmed.strip_prefix('+').unwrap_or(trimmed)),
    };
    if digits.is_empty()
        || digits.starts_with('_')
        || digits.ends_with('_')
        || digits.contains("__")
    {
        return None;
    }
    let cleaned: String = digits.chars().filter(|c| *c != '_').collect();
    if !cleaned.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let magnitude: i64 = cleaned.parse().unwrap_or(i64::MAX);
    Some(if negative { -magnitude } else { magnitude })
}
