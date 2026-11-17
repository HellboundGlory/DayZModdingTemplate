mod common;

use std::fs;
use std::path::Path;

use common::{checks, text, TempDir};

fn mod_cpp(name: &str, version: &str) -> String {
    format!("name = \"{name}\";\nauthor = \"Acme\";\nversion = \"{version}\";\n")
}

fn mod_config(name: &str, version: &str) -> String {
    format!(
        "class CfgPatches\n{{\n    class {name}\n    {{\n        units[] = {{}};\n        weapons[] = {{}};\n        requiredVersion = 0.1;\n    }};\n}};\n\nclass CfgMods\n{{\n    class {name}\n    {{\n        type = \"mod\";\n        dir = \"{name}\";\n        version = \"{version}\";\n    }};\n}};\n"
    )
}

// Writes the family file (when given), each mod and test mod with its config, then the raw files.
// A raw mod.cpp gets a matching config.cpp unless the raw set already has one.
fn make_tree(
    root: &TempDir,
    family: Option<&str>,
    mods: &[(&str, &str)],
    tests: &[(&str, &str)],
    raw: &[(&str, String)],
) {
    if let Some(family) = family {
        root.write("VERSION", &format!("{family}\n"));
    }
    for (folder, names) in [("mods", mods), ("tests", tests)] {
        for (name, version) in names {
            root.write(&format!("{folder}/{name}/mod.cpp"), &mod_cpp(name, version));
            root.write(
                &format!("{folder}/{name}/config.cpp"),
                &mod_config(name, version),
            );
        }
    }
    for (rel, body) in raw {
        root.write(rel, body);
    }
    for (rel, body) in raw {
        let Some(folder) = rel.strip_suffix("mod.cpp") else {
            continue;
        };
        let config = format!("{folder}config.cpp");
        if !root.join(&config).exists() {
            let version = version_in(body).unwrap_or_else(|| "0.1.0".to_string());
            let name = Path::new(rel)
                .parent()
                .unwrap()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned();
            root.write(&config, &mod_config(&name, &version));
        }
    }
}

#[test]
fn a_write_on_an_empty_mods_folder_still_errors() {
    let root = TempDir::new("sync-write-empty");
    root.write("VERSION", "0.1.0\n");
    root.write("mods/.gitkeep", "");
    let done = run(root.path(), &["--set", "0.2.0"]);
    assert_eq!(done.status.code(), Some(2), "{}", text(&done.stdout));
    assert!(
        text(&done.stdout).contains("cannot run: no mods under"),
        "{}",
        text(&done.stdout)
    );
}

fn version_in(body: &str) -> Option<String> {
    let start = body.find("version")? + "version".len();
    let rest = body[start..]
        .trim_start()
        .strip_prefix('=')?
        .trim_start()
        .strip_prefix('"')?;
    Some(rest[..rest.find('"')?].to_string())
}

fn run(root: &Path, extra: &[&str]) -> std::process::Output {
    let root = root.to_str().unwrap();
    let mut args = vec!["sync-version", "--root", root];
    args.extend_from_slice(extra);
    checks(&args)
}

struct Case {
    name: &'static str,
    family: Option<&'static str>,
    mods: Vec<(&'static str, &'static str)>,
    tests: Vec<(&'static str, &'static str)>,
    raw: Vec<(&'static str, String)>,
    code: i32,
    needles: Vec<&'static str>,
}

fn case(
    name: &'static str,
    family: Option<&'static str>,
    mods: &[(&'static str, &'static str)],
    tests: &[(&'static str, &'static str)],
    raw: Vec<(&'static str, String)>,
    code: i32,
    needles: &[&'static str],
) -> Case {
    Case {
        name,
        family,
        mods: mods.to_vec(),
        tests: tests.to_vec(),
        raw,
        code,
        needles: needles.to_vec(),
    }
}

#[test]
fn outcomes() {
    let core = ("AcmeCore", "0.1.0");
    let cases = vec![
        case("clean at 0.1.0", Some("0.1.0"), &[core, ("AcmeMarket", "0.1.0")], &[("AcmeCoreTests", "0.1.0")], vec![], 0, &["OK: 3 mod files"]),
        case("a mod off at 0.x fails", Some("0.1.0"), &[core, ("AcmeMarket", "0.2.0")], &[("AcmeCoreTests", "0.1.0")], vec![], 1, &["mods/AcmeMarket/mod.cpp: version 0.2.0, expected 0.1.0"]),
        case("every problem is printed", Some("0.1.0"), &[core, ("AcmeMarket", "0.2.0"), ("AcmeGroups", "0.3.0")], &[], vec![], 1, &["FAILED: 2 problems", "mods/AcmeMarket", "mods/AcmeGroups"]),
        case("a mod above 1.0.0 passes", Some("1.0.0"), &[("AcmeCore", "1.0.0"), ("AcmeMarket", "1.2.3")], &[("AcmeCoreTests", "1.0.0")], vec![], 0, &["OK: 3 mod files"]),
        case("a mod on another major fails", Some("1.0.0"), &[("AcmeCore", "1.0.0"), ("AcmeMarket", "2.0.0")], &[("AcmeCoreTests", "1.0.0")], vec![], 1, &["mods/AcmeMarket/mod.cpp: version 2.0.0, major differs from VERSION 1.0.0"]),
        case("a mod below VERSION fails", Some("1.2.0"), &[("AcmeCore", "1.2.0"), ("AcmeMarket", "1.1.0")], &[("AcmeCoreTests", "1.2.0")], vec![], 1, &["mods/AcmeMarket/mod.cpp: version 1.1.0, below VERSION 1.2.0"]),
        case("a test mod off its mod fails", Some("0.1.0"), &[core], &[("AcmeCoreTests", "0.1.1")], vec![], 1, &["tests/AcmeCoreTests/mod.cpp: version 0.1.1, differs from AcmeCore at 0.1.0"]),
        case("a config.cpp version that drifts fails", Some("0.1.0"), &[core], &[], vec![("mods/AcmeCore/config.cpp", mod_config("AcmeCore", "0.2.0"))], 1, &["mods/AcmeCore/config.cpp: version 0.2.0, differs from mods/AcmeCore/mod.cpp at 0.1.0"]),
        case("a test mod's config.cpp that drifts fails", Some("0.1.0"), &[core], &[("AcmeCoreTests", "0.1.0")], vec![("tests/AcmeCoreTests/config.cpp", mod_config("AcmeCoreTests", "0.2.0"))], 1, &["tests/AcmeCoreTests/config.cpp: version 0.2.0, differs from tests/AcmeCoreTests/mod.cpp at 0.1.0"]),
        case("a config.cpp without a version line cannot run", Some("0.1.0"), &[core], &[], vec![("mods/AcmeCore/config.cpp", "class CfgPatches\n{\n    class AcmeCore\n    {\n    };\n};\n".to_string())], 2, &["cannot run: mods/AcmeCore/config.cpp has no version line"]),
        case("two config.cpp version lines cannot run", Some("0.1.0"), &[core], &[], vec![("mods/AcmeCore/config.cpp", "version = \"0.1.0\";\nversion = \"0.1.0\";\n".to_string())], 2, &["cannot run: mods/AcmeCore/config.cpp has 2 version lines"]),
        case("a short config.cpp version cannot run", Some("0.1.0"), &[core], &[], vec![("mods/AcmeCore/config.cpp", "version = \"0.1\";\n".to_string())], 2, &["cannot run: mods/AcmeCore/config.cpp: not x.y.z"]),
        case("requiredVersion is not the version line", Some("0.1.0"), &[core], &[], vec![("mods/AcmeCore/config.cpp", "class CfgPatches\n{\n    class AcmeCore\n    {\n        requiredVersion = 0.2;\n    };\n};\n\nclass CfgMods\n{\n    class AcmeCore\n    {\n        version = \"0.1.0\";\n    };\n};\n".to_string())], 0, &["OK: 1 mod file"]),
        case("a missing VERSION cannot run", None, &[core], &[], vec![], 2, &["cannot run: VERSION missing"]),
        case("a short VERSION cannot run", Some("0.1"), &[core], &[], vec![], 2, &["cannot run: VERSION: not x.y.z"]),
        case("a double VERSION cannot run", Some("0.1.0\n0.1.0"), &[core], &[], vec![], 2, &["cannot run: VERSION: not x.y.z"]),
        case("a mod folder without mod.cpp cannot run", Some("0.1.0"), &[], &[], vec![("mods/AcmeCore/config.cpp", "class CfgPatches\n{\n};\n".to_string())], 2, &["cannot run: mods/AcmeCore/mod.cpp missing"]),
        case("a mod.cpp without a version line cannot run", Some("0.1.0"), &[], &[], vec![("mods/AcmeCore/mod.cpp", "name = \"Acme Core\";\n".to_string())], 2, &["cannot run: mods/AcmeCore/mod.cpp has no version line"]),
        case("two version lines cannot run", Some("0.1.0"), &[], &[], vec![("mods/AcmeCore/mod.cpp", "version = \"0.1.0\";\nversion = \"0.1.0\";\n".to_string())], 2, &["cannot run: mods/AcmeCore/mod.cpp has 2 version lines"]),
        case("a test mod for an absent mod cannot run", Some("0.1.0"), &[core], &[("AcmeMarketTests", "0.1.0")], vec![], 2, &["cannot run: tests/AcmeMarketTests/mod.cpp: no AcmeMarket in mods"]),
        case("a tests folder without the suffix is left alone", Some("0.1.0"), &[core], &[], vec![("tests/fixtures/thing/mod.cpp", "version = \"9.9.9\";\n".to_string())], 0, &["OK: 1 mod file"]),
        case("a version line with a trailing comment passes", Some("0.1.0"), &[], &[], vec![("mods/AcmeCore/mod.cpp", "name = \"Acme Core\";\nversion = \"0.1.0\"; // keep in sync\n".to_string())], 0, &["OK: 1 mod file"]),
        case("a commented-out version does not count", Some("0.1.0"), &[], &[], vec![("mods/AcmeCore/mod.cpp", "name = \"Acme Core\";\n/*\nversion = \"0.1.0\";\n*/\n".to_string())], 2, &["cannot run: mods/AcmeCore/mod.cpp has no version line"]),
        case("an old version in a comment is ignored", Some("0.1.0"), &[], &[], vec![("mods/AcmeCore/mod.cpp", "version = \"0.1.0\";\n/*\nversion = \"0.0.9\";\n*/\n".to_string())], 0, &["OK: 1 mod file"]),
        case("a leading zero in a mod cannot run", Some("0.1.0"), &[("AcmeCore", "0.01.0")], &[], vec![], 2, &["cannot run: mods/AcmeCore/mod.cpp: not x.y.z"]),
        case("a second badly formed version line cannot run", Some("0.1.0"), &[], &[], vec![("mods/AcmeCore/mod.cpp", "version = \"0.1.0\";\nversion = \"0.2\";\nVersion = \"9.9.9\";\n".to_string())], 2, &["cannot run: mods/AcmeCore/mod.cpp has 3 version lines"]),
        case("a comment opener inside a string is only text", Some("0.1.0"), &[], &[], vec![("mods/AcmeCore/mod.cpp", "overview = \"a /* b\";\nversion = \"0.1.0\";\nname = \"c */ d\";\n".to_string())], 0, &["OK: 1 mod file"]),
        case("a tree without mods cannot run", Some("0.1.0"), &[], &[], vec![], 2, &["cannot run: no mods under"]),
        case("an empty mods folder is skipped", Some("0.1.0"), &[], &[], vec![("mods/.gitkeep", String::new())], 0, &["skipped: no mods under"]),
    ];
    for case in cases {
        let root = TempDir::new("sync-outcome");
        make_tree(&root, case.family, &case.mods, &case.tests, &case.raw);
        let done = run(root.path(), &[]);
        assert_eq!(
            done.status.code(),
            Some(case.code),
            "{}: {}{}",
            case.name,
            text(&done.stdout),
            text(&done.stderr)
        );
        for needle in &case.needles {
            assert!(
                text(&done.stdout).contains(needle),
                "{}: {}",
                case.name,
                text(&done.stdout)
            );
        }
    }
}

#[test]
fn a_missing_config_cannot_run() {
    let root = TempDir::new("sync-missing-config");
    make_tree(&root, Some("0.1.0"), &[("AcmeCore", "0.1.0")], &[], &[]);
    fs::remove_file(root.join("mods/AcmeCore/config.cpp")).unwrap();
    let done = run(root.path(), &[]);
    assert_eq!(
        done.status.code(),
        Some(2),
        "{}{}",
        text(&done.stdout),
        text(&done.stderr)
    );
    assert!(
        text(&done.stdout).contains("cannot run: mods/AcmeCore/config.cpp missing"),
        "{}",
        text(&done.stdout)
    );
}

#[test]
fn an_undecodable_file_cannot_run() {
    for rel in ["VERSION", "mods/AcmeCore/mod.cpp"] {
        let root = TempDir::new("sync-undecodable");
        make_tree(&root, Some("0.1.0"), &[("AcmeCore", "0.1.0")], &[], &[]);
        root.write_bytes(rel, b"\xff\xfe version = \"0.1.0\";\n");
        let done = run(root.path(), &[]);
        assert_eq!(
            done.status.code(),
            Some(2),
            "{rel}: {}{}",
            text(&done.stdout),
            text(&done.stderr)
        );
        assert!(
            text(&done.stdout).contains("cannot read"),
            "{rel}: {}",
            text(&done.stdout)
        );
        assert_eq!(text(&done.stderr), "", "{rel}");
    }
}

#[test]
fn a_folder_name_that_is_not_utf8_cannot_run() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    let root = TempDir::new("sync-bad-name");
    make_tree(&root, Some("0.1.0"), &[("AcmeCore", "0.1.0")], &[], &[]);
    fs::create_dir_all(root.join("mods").join(OsStr::from_bytes(b"bad\xff"))).unwrap();
    let done = run(root.path(), &[]);
    assert_eq!(
        done.status.code(),
        Some(2),
        "{}{}",
        text(&done.stdout),
        text(&done.stderr)
    );
    assert!(
        text(&done.stdout).contains("cannot run"),
        "{}",
        text(&done.stdout)
    );
    assert!(
        !text(&done.stderr).contains("Traceback"),
        "{}",
        text(&done.stderr)
    );
}

#[test]
fn a_misshapen_version_file_cannot_run() {
    for text_body in ["\n\n  0.1.0  \n\n", "0.01.0\n", "0.1.0\n0.1.0\n"] {
        let root = TempDir::new("sync-family-file");
        make_tree(&root, None, &[("AcmeCore", "0.1.0")], &[], &[]);
        root.write("VERSION", text_body);
        let done = run(root.path(), &[]);
        assert_eq!(
            done.status.code(),
            Some(2),
            "{text_body:?}: {}{}",
            text(&done.stdout),
            text(&done.stderr)
        );
        assert!(
            text(&done.stdout).contains("cannot run: VERSION"),
            "{text_body:?}: {}",
            text(&done.stdout)
        );
    }
}

#[test]
fn a_family_release_moves_every_file() {
    use std::os::unix::fs::PermissionsExt;
    let root = TempDir::new("sync-set");
    make_tree(
        &root,
        Some("0.1.0"),
        &[("AcmeCore", "0.1.0"), ("AcmeMarket", "0.1.0")],
        &[("AcmeCoreTests", "0.1.0"), ("AcmeMarketTests", "0.1.0")],
        &[],
    );
    let paths = [
        "mods/AcmeCore/mod.cpp",
        "mods/AcmeCore/config.cpp",
        "mods/AcmeMarket/mod.cpp",
        "mods/AcmeMarket/config.cpp",
    ];
    fs::set_permissions(root.join(paths[3]), fs::Permissions::from_mode(0o640)).unwrap();
    let modes: Vec<u32> = paths
        .iter()
        .map(|path| fs::metadata(root.join(path)).unwrap().permissions().mode() & 0o7777)
        .collect();
    for wanted in ["1.0.0", "2.0.0"] {
        let done = run(root.path(), &["--set", wanted]);
        assert_eq!(
            done.status.code(),
            Some(0),
            "{}{}",
            text(&done.stdout),
            text(&done.stderr)
        );
        assert_eq!(
            fs::read_to_string(root.join("VERSION")).unwrap(),
            format!("{wanted}\n")
        );
        for (folder, mods) in [
            ("mods", ["AcmeCore", "AcmeMarket"]),
            ("tests", ["AcmeCoreTests", "AcmeMarketTests"]),
        ] {
            for name in mods {
                assert_eq!(
                    fs::read_to_string(root.join(folder).join(name).join("mod.cpp")).unwrap(),
                    mod_cpp(name, wanted)
                );
                assert_eq!(
                    fs::read_to_string(root.join(folder).join(name).join("config.cpp")).unwrap(),
                    mod_config(name, wanted)
                );
            }
        }
    }
    let after: Vec<u32> = paths
        .iter()
        .map(|path| fs::metadata(root.join(path)).unwrap().permissions().mode() & 0o7777)
        .collect();
    assert_eq!(after, modes);
}

#[test]
fn anything_but_a_family_release_is_refused() {
    let cases = [
        ("1.0.0", "0.2.0"),
        ("0.1.0", "1.1.0"),
        ("0.1.0", "1.0.1"),
        ("1.0.0", "1.0.0"),
        ("1.0.0", "1.0.1"),
        ("2.0.0", "1.0.0"),
        ("0.1.0", "0.2"),
        ("0.1.0", "banana"),
    ];
    for (family, wanted) in cases {
        let root = TempDir::new("sync-refused");
        make_tree(
            &root,
            Some(family),
            &[("AcmeCore", family)],
            &[("AcmeCoreTests", family)],
            &[],
        );
        let done = run(root.path(), &["--set", wanted]);
        assert_eq!(
            done.status.code(),
            Some(1),
            "{family} -> {wanted}: {}{}",
            text(&done.stdout),
            text(&done.stderr)
        );
        assert!(
            text(&done.stdout).contains("refused"),
            "{family} -> {wanted}: {}",
            text(&done.stdout)
        );
    }
}

#[test]
fn one_mod_and_its_test_mod_move() {
    let root = TempDir::new("sync-bump");
    make_tree(
        &root,
        Some("1.0.0"),
        &[("AcmeCore", "1.0.0"), ("AcmeMarket", "1.2.3")],
        &[("AcmeCoreTests", "1.0.0"), ("AcmeMarketTests", "1.2.3")],
        &[],
    );
    let done = run(root.path(), &["--bump", "AcmeMarket", "minor"]);
    assert_eq!(
        done.status.code(),
        Some(0),
        "{}{}",
        text(&done.stdout),
        text(&done.stderr)
    );
    assert_eq!(version_of(&root, "mods/AcmeMarket/mod.cpp"), "1.3.0");
    assert_eq!(version_of(&root, "mods/AcmeMarket/config.cpp"), "1.3.0");
    assert_eq!(version_of(&root, "tests/AcmeMarketTests/mod.cpp"), "1.3.0");
    assert_eq!(
        version_of(&root, "tests/AcmeMarketTests/config.cpp"),
        "1.3.0"
    );
    assert_eq!(version_of(&root, "mods/AcmeCore/mod.cpp"), "1.0.0");
    assert_eq!(version_of(&root, "mods/AcmeCore/config.cpp"), "1.0.0");
    assert_eq!(version_of(&root, "tests/AcmeCoreTests/mod.cpp"), "1.0.0");
    assert_eq!(fs::read_to_string(root.join("VERSION")).unwrap(), "1.0.0\n");

    let done = run(root.path(), &["--bump", "AcmeMarket", "patch"]);
    assert_eq!(
        done.status.code(),
        Some(0),
        "{}{}",
        text(&done.stdout),
        text(&done.stderr)
    );
    assert_eq!(version_of(&root, "mods/AcmeMarket/mod.cpp"), "1.3.1");
    assert_eq!(version_of(&root, "mods/AcmeMarket/config.cpp"), "1.3.1");
    assert_eq!(version_of(&root, "tests/AcmeMarketTests/mod.cpp"), "1.3.1");
    assert_eq!(
        version_of(&root, "tests/AcmeMarketTests/config.cpp"),
        "1.3.1"
    );

    let done = run(root.path(), &["--bump", "AcmeMarket", "minor"]);
    assert_eq!(
        done.status.code(),
        Some(0),
        "{}{}",
        text(&done.stdout),
        text(&done.stderr)
    );
    assert_eq!(version_of(&root, "mods/AcmeMarket/mod.cpp"), "1.4.0");
}

fn version_of(root: &TempDir, rel: &str) -> String {
    let body = fs::read_to_string(root.join(rel)).unwrap();
    body.lines()
        .map(str::trim)
        .find_map(|line| {
            line.strip_prefix("version = \"")
                .map(|rest| rest[..rest.len() - 2].to_string())
        })
        .unwrap_or_default()
}

#[test]
fn bump_is_refused_before_the_first_release() {
    let root = TempDir::new("sync-bump-zero");
    make_tree(
        &root,
        Some("0.1.0"),
        &[("AcmeCore", "0.1.0")],
        &[("AcmeCoreTests", "0.1.0")],
        &[],
    );
    let done = run(root.path(), &["--bump", "AcmeCore", "patch"]);
    assert_eq!(
        done.status.code(),
        Some(1),
        "{}{}",
        text(&done.stdout),
        text(&done.stderr)
    );
    assert!(
        text(&done.stdout).contains("refused"),
        "{}",
        text(&done.stdout)
    );
    assert_eq!(version_of(&root, "mods/AcmeCore/mod.cpp"), "0.1.0");
}

#[test]
fn an_unknown_mod_or_a_major_bump_is_turned_away() {
    let root = TempDir::new("sync-bump-unknown");
    make_tree(
        &root,
        Some("1.0.0"),
        &[("AcmeCore", "1.0.0")],
        &[("AcmeCoreTests", "1.0.0")],
        &[],
    );
    let done = run(root.path(), &["--bump", "AcmeMarket", "minor"]);
    assert_eq!(
        done.status.code(),
        Some(2),
        "{}{}",
        text(&done.stdout),
        text(&done.stderr)
    );
    assert!(
        text(&done.stdout).contains("cannot run: no mods/AcmeMarket"),
        "{}",
        text(&done.stdout)
    );
    let done = run(root.path(), &["--bump", "AcmeCore", "major"]);
    assert_eq!(
        done.status.code(),
        Some(1),
        "{}{}",
        text(&done.stdout),
        text(&done.stderr)
    );
    assert!(
        text(&done.stdout).contains("refused"),
        "{}",
        text(&done.stdout)
    );
    assert_eq!(version_of(&root, "mods/AcmeCore/mod.cpp"), "1.0.0");
}

#[test]
fn a_family_already_failing_is_refused() {
    for (family, module) in [("1.0.0", "2.0.0"), ("1.2.0", "1.0.0")] {
        let root = TempDir::new("sync-bump-drift");
        make_tree(&root, Some(family), &[("AcmeCore", module)], &[], &[]);
        let done = run(root.path(), &["--bump", "AcmeCore", "patch"]);
        assert_eq!(
            done.status.code(),
            Some(1),
            "{family}/{module}: {}{}",
            text(&done.stdout),
            text(&done.stderr)
        );
        assert!(
            text(&done.stdout).contains("refused"),
            "{family}/{module}: {}",
            text(&done.stdout)
        );
        assert_eq!(version_of(&root, "mods/AcmeCore/mod.cpp"), module);
    }
}

#[test]
fn one_bad_file_changes_nothing() {
    let root = TempDir::new("sync-one-bad");
    make_tree(
        &root,
        Some("0.1.0"),
        &[("AcmeCore", "0.1.0"), ("AcmeMarket", "0.1.0")],
        &[("AcmeCoreTests", "0.1.0")],
        &[],
    );
    root.write("mods/AcmeBroken/mod.cpp", "name = \"AcmeBroken\";\n");
    let done = run(root.path(), &["--set", "1.0.0"]);
    assert_eq!(
        done.status.code(),
        Some(2),
        "{}{}",
        text(&done.stdout),
        text(&done.stderr)
    );
    assert!(
        text(&done.stdout).contains("has no version line"),
        "{}",
        text(&done.stdout)
    );
    assert_eq!(fs::read_to_string(root.join("VERSION")).unwrap(), "0.1.0\n");
    assert_eq!(
        fs::read_to_string(root.join("mods/AcmeCore/mod.cpp")).unwrap(),
        mod_cpp("AcmeCore", "0.1.0")
    );
    assert_eq!(
        fs::read_to_string(root.join("mods/AcmeCore/config.cpp")).unwrap(),
        mod_config("AcmeCore", "0.1.0")
    );
    let mut names: Vec<String> = fs::read_dir(root.join("mods/AcmeCore"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, ["config.cpp", "mod.cpp"]);
}

#[test]
fn a_write_that_fails_leaves_every_file_alone() {
    use std::os::unix::fs::PermissionsExt;
    if common::running_as_root() {
        return;
    }
    let root = TempDir::new("sync-write-fail");
    make_tree(
        &root,
        Some("0.1.0"),
        &[("AcmeCore", "0.1.0"), ("AcmeMarket", "0.1.0")],
        &[("AcmeCoreTests", "0.1.0")],
        &[],
    );
    let files = [
        "VERSION",
        "mods/AcmeCore/mod.cpp",
        "mods/AcmeCore/config.cpp",
        "mods/AcmeMarket/mod.cpp",
        "mods/AcmeMarket/config.cpp",
        "tests/AcmeCoreTests/mod.cpp",
        "tests/AcmeCoreTests/config.cpp",
    ];
    let before: Vec<String> = files
        .iter()
        .map(|rel| fs::read_to_string(root.join(rel)).unwrap())
        .collect();
    let folder = root.join("mods/AcmeMarket");
    fs::set_permissions(folder.join("mod.cpp"), fs::Permissions::from_mode(0o444)).unwrap();
    fs::set_permissions(&folder, fs::Permissions::from_mode(0o555)).unwrap();
    let done = run(root.path(), &["--set", "1.0.0"]);
    fs::set_permissions(&folder, fs::Permissions::from_mode(0o700)).unwrap();
    fs::set_permissions(folder.join("mod.cpp"), fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(
        done.status.code(),
        Some(2),
        "{}{}",
        text(&done.stdout),
        text(&done.stderr)
    );
    assert!(
        text(&done.stdout).contains("cannot write"),
        "{}",
        text(&done.stdout)
    );
    assert_eq!(text(&done.stderr), "");
    for (rel, body) in files.iter().zip(&before) {
        assert_eq!(&fs::read_to_string(root.join(rel)).unwrap(), body, "{rel}");
    }
    let leftovers = fs::read_dir(&folder)
        .unwrap()
        .filter(|entry| {
            entry
                .as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".sync_version-")
        })
        .count();
    assert_eq!(leftovers, 0);
    let retry = run(root.path(), &["--set", "1.0.0"]);
    assert_eq!(
        retry.status.code(),
        Some(0),
        "{}{}",
        text(&retry.stdout),
        text(&retry.stderr)
    );
}

#[test]
fn a_windows_line_ending_is_kept() {
    let root = TempDir::new("sync-crlf");
    let body = "name = \"Acme Groups\";\r\nversion = \"0.1.0\";  // keep in sync\r\n";
    make_tree(
        &root,
        Some("0.1.0"),
        &[],
        &[],
        &[("mods/AcmeGroups/mod.cpp", body.to_string())],
    );
    let done = run(root.path(), &["--set", "1.0.0"]);
    assert_eq!(
        done.status.code(),
        Some(0),
        "{}{}",
        text(&done.stdout),
        text(&done.stderr)
    );
    assert_eq!(
        fs::read_to_string(root.join("mods/AcmeGroups/mod.cpp")).unwrap(),
        body.replace("\"0.1.0\"", "\"1.0.0\"")
    );
}

#[test]
fn only_the_version_line_changes() {
    let root = TempDir::new("sync-only-version");
    let body = "name = \"Acme Groups\";\n/*\nversion = \"0.0.9\";\n*/\noverview = \"Groups, with a version note\";\nversion = \"0.1.0\";\n";
    make_tree(
        &root,
        Some("0.1.0"),
        &[],
        &[],
        &[("mods/AcmeGroups/mod.cpp", body.to_string())],
    );
    let done = run(root.path(), &["--set", "1.0.0"]);
    assert_eq!(
        done.status.code(),
        Some(0),
        "{}{}",
        text(&done.stdout),
        text(&done.stderr)
    );
    assert_eq!(
        fs::read_to_string(root.join("mods/AcmeGroups/mod.cpp")).unwrap(),
        body.replace("\"0.1.0\"", "\"1.0.0\"")
    );
}

#[test]
fn a_version_line_spelled_with_long_s_and_dotless_i_counts() {
    let root = TempDir::new("sync-folded-version");
    make_tree(
        &root,
        Some("0.1.0"),
        &[],
        &[],
        &[(
            "mods/AcmeCore/mod.cpp",
            "version = \"0.1.0\";\nver\u{17f}\u{131}on = \"0.1.0\";\n".to_string(),
        )],
    );
    let done = run(root.path(), &[]);
    assert_eq!(
        done.status.code(),
        Some(2),
        "{}{}",
        text(&done.stdout),
        text(&done.stderr)
    );
    assert!(
        text(&done.stdout).contains("has 2 version lines"),
        "{}",
        text(&done.stdout)
    );
}
