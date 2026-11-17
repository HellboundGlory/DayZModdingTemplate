mod common;

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use common::{checks, code, stdout, text, TempDir, ACME_FAMILY};

const LONG_AGO: u64 = 1_700_000_000;

fn stamp(day: u32) -> String {
    format!("2026-11-{day:02}_12-00-00")
}

fn strs(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| name.to_string()).collect()
}

fn runs(prefix: &str, extension: &str, days: &[u32]) -> Vec<String> {
    days.iter()
        .map(|day| format!("{prefix}_{}.{extension}", stamp(*day)))
        .collect()
}

fn run_files(prefix: &str, day: u32) -> Vec<String> {
    vec![
        format!("{prefix}_{}.RPT", stamp(day)),
        format!("{prefix}_{}.ADM", stamp(day)),
        format!("script_{}.log", stamp(day)),
        format!("error_{}.log", stamp(day)),
    ]
}

fn concat(parts: &[Vec<String>]) -> Vec<String> {
    parts.concat()
}

fn set_modified(path: &Path, seconds: u64) {
    File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(UNIX_EPOCH + Duration::from_secs(seconds))
        .unwrap();
}

fn make_old_files(root: &TempDir, names: &[String]) {
    for name in names {
        let path = root.write(name, "x");
        set_modified(&path, LONG_AGO);
    }
}

fn listing(root: &Path) -> Vec<String> {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<String>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(base, &path, out);
            } else {
                let relative: PathBuf = path.strip_prefix(base).unwrap().to_path_buf();
                out.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

fn prune(root: &Path, extra: &[&str]) -> std::process::Output {
    let family = TempDir::new("prune-family");
    family.write("tools/family.conf", ACME_FAMILY);
    let mut args = vec![
        "prune-server-logs",
        root.to_str().unwrap(),
        "--root",
        family.path().to_str().unwrap(),
    ];
    args.extend_from_slice(extra);
    checks(&args)
}

type Case = (&'static str, Vec<&'static str>, Vec<String>, Vec<String>);

#[test]
fn what_stays_and_what_goes() {
    let five_runs = concat(
        &(1..=5)
            .map(|day| run_files("DayZDiag_x64", day))
            .collect::<Vec<_>>(),
    );
    let last_two = concat(&[run_files("DayZDiag_x64", 4), run_files("DayZDiag_x64", 5)]);
    let old_extras = concat(&[
        runs("crash", "log", &[1]),
        runs("ErrorMessage_DayZDiag_x64", "mdmp", &[2]),
        runs("FROZEN_DayZDiag_x64", "mdmp", &[3]),
    ]);
    let new_extras = concat(&[
        runs("crash", "log", &[4]),
        runs("ErrorMessage_DayZDiag_x64", "mdmp", &[5]),
    ]);
    let late_adm = strs(&[
        "DayZServer_2026-11-05_12-00-01.ADM",
        "script_2026-11-05_11-59-59.log",
    ]);
    let boot_folders = |days: &[u32]| -> Vec<String> {
        ["EventManagerLog", "WebApiLog"]
            .iter()
            .flat_map(|folder| {
                days.iter()
                    .map(move |day| format!("{folder}/{folder}_{}.log", stamp(*day)))
            })
            .collect()
    };
    let not_logs = strs(&[
        "acmecoretests_admins",
        "acmecoretests_mortal",
        "scriptExt.log",
        "script_notadate.log",
        &format!("notes_{}.txt", stamp(1)),
        &format!("AcmeCore/logs/AcmeCore_{}.log", stamp(1)),
        &format!("AcmeCore/audit/audit_{}.log", stamp(1)),
        "AcmeCore/settings.json",
        "AcmeCoreTests/golden/a.bin",
        &format!("users/script_{}.log", stamp(1)),
        "AcmeCoreTestsLooksLikeAFile",
    ]);
    let cases: Vec<Case> = vec![
        (
            "only the last two runs stay",
            vec![],
            five_runs.clone(),
            last_two.clone(),
        ),
        (
            "old crash files go with their runs",
            vec![],
            concat(&[five_runs.clone(), old_extras.clone(), new_extras.clone()]),
            concat(&[last_two.clone(), new_extras.clone()]),
        ),
        (
            "another server's old run goes",
            vec![],
            concat(&[run_files("DayZServer", 1), last_two.clone()]),
            last_two.clone(),
        ),
        (
            "files a second off their run stay",
            vec!["--keep", "1"],
            concat(&[
                run_files("DayZDiag_x64", 4),
                runs("DayZServer", "RPT", &[5]),
                late_adm.clone(),
            ]),
            concat(&[runs("DayZServer", "RPT", &[5]), late_adm.clone()]),
        ),
        (
            "the engine's per-boot folders follow the same runs",
            vec![],
            concat(&[last_two.clone(), boot_folders(&[1, 4, 5])]),
            concat(&[last_two.clone(), boot_folders(&[4, 5])]),
        ),
        (
            "an impossible date stays and stops nothing",
            vec![],
            concat(&[five_runs.clone(), strs(&["script_2026-13-45_12-00-00.log"])]),
            concat(&[last_two.clone(), strs(&["script_2026-13-45_12-00-00.log"])]),
        ),
        (
            "a hand-saved copy stays",
            vec![],
            concat(&[five_runs.clone(), runs("before-fix", "log", &[1])]),
            concat(&[last_two.clone(), runs("before-fix", "log", &[1])]),
        ),
        (
            "keep one",
            vec!["--keep", "1"],
            last_two.clone(),
            run_files("DayZDiag_x64", 5),
        ),
        (
            "fewer runs than the keep count",
            vec![],
            run_files("DayZDiag_x64", 1),
            run_files("DayZDiag_x64", 1),
        ),
        (
            "no run report keeps everything",
            vec![],
            runs("script", "log", &[1, 2, 3]),
            runs("script", "log", &[1, 2, 3]),
        ),
        (
            "anything else stays",
            vec!["--keep", "1"],
            not_logs.clone(),
            not_logs.clone(),
        ),
        (
            "leftover check folders go",
            vec![],
            strs(&[
                "AcmeCoreTestsSweep1/a.json",
                "AcmeCoreTestsReloadCut/a.json",
                "AcmeCoreTests/a.json",
            ]),
            strs(&["AcmeCoreTests/a.json"]),
        ),
    ];
    for (label, args, before, after) in cases {
        let profiles = TempDir::new("prune");
        make_old_files(&profiles, &before);
        let done = prune(profiles.path(), &args);
        assert_eq!(
            done.status.code(),
            Some(0),
            "{label}: {}{}",
            text(&done.stdout),
            text(&done.stderr)
        );
        let mut expected = after.clone();
        expected.sort();
        assert_eq!(listing(profiles.path()), expected, "{label}");
    }
}

#[test]
fn keep_below_one_is_refused() {
    let profiles = TempDir::new("prune-keep");
    let path = profiles.write(&format!("script_{}.log", stamp(1)), "x");
    let done = prune(profiles.path(), &["--keep", "0"]);
    assert_ne!(done.status.code(), Some(0), "{}", text(&done.stdout));
    assert!(path.exists());
}

#[test]
fn a_file_still_being_written_stays() {
    let profiles = TempDir::new("prune-live");
    let names: Vec<String> = (1..=4)
        .flat_map(|day| run_files("DayZDiag_x64", day))
        .collect();
    make_old_files(&profiles, &names);
    let live = profiles.write(&format!("DayZServer_{}.RPT", stamp(1)), "x");
    let just_now = SystemTime::now() - Duration::from_secs(60);
    File::options()
        .write(true)
        .open(&live)
        .unwrap()
        .set_modified(just_now)
        .unwrap();
    prune(profiles.path(), &[]);
    assert!(live.exists());
}

#[test]
fn a_file_it_cannot_remove_warns_and_stops_nothing() {
    use std::os::unix::fs::PermissionsExt;
    if common::running_as_root() {
        return;
    }
    let profiles = TempDir::new("prune-locked");
    let names: Vec<String> = (1..=3)
        .flat_map(|day| run_files("DayZDiag_x64", day))
        .collect();
    make_old_files(&profiles, &names);
    fs::set_permissions(profiles.path(), fs::Permissions::from_mode(0o555)).unwrap();
    let done = prune(profiles.path(), &[]);
    fs::set_permissions(profiles.path(), fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        done.status.code(),
        Some(0),
        "{}{}",
        text(&done.stdout),
        text(&done.stderr)
    );
    assert!(
        text(&done.stdout).contains("could not remove"),
        "{}",
        text(&done.stdout)
    );
}

#[test]
fn only_python_word_characters_make_a_name() {
    let profiles = TempDir::new("prune-word-chars");
    let mut before: Vec<String> = Vec::new();
    before.push("DayZ\u{203f}Diag_x64_2026-11-01_12-00-00.RPT".to_string());
    before.extend(run_files("DayZDiag_x64", 2));
    before.extend(run_files("DayZDiag_x64", 3));
    make_old_files(&profiles, &before);
    let done = prune(profiles.path(), &[]);
    assert_eq!(done.status.code(), Some(0), "{}", text(&done.stdout));
    let mut expected = before.clone();
    expected.sort();
    assert_eq!(listing(profiles.path()), expected);
}

#[test]
fn a_superscript_two_is_a_word_character() {
    let profiles = TempDir::new("prune-superscript");
    let mut before: Vec<String> = vec![format!("DayZ\u{b2}Diag_x64_{}.RPT", stamp(1))];
    before.extend(run_files("DayZDiag_x64", 2));
    before.extend(run_files("DayZDiag_x64", 3));
    make_old_files(&profiles, &before);
    fs::create_dir_all(profiles.join("AcmeCoreTestsA\u{b2}")).unwrap();
    make_old_files(&profiles, &["AcmeCoreTestsA\u{b2}/a.json".to_string()]);
    let done = prune(profiles.path(), &[]);
    assert_eq!(done.status.code(), Some(0), "{}", text(&done.stdout));
    assert!(!profiles
        .join(&format!("DayZ\u{b2}Diag_x64_{}.RPT", stamp(1)))
        .exists());
    assert!(!profiles.join("AcmeCoreTestsA\u{b2}").exists());
}

#[test]
fn a_name_ending_in_a_newline_still_matches() {
    let profiles = TempDir::new("prune-newline-name");
    let mut before = vec![format!("script_{}.log\n", stamp(1))];
    before.extend(run_files("DayZDiag_x64", 2));
    before.extend(run_files("DayZDiag_x64", 3));
    make_old_files(&profiles, &before);
    let done = prune(profiles.path(), &[]);
    assert_eq!(done.status.code(), Some(0), "{}", text(&done.stdout));
    assert!(!profiles
        .join(&format!("script_{}.log\n", stamp(1)))
        .exists());
}

#[test]
fn a_stamp_in_unicode_digits_is_read_by_value() {
    let profiles = TempDir::new("prune-unicode-digits");
    let fullwidth = "DayZServer_\u{ff12}\u{ff10}\u{ff12}\u{ff16}-11-01_12-00-00.RPT";
    let mut before = vec![fullwidth.to_string()];
    before.extend(run_files("DayZDiag_x64", 2));
    before.extend(run_files("DayZDiag_x64", 3));
    make_old_files(&profiles, &before);
    let done = prune(profiles.path(), &[]);
    assert_eq!(done.status.code(), Some(0), "{}", text(&done.stdout));
    assert!(!profiles.join(fullwidth).exists());
}

#[test]
fn a_cutoff_before_year_one_removes_nothing() {
    let profiles = TempDir::new("prune-year-one");
    let before = strs(&[
        "DayZServer_0001-01-01_00-00-30.RPT",
        "DayZDiag_x64_0001-01-01_00-00-40.RPT",
        "script_0001-01-01_00-00-10.log",
        "AcmeCoreTestsOld/a.json",
    ]);
    make_old_files(&profiles, &before);
    let done = prune(profiles.path(), &[]);
    assert_eq!(
        done.status.code(),
        Some(1),
        "{}{}",
        text(&done.stdout),
        text(&done.stderr)
    );
    assert_eq!(text(&done.stdout), "");
    let mut expected = before.clone();
    expected.sort();
    assert_eq!(listing(profiles.path()), expected);
}

#[test]
fn a_check_folder_failure_names_the_folder_removed() {
    if common::running_as_root() {
        return;
    }
    use std::os::unix::fs::PermissionsExt;
    let profiles = TempDir::new("prune-top-folder");
    profiles.write("AcmeCoreTestsLocked/locked/a.json", "x");
    let locked = profiles.join("AcmeCoreTestsLocked/locked");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o555)).unwrap();
    let done = prune(profiles.path(), &[]);
    let out = text(&done.stdout);
    assert_eq!(done.status.code(), Some(0), "{out}{}", text(&done.stderr));
    let top = profiles.join("AcmeCoreTestsLocked");
    assert!(
        out.contains(&format!(
            "server logs: could not remove {}: ",
            top.display()
        )),
        "{out}"
    );
    assert!(out.contains(&format!("'{}'", locked.display())), "{out}");
}

#[test]
fn a_fullwidth_month_is_not_a_date() {
    let profiles = TempDir::new("prune-fullwidth-month");
    let kept = "DayZServer_2026-\u{ff11}\u{ff11}-01_12-00-00.RPT";
    let mut before = vec![kept.to_string()];
    before.extend(run_files("DayZDiag_x64", 2));
    before.extend(run_files("DayZDiag_x64", 3));
    make_old_files(&profiles, &before);
    let done = prune(profiles.path(), &[]);
    assert_eq!(done.status.code(), Some(0), "{}", text(&done.stdout));
    assert!(profiles.join(kept).exists());
}

#[test]
fn the_check_folders_follow_the_family_core() {
    let family = TempDir::new("prune-family-root");
    family.write("tools/family.conf", ACME_FAMILY);
    let profiles = TempDir::new("prune-family-profiles");
    profiles.write("AcmeCoreTestsA/a.json", "x");
    profiles.write("OutsideCoreTestsA/a.json", "x");
    let root = family.path().to_str().unwrap();
    let done = checks(&[
        "prune-server-logs",
        profiles.path().to_str().unwrap(),
        "--root",
        root,
    ]);
    assert_eq!(done.status.code(), Some(0), "{}", text(&done.stdout));
    assert!(!profiles.join("AcmeCoreTestsA").exists());
    assert!(profiles.join("OutsideCoreTestsA").exists());
}

#[test]
fn a_missing_profiles_folder_needs_no_family_file() {
    let empty = TempDir::new("prune-no-family");
    let out = checks(&[
        "prune-server-logs",
        "--root",
        empty.path().to_str().unwrap(),
        empty.join("absent").to_str().unwrap(),
    ]);
    assert_eq!(code(&out), Some(0), "{}", stdout(&out));
    assert_eq!(stdout(&out), "");
}
