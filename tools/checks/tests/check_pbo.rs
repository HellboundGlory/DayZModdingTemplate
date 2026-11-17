mod common;

use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Output};

use common::{checks, fixture, text, write_file, Scratch};

fn run_fixture(name: &str) -> std::process::Output {
    let path = fixture(&format!("check_pbo/{name}"));
    checks(&["check-pbo", "--list-file", path.to_str().unwrap()])
}

#[test]
fn list_file_outcomes() {
    let cases = [
        ("clean.txt", 0, "OK: 2 packed paths", "stdout"),
        ("crlf.txt", 0, "OK: 1 packed path", "stdout"),
        ("saved-settings.txt", 0, "OK: 2 packed paths", "stdout"),
        ("docs.txt", 1, "docs/notes.md: docs/ never ships", "stdout"),
        (
            "tools.txt",
            1,
            "tools/check_pbo.py: tools/ never ships",
            "stdout",
        ),
        (
            "tests.txt",
            1,
            "tests/probe.c: tests/ never ships",
            "stdout",
        ),
        (
            "server.txt",
            1,
            "server/sub/deep.c: server/ never ships",
            "stdout",
        ),
        ("nested-server.txt", 0, "OK: 2 packed paths", "stdout"),
        (
            "nested-tests.txt",
            1,
            "Scripts/3_Game/tests/t.c: tests/ never ships",
            "stdout",
        ),
        (
            "nested-test.txt",
            1,
            "Test/t.c: Test/ never ships",
            "stdout",
        ),
        (
            "nested-docs.txt",
            1,
            "Scripts/3_Game/docs/x.c: docs/ never ships",
            "stdout",
        ),
        (
            "nested-markdown.txt",
            1,
            "Scripts/notes.md: markdown never ships",
            "stdout",
        ),
        (
            "mods-path.txt",
            1,
            "mods/AcmeCore/x.c: mods/ never ships",
            "stdout",
        ),
        (
            "root-markdown.txt",
            1,
            "CHANGELOG.md: markdown never ships",
            "stdout",
        ),
        (
            "brackets.txt",
            1,
            "notes [draft].md: markdown never ships",
            "stdout",
        ),
        ("empty.txt", 2, "no packed paths", "stderr"),
        ("all-excluded.txt", 2, "no packed paths", "stderr"),
        ("no-config.txt", 2, "no config.cpp packed", "stderr"),
        ("nested-config.txt", 2, "no config.cpp packed", "stderr"),
        ("unparsable.txt", 2, "cannot parse", "stderr"),
        ("summary-only.txt", 2, "cannot parse", "stderr"),
        ("truncated.txt", 2, "cannot parse", "stderr"),
        ("count-mismatch.txt", 2, "cannot parse", "stderr"),
    ];
    for (name, code, needle, stream) in cases {
        let done = run_fixture(name);
        assert_eq!(
            done.status.code(),
            Some(code),
            "{name}: {}{}",
            text(&done.stdout),
            text(&done.stderr)
        );
        let shown = if stream == "stdout" {
            text(&done.stdout)
        } else {
            text(&done.stderr)
        };
        assert!(shown.contains(needle), "{name}: {shown}");
    }
}

#[test]
fn a_warning_is_printed_through() {
    let done = run_fixture("warning.txt");
    assert_eq!(
        done.status.code(),
        Some(0),
        "{}{}",
        text(&done.stdout),
        text(&done.stderr)
    );
    let out = text(&done.stdout);
    assert!(
        out.contains("warning: 2 config files will be packed"),
        "{out}"
    );
    assert!(out.contains("OK: 1 packed path"), "{out}");
}

#[test]
fn every_violation_is_named() {
    let done = run_fixture("several.txt");
    assert_eq!(done.status.code(), Some(1), "{}", text(&done.stderr));
    let out = text(&done.stdout);
    for path in ["docs/a.md", "tools/t.py", "Tools/x.c", "CHANGELOG.md"] {
        assert!(out.contains(path), "{out}");
    }
    assert!(out.contains("FAILED: 4 packed paths"), "{out}");
    assert_eq!(out.matches('\n').count(), 5, "{out}");
}

#[test]
fn a_clean_path_is_not_reported() {
    let done = run_fixture("docs.txt");
    let out = text(&done.stdout);
    assert!(
        !out.contains("config.cpp"),
        "a clean path must not be reported: {out}"
    );
    assert_eq!(out.matches('\n').count(), 2, "{out}");
}

#[test]
fn arguments_are_required_once() {
    let clean = fixture("check_pbo/clean.txt");
    let clean = clean.to_str().unwrap();
    let missing = fixture("check_pbo/missing.txt");
    let missing = missing.to_str().unwrap();
    let cases: [(Vec<&str>, &str); 3] = [
        (vec![], "give a mod root"),
        (vec!["mods/AcmeCore", "--list-file", clean], "not both"),
        (vec!["--list-file", missing], "cannot read"),
    ];
    for (args, needle) in cases {
        let done = checks(&[&["check-pbo"], args.as_slice()].concat());
        assert_eq!(
            done.status.code(),
            Some(2),
            "{args:?}: {}",
            text(&done.stdout)
        );
        let shown = text(&done.stdout) + &text(&done.stderr);
        assert!(shown.contains(needle), "{args:?}: {shown}");
    }
    assert!(!text(&checks(&["check-pbo"]).stderr).contains("not both"));
}

#[test]
fn a_failed_mf_is_not_a_pass() {
    let mod_root = fixture("check_pbo/no-such-mod");
    let done = checks(&["check-pbo", mod_root.to_str().unwrap()]);
    assert_eq!(
        done.status.code(),
        Some(2),
        "{}{}",
        text(&done.stdout),
        text(&done.stderr)
    );
    assert!(
        text(&done.stderr).contains("mf failed with exit"),
        "{}",
        text(&done.stderr)
    );
}

fn run_with_path(path: &str, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_checks"))
        .env("PATH", path)
        .args(args)
        .output()
        .unwrap()
}

// A stand-in `mf` that answers `--version` as the given line and fails every other call.
fn stand_in_mf(scratch: &Scratch, version: &str) -> std::path::PathBuf {
    let bin = scratch.join("bin");
    let script = format!(
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo \"{version}\"; exit 0; fi\necho \"stand-in mf ran\" >&2\nexit 1\n"
    );
    write_file(&bin.join("mf"), &script);
    std::fs::set_permissions(bin.join("mf"), std::fs::Permissions::from_mode(0o755)).unwrap();
    bin
}

#[test]
fn without_modfoundry_the_build_check_is_skipped() {
    let scratch = Scratch::new("pbo-no-modfoundry");
    let metafont = stand_in_mf(&scratch, "This is METAFONT, Version 2.71828182");
    let empty = scratch.join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    let mod_root = scratch.path().to_str().unwrap().to_string();
    let cases = [
        (
            "METAFONT first on PATH",
            metafont,
            "skipped: ModFoundry mf not found first on PATH",
        ),
        (
            "no mf on PATH",
            empty,
            "skipped: ModFoundry mf not found on PATH",
        ),
    ];
    for (name, bin, needle) in cases {
        let done = run_with_path(bin.to_str().unwrap(), &["check-pbo", &mod_root]);
        let out = text(&done.stdout);
        assert_eq!(
            done.status.code(),
            Some(0),
            "{name}: {out}{}",
            text(&done.stderr)
        );
        assert!(out.starts_with(needle), "{name}: {out}");
        assert!(
            !text(&done.stderr).contains("stand-in mf ran"),
            "{name}: mf build ran"
        );
    }
}
