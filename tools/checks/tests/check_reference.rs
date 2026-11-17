mod common;

use common::{code, fixture, stderr, stdout, Scratch};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

// Runs the check on a root that has a family.conf, with the references given as folders inside it.
fn run_root(root: &Path, reference: Option<&str>, vanilla: Option<&str>) -> Output {
    let mut args = vec![
        "check-reference".to_string(),
        "--root".to_string(),
        root.display().to_string(),
    ];
    if let Some(folder) = reference {
        args.push("--reference".to_string());
        args.push(root.join(folder).display().to_string());
    }
    if let Some(folder) = vanilla {
        args.push("--vanilla".to_string());
        args.push(root.join(folder).display().to_string());
    }
    Command::new(env!("CARGO_BIN_EXE_checks"))
        .args(&args)
        .output()
        .unwrap()
}

fn check(case: &str, reference: Option<&str>, vanilla: Option<&str>) -> Output {
    let scratch = Scratch::copy_of("reference-case", &fixture("check_reference").join(case));
    common::write_family(&scratch, common::ACME_FAMILY_TWO_FRAMEWORKS);
    run_root(scratch.path(), reference, vanilla)
}

#[test]
fn outcomes() {
    let cases = [
        (
            "clean tree passes",
            "clean",
            Some("reference"),
            Some("vanilla"),
            0,
            "OK",
        ),
        (
            "declared name used fails",
            "declared_used",
            Some("reference"),
            Some("vanilla"),
            1,
            "FixtureUpstreamAlpha",
        ),
        (
            "indented reference class is collected",
            "indented_used",
            Some("reference"),
            Some("vanilla"),
            1,
            "FixtureIndented",
        ),
        (
            "declared name used in a test mod fails",
            "test_used",
            Some("reference"),
            Some("vanilla"),
            1,
            "FixtureUpstreamAlpha",
        ),
        (
            "modded class is skipped",
            "modded_used",
            Some("reference"),
            Some("vanilla"),
            0,
            "OK",
        ),
        (
            "zero reference names cannot run",
            "no_names",
            Some("reference"),
            Some("vanilla"),
            2,
            "no reference names",
        ),
        (
            "backslash in a config string is literal",
            "config_backslash",
            Some("reference"),
            Some("vanilla"),
            0,
            "OK",
        ),
        (
            "name also declared in vanilla passes",
            "vanilla_shared",
            Some("reference"),
            Some("vanilla"),
            0,
            "OK",
        ),
        (
            "name in a layout fails",
            "layout_used",
            Some("reference"),
            Some("vanilla"),
            1,
            "FixtureLayoutName",
        ),
        (
            "name in config.cpp fails",
            "config_used",
            Some("reference"),
            Some("vanilla"),
            1,
            "FixtureConfigName",
        ),
        (
            "longer identifiers do not match",
            "word_boundary",
            Some("reference"),
            Some("vanilla"),
            0,
            "OK",
        ),
        (
            "name only in a comment passes",
            "repo_comment",
            Some("reference"),
            Some("vanilla"),
            0,
            "OK",
        ),
        (
            "the game's own building classes are ignored",
            "game_classes",
            Some("reference"),
            Some("vanilla"),
            0,
            "OK",
        ),
        (
            "missing reference folder cannot run",
            "clean",
            Some("absent"),
            Some("vanilla"),
            2,
            "--reference",
        ),
        (
            "missing vanilla reference cannot run",
            "clean",
            Some("reference"),
            Some("absent"),
            2,
            "--vanilla",
        ),
        (
            "an empty mods folder is skipped",
            "empty",
            Some("reference"),
            Some("vanilla"),
            0,
            "skipped: no mods under",
        ),
    ];
    for (name, case, reference, vanilla, expected, needle) in cases {
        let out = check(case, reference, vanilla);
        assert_eq!(
            code(&out),
            Some(expected),
            "{name}: {}{}",
            stdout(&out),
            stderr(&out)
        );
        assert!(stdout(&out).contains(needle), "{name}: {}", stdout(&out));
    }
}

#[test]
fn a_violation_reports_its_line_and_quotes_the_rule() {
    let out = check("declared_used", Some("reference"), Some("vanilla"));
    let groups = std::path::Path::new("mods/AcmeGroups/scripts/Groups.c")
        .display()
        .to_string();
    assert!(
        stdout(&out).contains(&format!("{groups}:3:")),
        "{}",
        stdout(&out)
    );
    assert!(
        stdout(&out).contains("a reference is never a source"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn an_unreadable_folder_cannot_run() {
    use std::os::unix::fs::PermissionsExt;
    let scratch = Scratch::new("reference-unreadable");
    common::write_family(&scratch, common::ACME_FAMILY_TWO_FRAMEWORKS);
    common::write_file(
        &scratch.join("mods/AcmeCore/Core.c"),
        "class AcmeThing\n{\n};\n",
    );
    let folder = scratch.join("mods/AcmeCore");
    std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o0)).unwrap();
    let fixture_root = fixture("check_reference/clean");
    let out = Command::new(env!("CARGO_BIN_EXE_checks"))
        .args([
            "check-reference",
            "--root",
            scratch.path().to_str().unwrap(),
            "--reference",
            fixture_root.join("reference").to_str().unwrap(),
            "--vanilla",
            fixture_root.join("vanilla").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(code(&out), Some(2), "{}{}", stdout(&out), stderr(&out));
    assert!(stdout(&out).contains("cannot read"), "{}", stdout(&out));
}

#[test]
fn links_are_walked_but_a_server_folder_is_not() {
    let scratch = Scratch::new("reference-links");
    common::write_family(&scratch, common::ACME_FAMILY_TWO_FRAMEWORKS);
    let root = scratch.path().to_path_buf();
    let write = |rel: &str, text: &str| common::write_file(&root.join(rel), text);
    write(
        "reference/reference/A.c",
        "class FixtureLinked\n{\n};\n\nclass FixtureUpper\n{\n};\n\nclass FixtureMap\n{\n};\n\nclass FixtureServer\n{\n};\n",
    );
    write("reference/vanilla/A.c", "class FixtureVanillaBase\n{\n};\n");
    write(
        "linked/Use.C",
        "class Thing\n{\n\tFixtureLinked value;\n};\n",
    );
    write(
        "linked/Upper.C",
        "class Thing\n{\n\tFixtureUpper value;\n};\n",
    );
    write(
        "server/Sneak.c",
        "class Thing\n{\n\tFixtureServer value;\n};\n",
    );
    write(
        "elsewhere/AcmeMap/scripts/Map.c",
        "class Thing\n{\n\tFixtureMap value;\n};\n",
    );
    write(
        "mods/AcmeCore/config.cpp",
        "class CfgPatches\n{\n\tclass AcmeCore\n\t{\n\t\trequiredAddons[] = {};\n\t};\n};\n",
    );
    let link = |target: &str, at: &str| {
        std::os::unix::fs::symlink(root.join(target), root.join(at)).unwrap()
    };
    link("linked", "mods/AcmeCore/scripts");
    link("server", "mods/AcmeCore/server");
    link("elsewhere/AcmeMap", "mods/AcmeMap");
    link("mods/AcmeMap", "linked/alias");
    link("server", "mods/AcmeCore/Server");
    std::os::unix::fs::symlink(root.join("missing.c"), root.join("linked/Dead.c")).unwrap();

    let out = run_root(
        &root,
        Some("reference/reference"),
        Some("reference/vanilla"),
    );
    assert_eq!(code(&out), Some(1), "{}{}", stdout(&out), stderr(&out));
    let text = stdout(&out);
    for needle in [
        "Use.C",
        "Upper.C",
        &Path::new("AcmeMap/scripts/Map.c").display().to_string(),
    ] {
        assert!(text.contains(needle), "{needle}: {text}");
    }
    for absent in ["server", "Server", "Dead.c"] {
        assert!(!text.contains(absent), "{absent}: {text}");
    }
}

#[test]
fn a_link_cycle_or_a_self_link_cannot_run() {
    for (name, at, target, needle) in [
        ("cycle", "loop", "FOLDER", "cycle"),
        ("self", "self", "self", "cannot read"),
    ] {
        let scratch = Scratch::new("reference-cycle");
        common::write_family(&scratch, common::ACME_FAMILY_TWO_FRAMEWORKS);
        let root = scratch.path();
        common::write_file(&root.join("reference/A.c"), "class FixtureAlpha\n{\n};\n");
        common::write_file(&root.join("vanilla/A.c"), "class FixtureVanilla\n{\n};\n");
        let mod_folder = root.join("mods/AcmeCore");
        common::write_file(
            &mod_folder.join("Scripts/Core.c"),
            "class AcmeCoreThing\n{\n};\n",
        );
        let target = if target == "FOLDER" {
            mod_folder.clone()
        } else {
            PathBuf::from(target)
        };
        std::os::unix::fs::symlink(&target, mod_folder.join("Scripts").join(at)).unwrap();
        let out = run_root(root, Some("reference"), Some("vanilla"));
        assert_eq!(
            code(&out),
            Some(2),
            "{name}: {}{}",
            stdout(&out),
            stderr(&out)
        );
        assert!(stdout(&out).contains(needle), "{name}: {}", stdout(&out));
    }
}

#[test]
fn the_real_reference_names_are_collected() {
    // Reads the folders in the repo's own tools/family.conf. About a second, so run with CHECKS_REAL_REFERENCE=1.
    if std::env::var("CHECKS_REAL_REFERENCE").as_deref() != Ok("1") {
        return;
    }
    let conf = std::fs::read_to_string(common::repo().join("tools/family.conf"))
        .expect("CHECKS_REAL_REFERENCE needs tools/family.conf: run setup.sh first");
    let core = conf
        .lines()
        .find_map(|line| line.trim().strip_prefix("core"))
        .and_then(|rest| rest.trim_start().strip_prefix('='))
        .map(str::trim)
        .expect("tools/family.conf has no core line");
    let scratch = Scratch::new("reference-real");
    common::write_family(&scratch, &conf);
    common::write_file(
        &scratch.join(&format!("mods/{core}/config.cpp")),
        &format!("class CfgPatches\n{{\n\tclass {core}\n\t{{\n\t\trequiredAddons[] = {{}};\n\t}};\n}};\n"),
    );
    let out = Command::new(env!("CARGO_BIN_EXE_checks"))
        .args([
            "check-reference",
            "--root",
            scratch.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(code(&out), Some(0), "{}{}", stdout(&out), stderr(&out));
    assert!(stdout(&out).contains("reference names"), "{}", stdout(&out));
}

#[test]
fn a_separator_control_character_is_whitespace_in_a_reference_class() {
    let scratch = Scratch::new("reference-separator");
    common::write_family(&scratch, common::ACME_FAMILY_TWO_FRAMEWORKS);
    scratch.write(
        "ref/E.c",
        "class UpstreamNormal {}\nclass\u{1d}UpstreamHidden {}\n",
    );
    scratch.write("van/V.c", "class UpstreamOther {}\n");
    scratch.write(
        "mods/AcmeY/Scripts/3_Game/A.c",
        "class Foo { UpstreamHidden x; }\n",
    );
    let out = run_root(scratch.path(), Some("ref"), Some("van"));
    assert_eq!(code(&out), Some(1), "{}{}", stdout(&out), stderr(&out));
    assert!(
        stdout(&out).contains("uses reference name UpstreamHidden"),
        "{}",
        stdout(&out)
    );
}

// A made-up family whose reference and vanilla folders sit in the scratch root.
fn acme_tree(conf: &str) -> Scratch {
    let scratch = Scratch::new("reference-acme");
    scratch.write("tools/family.conf", conf);
    scratch.write("ref/A.c", "class RefAlpha\n{\n};\n");
    scratch.write("van/V.c", "class VanillaThing\n{\n};\n");
    scratch.write(
        "mods/AcmeCore/config.cpp",
        "class CfgPatches\n{\n\tclass AcmeCore\n\t{\n\t\trequiredAddons[] = {};\n\t};\n};\n",
    );
    scratch.write("mods/AcmeCore/Scripts/Core.c", "class AcmeThing\n{\n};\n");
    scratch
}

#[test]
fn the_family_supplies_the_reference_folders() {
    let scratch = acme_tree(&format!(
        "{}reference = ref\nvanilla = van\n",
        common::ACME_FAMILY
    ));
    let out = run_root(scratch.path(), None, None);
    assert_eq!(code(&out), Some(0), "{}{}", stdout(&out), stderr(&out));
    assert!(
        stdout(&out).contains("OK: 1 reference names"),
        "{}",
        stdout(&out)
    );

    scratch.write(
        "mods/AcmeCore/Scripts/Use.c",
        "class AcmeUse\n{\n\tRefAlpha value;\n};\n",
    );
    let out = run_root(scratch.path(), None, None);
    assert_eq!(code(&out), Some(1), "{}{}", stdout(&out), stderr(&out));
    let file = Path::new("mods/AcmeCore/Scripts/Use.c")
        .display()
        .to_string();
    assert!(
        stdout(&out).contains(&format!("{file}:3: uses reference name RefAlpha")),
        "{}",
        stdout(&out)
    );

    let scratch = acme_tree(&format!("{}reference = ref\n", common::ACME_FAMILY));
    let out = run_root(scratch.path(), None, None);
    assert_eq!(code(&out), Some(2), "{}{}", stdout(&out), stderr(&out));
    assert!(
        stdout(&out)
            .contains("--vanilla is missing: pass --vanilla or set vanilla in tools/family.conf"),
        "{}",
        stdout(&out)
    );

    let scratch = acme_tree(&format!(
        "{}reference = ref\nvanilla = absent\n",
        common::ACME_FAMILY
    ));
    let out = run_root(scratch.path(), None, None);
    assert_eq!(code(&out), Some(2), "{}{}", stdout(&out), stderr(&out));
    assert!(
        stdout(&out).contains("--vanilla folder not found")
            && stdout(&out).contains("set vanilla in tools/family.conf"),
        "{}",
        stdout(&out)
    );
    let out = run_root(scratch.path(), None, Some("van"));
    assert_eq!(code(&out), Some(0), "{}{}", stdout(&out), stderr(&out));

    let scratch = Scratch::new("reference-no-conf");
    scratch.write("mods/AcmeCore/Scripts/Core.c", "class AcmeThing\n{\n};\n");
    let out = run_root(scratch.path(), Some("ref"), Some("van"));
    assert_eq!(code(&out), Some(2), "{}{}", stdout(&out), stderr(&out));
    assert!(
        stdout(&out).contains("cannot run: tools/family.conf not found"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn a_flag_overrides_the_family_folder() {
    let scratch = acme_tree(&format!(
        "{}reference = absent\nvanilla = van\n",
        common::ACME_FAMILY
    ));
    let out = run_root(scratch.path(), Some("ref"), None);
    assert_eq!(code(&out), Some(0), "{}{}", stdout(&out), stderr(&out));
}

#[test]
fn a_root_without_a_mods_folder_cannot_run() {
    let scratch = Scratch::new("reference-no-mods-folder");
    common::write_family(&scratch, common::ACME_FAMILY_TWO_FRAMEWORKS);
    let out = run_root(scratch.path(), Some("ref"), Some("van"));
    assert_eq!(code(&out), Some(2), "{}{}", stdout(&out), stderr(&out));
    assert!(stdout(&out).contains("no mods under"), "{}", stdout(&out));
}

#[test]
fn no_reference_folder_skips_the_check() {
    let scratch = acme_tree(common::ACME_FAMILY);
    let out = run_root(scratch.path(), None, None);
    assert_eq!(code(&out), Some(0), "{}{}", stdout(&out), stderr(&out));
    assert!(
        stdout(&out).starts_with("skipped: no reference folder"),
        "{}",
        stdout(&out)
    );
    let out = Command::new(env!("CARGO_BIN_EXE_checks"))
        .args([
            "check-reference",
            "--root",
            scratch.path().to_str().unwrap(),
            "--reference",
            "",
        ])
        .output()
        .unwrap();
    assert_eq!(code(&out), Some(0), "{}{}", stdout(&out), stderr(&out));
}

#[test]
fn a_missing_mods_folder_is_reported_before_the_reference_skip() {
    let no_mods = Scratch::new("reference-no-mods-no-reference");
    common::write_family(&no_mods, common::ACME_FAMILY);
    let out = run_root(no_mods.path(), None, None);
    assert_eq!(code(&out), Some(2), "{}{}", stdout(&out), stderr(&out));
    assert!(stdout(&out).contains("no mods under"), "{}", stdout(&out));
}

#[test]
fn a_vanilla_flag_without_a_reference_is_not_skipped() {
    let tree = acme_tree(common::ACME_FAMILY);
    let out = run_root(tree.path(), None, Some("van"));
    assert_eq!(code(&out), Some(2), "{}{}", stdout(&out), stderr(&out));
    assert!(stdout(&out).contains("--reference"), "{}", stdout(&out));
}
