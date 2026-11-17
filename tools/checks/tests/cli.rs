mod common;

use common::checks;

#[test]
fn unknown_command_is_refused_with_the_command_list() {
    let out = checks(&["no-such-command"]);
    assert_eq!(out.status.code(), Some(2));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("unknown command: no-such-command"), "{err}");
    assert!(err.contains("commands:"), "{err}");
}

#[test]
fn no_command_prints_usage() {
    let out = checks(&[]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).starts_with("usage: checks <command>"));
}

#[test]
fn help_flags_list_every_command() {
    let names = [
        "check-boundaries",
        "check-dependencies",
        "check-inputs",
        "check-modded",
        "check-pbo",
        "check-perf-static",
        "check-reference",
        "family",
        "prune-server-logs",
        "sync-version",
    ];
    for flag in ["--help", "-h", "help"] {
        let out = checks(&[flag]);
        let text = String::from_utf8_lossy(&out.stdout);
        assert_eq!(out.status.code(), Some(0), "{flag}: {text}");
        assert!(
            text.contains("usage: checks <command> [args]"),
            "{flag}: {text}"
        );
        for name in names {
            assert!(text.contains(name), "{flag} lacks {name}: {text}");
        }
    }
}

#[test]
fn a_non_utf8_argument_is_refused_in_one_line() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_checks"))
        .args(["check-pbo", "--list-file"])
        .arg(OsStr::from_bytes(b"lst\xff.txt"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("not valid UTF-8"), "{err}");
    assert_eq!(err.lines().count(), 1, "{err}");
}
