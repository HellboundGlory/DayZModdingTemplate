use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

pub const FILE: &str = "tools/family.conf";

#[derive(Debug, Clone, PartialEq)]
pub struct Framework {
    pub name: String,
    pub addons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Family {
    pub prefix: String,
    pub core: String,
    pub frameworks: Vec<Framework>,
    pub reference: Option<PathBuf>,
    pub vanilla: Option<PathBuf>,
}

impl Family {
    pub fn rule(&self) -> String {
        let mut names: Vec<&str> = self
            .frameworks
            .iter()
            .map(|framework| framework.name.as_str())
            .collect();
        names.push(&self.core);
        format!("a mod requires only {}", join_names(&names, "and"))
    }

    // The family's owner or group word: the prefix in lower case.
    pub fn word(&self) -> String {
        self.prefix.to_lowercase()
    }
}

// "a, b and c" with `last` as the final joining word.
pub fn join_names<S: AsRef<str>>(names: &[S], last: &str) -> String {
    match names {
        [] => String::new(),
        [only] => only.as_ref().to_string(),
        [rest @ .., final_name] => {
            let rest: Vec<&str> = rest.iter().map(AsRef::as_ref).collect();
            format!("{} {last} {}", rest.join(", "), final_name.as_ref())
        }
    }
}

// For a command: prints why the file cannot be used and gives the exit code.
pub fn load_or_report(root: &Path) -> Result<Family, u8> {
    load(root).map_err(|error| {
        println!("cannot run: {error}");
        2
    })
}

pub fn load(root: &Path) -> Result<Family, String> {
    let text = fs::read_to_string(root.join(FILE)).map_err(|error| match error.kind() {
        ErrorKind::NotFound => format!("{FILE} not found"),
        _ => format!("cannot read {FILE}: {error}"),
    })?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let mut prefix = None;
    let mut core = None;
    let mut frameworks = Vec::new();
    let mut reference = None;
    let mut vanilla = None;
    let mut core_line = 0;
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fail = |message: String| format!("{FILE}:{}: {message}", index + 1);
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| fail("not key = value".into()))?;
        let (key, value) = (key.trim(), value.trim());
        let twice = || fail(format!("{key} given twice"));
        match key {
            "prefix" => {
                set_once(&mut prefix, value.to_string()).ok_or_else(twice)?;
                if !is_name(value) {
                    return Err(fail(
                        "prefix must be letters and digits, starting with a capital".into(),
                    ));
                }
            }
            "core" => {
                set_once(&mut core, value.to_string()).ok_or_else(twice)?;
                core_line = index + 1;
            }
            "framework" => {
                let mut words = value.split_whitespace().map(str::to_string);
                let name = words.next();
                let addons: Vec<String> = words.collect();
                let Some(name) = name.filter(|_| !addons.is_empty()) else {
                    return Err(fail("framework needs a name and at least one addon".into()));
                };
                if frameworks
                    .iter()
                    .any(|known: &Framework| known.name == name)
                {
                    return Err(fail(format!("framework {name} given twice")));
                }
                frameworks.push(Framework { name, addons });
            }
            "reference" | "vanilla" => {
                if value.is_empty() {
                    return Err(fail(format!("{key} is empty")));
                }
                let path = expand(root, value)
                    .ok_or_else(|| fail(format!("{key} starts with ~ but HOME is not set")))?;
                let slot = if key == "reference" {
                    &mut reference
                } else {
                    &mut vanilla
                };
                set_once(slot, path).ok_or_else(twice)?;
            }
            _ => return Err(fail(format!("unknown key {key}"))),
        }
    }
    let prefix = prefix.ok_or(format!("{FILE}: no prefix"))?;
    let core = core.ok_or(format!("{FILE}: no core"))?;
    if !core
        .strip_prefix(prefix.as_str())
        .is_some_and(|rest| !rest.is_empty() && is_name_tail(rest))
    {
        return Err(format!(
            "{FILE}:{core_line}: core must be the prefix {prefix} followed by letters and digits"
        ));
    }
    Ok(Family {
        prefix,
        core,
        frameworks,
        reference,
        vanilla,
    })
}

fn set_once<T>(slot: &mut Option<T>, value: T) -> Option<()> {
    match slot {
        Some(_) => None,
        None => {
            *slot = Some(value);
            Some(())
        }
    }
}

fn is_name(value: &str) -> bool {
    value.starts_with(|c: char| c.is_ascii_uppercase()) && is_name_tail(value)
}

fn is_name_tail(value: &str) -> bool {
    value.chars().all(|c| c.is_ascii_alphanumeric())
}

// A path starting with ~ is under HOME, and a relative one is under the repo root.
fn expand(root: &Path, value: &str) -> Option<PathBuf> {
    let Some(rest) = value
        .strip_prefix('~')
        .filter(|rest| rest.is_empty() || rest.starts_with('/'))
    else {
        return Some(root.join(value));
    };
    let home = std::env::var_os("HOME").filter(|home| !home.is_empty())?;
    let home = PathBuf::from(home);
    match rest.trim_start_matches('/') {
        "" => Some(home),
        rest => Some(home.join(rest)),
    }
}
