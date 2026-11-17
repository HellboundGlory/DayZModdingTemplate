#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNT: AtomicUsize = AtomicUsize::new(0);

pub struct Scratch(PathBuf);

impl Scratch {
    pub fn new(label: &str) -> Scratch {
        let n = COUNT.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!("checks-{label}-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Scratch(path)
    }

    pub fn copy_of(label: &str, source: &Path) -> Scratch {
        let scratch = Scratch::new(label);
        copy_dir(source, &scratch.0);
        scratch
    }

    // A copy of a fixture with its edits, then its added files, then its removals: an add can overwrite an
    // edited file, and a remove can drop an added one.
    pub fn copy_with(
        label: &str,
        source: &Path,
        edits: &[(&str, &str, &str)],
        adds: &[(&str, &str)],
        removes: &[&str],
    ) -> Scratch {
        let scratch = Scratch::copy_of(label, source);
        for (rel, old, new) in edits {
            scratch.edit(rel, old, new);
        }
        for (rel, text) in adds {
            scratch.write(rel, text);
        }
        for rel in removes {
            scratch.remove(rel);
        }
        scratch
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    pub fn join(&self, rel: &str) -> PathBuf {
        self.0.join(rel)
    }

    pub fn write(&self, rel: &str, text: &str) {
        write_file(&self.0.join(rel), text);
    }

    pub fn edit(&self, rel: &str, old: &str, new: &str) {
        let path = self.0.join(rel);
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains(old), "{rel} has no {old:?}");
        fs::write(&path, text.replacen(old, new, 1)).unwrap();
    }

    pub fn remove(&self, rel: &str) {
        fs::remove_file(self.0.join(rel)).unwrap();
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub fn write_file(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

pub fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn fixture(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(rel)
}

pub fn checks(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_checks"))
        .args(args)
        .output()
        .unwrap()
}

pub fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

pub fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

pub fn code(out: &Output) -> Option<i32> {
    out.status.code()
}

// A case's exit code and its needle, both failures naming the case and showing the output.
pub fn expect(name: &str, out: &Output, exit: i32, needle: &str) {
    assert_eq!(code(out), Some(exit), "{name}: {}", stdout(out));
    assert!(stdout(out).contains(needle), "{name}: {}", stdout(out));
}

pub fn text(output: &[u8]) -> String {
    String::from_utf8_lossy(output).into_owned()
}

pub fn running_as_root() -> bool {
    fs::read_to_string("/proc/self/status")
        .map(|status| {
            status
                .lines()
                .any(|line| line.starts_with("Uid:") && line.split_whitespace().nth(2) == Some("0"))
        })
        .unwrap_or(false)
}

pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new(label: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!("checks-{label}-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        TempDir(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    pub fn join(&self, relative: &str) -> PathBuf {
        self.0.join(relative)
    }

    pub fn write(&self, relative: &str, contents: &str) -> PathBuf {
        self.write_bytes(relative, contents.as_bytes())
    }

    pub fn write_bytes(&self, relative: &str, contents: &[u8]) -> PathBuf {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, contents).unwrap();
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        make_writable(&self.0);
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn make_writable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = fs::symlink_metadata(path) {
        if meta.is_dir() {
            let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o755));
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    make_writable(&entry.path());
                }
            }
        }
    }
}

// A made-up family, so each check is proved to read tools/family.conf and not fixed names.
pub const ACME_FAMILY: &str =
    "prefix = Acme\ncore = AcmeCore\nframework = Kit KIT_Scripts KIT_GUI\n";

// The same family with a second framework, for the cases that need one.
pub const ACME_FAMILY_TWO_FRAMEWORKS: &str = "prefix = Acme\ncore = AcmeCore\n\
    framework = Kit KIT_Scripts KIT_GUI\nframework = Forge FORGE_Scripts FORGE_GUI\n";

// Puts a family.conf in a scratch root.
pub fn write_family(scratch: &Scratch, conf: &str) {
    scratch.write("tools/family.conf", conf);
}
