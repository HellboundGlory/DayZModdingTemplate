mod common;

use common::{
    checks, code, fixture, stderr, stdout, write_family, Scratch, ACME_FAMILY,
    ACME_FAMILY_TWO_FRAMEWORKS,
};

fn check(case: &str) -> std::process::Output {
    let scratch = Scratch::copy_of(
        "dependencies-case",
        &fixture("check_dependencies").join(case),
    );
    write_family(&scratch, ACME_FAMILY_TWO_FRAMEWORKS);
    checks(&[
        "check-dependencies",
        "--root",
        scratch.path().to_str().unwrap(),
    ])
}

#[test]
fn outcomes() {
    let cases = [
        ("clean tree passes", "clean", 0, "OK"),
        ("unknown addon fails", "unknown", 1, "SomeOtherMod"),
        ("sibling addon fails", "sibling", 1, "AcmeMarket"),
        (
            "sibling merged with append fails",
            "append",
            1,
            "AcmeMarket",
        ),
        (
            "feature mod without Core fails",
            "missing_core",
            1,
            "does not list AcmeCore",
        ),
        (
            "test mod without its mod fails",
            "missing_target",
            1,
            "does not list AcmeGroups",
        ),
        (
            "test mod for an absent mod fails",
            "orphan_target",
            1,
            "which is not in mods/",
        ),
        (
            "test mod with a sibling fails",
            "test_sibling",
            1,
            "AcmeMarket",
        ),
        (
            "Core without the Kit addon fails",
            "core_missing_kit",
            1,
            "KIT_Scripts",
        ),
        (
            "Core without a Forge patch fails",
            "core_missing_forge",
            1,
            "FORGE_GUI",
        ),
        (
            "Core naming a feature mod fails",
            "core_names_mod",
            1,
            "AcmeGroups",
        ),
        ("base addons and DZ wildcard pass", "base_allow", 0, "OK"),
        ("comments do not count", "comments", 0, "OK"),
        (
            "commented requiredAddons do not count",
            "commented_addon",
            0,
            "OK",
        ),
        (
            "a class name inside a string is ignored",
            "string_class",
            0,
            "OK",
        ),
        (
            "nested requiredAddons do not satisfy a patch class",
            "addons_nested",
            2,
            "requiredAddons",
        ),
        ("backslash in a string is literal", "backslash", 0, "OK"),
        (
            "unquoted requiredAddons cannot run",
            "unquoted",
            2,
            "not a quoted string",
        ),
        (
            "patch class without requiredAddons cannot run",
            "patch_missing",
            2,
            "requiredAddons",
        ),
        (
            "patch class naming another mod fails",
            "patch_foreign",
            1,
            "AcmeMarket",
        ),
        (
            "nested class inside a patch class passes",
            "patch_nested",
            0,
            "OK",
        ),
        (
            "test folder not ending in Tests fails",
            "test_folder",
            1,
            "does not end in Tests",
        ),
        ("a clean nested config passes", "nested_clean", 0, "OK"),
        (
            "a nested config naming another mod's patch fails",
            "nested_foreign",
            1,
            "AcmeMarket",
        ),
        (
            "a nested config with a sibling addon fails",
            "nested_sibling",
            1,
            "lists sibling AcmeMarket",
        ),
        (
            "a nested config without CfgPatches cannot run",
            "nested_no_patches",
            2,
            "no CfgPatches",
        ),
        (
            "a root patch not named after the mod fails",
            "patch_wrong_name",
            1,
            "has no class AcmeGroups",
        ),
        ("config keywords ignore case", "case", 0, "OK"),
        (
            "a lower-case foreign patch class fails",
            "case_foreign",
            1,
            "belongs to AcmeMarket",
        ),
        (
            "missing requiredAddons cannot run",
            "unparsable",
            2,
            "requiredAddons",
        ),
        (
            "an empty mods folder is skipped",
            "empty",
            0,
            "skipped: no mods under",
        ),
        ("bare root cannot run", "bare", 2, "no mods/ or tests/"),
    ];
    for (name, case, expected, needle) in cases {
        let out = check(case);
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
    let out = check("append");
    let config = std::path::Path::new("mods/AcmeGroups/config.cpp")
        .display()
        .to_string();
    assert!(
        stdout(&out).contains(&format!("{config}:6:")),
        "{}",
        stdout(&out)
    );
    let out = check("sibling");
    assert!(
        stdout(&out).contains("a mod requires only Kit, Forge and AcmeCore"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn a_server_link_at_the_mod_root_is_not_read() {
    let scratch = Scratch::new("dependencies-server");
    write_family(&scratch, ACME_FAMILY_TWO_FRAMEWORKS);
    common::write_file(&scratch.join("server/config.cpp"), "class CfgMods\n{\n};\n");
    common::write_file(
        &scratch.join("mods/AcmeCore/config.cpp"),
        "class CfgPatches\n{\n\tclass AcmeCore\n\t{\n\t\trequiredAddons[] = {\"KIT_Scripts\", \"FORGE_GUI\"};\n\t};\n};\n",
    );
    std::os::unix::fs::symlink(scratch.join("server"), scratch.join("mods/AcmeCore/server"))
        .unwrap();
    let out = checks(&[
        "check-dependencies",
        "--root",
        scratch.path().to_str().unwrap(),
    ]);
    assert_eq!(code(&out), Some(0), "{}{}", stdout(&out), stderr(&out));
    assert!(stdout(&out).contains("OK: 1 config"), "{}", stdout(&out));
}

#[test]
fn an_unreadable_folder_cannot_run() {
    use std::os::unix::fs::PermissionsExt;
    let scratch = Scratch::new("dependencies-unreadable");
    write_family(&scratch, ACME_FAMILY_TWO_FRAMEWORKS);
    common::write_file(
        &scratch.join("mods/AcmeCore/config.cpp"),
        "class CfgPatches\n{\n\tclass AcmeCore\n\t{\n\t\trequiredAddons[] = {};\n\t};\n};\n",
    );
    let protected = scratch.join("mods");
    std::fs::set_permissions(&protected, std::fs::Permissions::from_mode(0o0)).unwrap();
    let out = checks(&[
        "check-dependencies",
        "--root",
        scratch.path().to_str().unwrap(),
    ]);
    std::fs::set_permissions(&protected, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(code(&out), Some(2), "{}{}", stdout(&out), stderr(&out));
    assert!(stdout(&out).contains("cannot read"), "{}", stdout(&out));
}

#[test]
fn a_separator_control_character_is_whitespace_in_a_patch_class() {
    let scratch = Scratch::new("dependencies-separator");
    write_family(&scratch, ACME_FAMILY_TWO_FRAMEWORKS);
    scratch.write(
        "mods/AcmeCore/config.cpp",
        "class CfgPatches\n{\n    class AcmeCore\n    {\n        requiredAddons[] = {\"KIT_Scripts\",\"FORGE_GUI\"};\n    };\n};\n",
    );
    scratch.write(
        "mods/AcmeY/config.cpp",
        "class CfgPatches\n{\n    class AcmeY\n    {\n        requiredAddons[] = {\"AcmeCore\"};\n    };\n    class\u{1d}AcmeCore\n    {\n        requiredAddons[] = {\"AcmeCore\"};\n    };\n};\n",
    );
    let out = checks(&[
        "check-dependencies",
        "--root",
        scratch.path().to_str().unwrap(),
    ]);
    assert_eq!(code(&out), Some(1), "{}{}", stdout(&out), stderr(&out));
    assert!(
        stdout(&out).contains("patch class AcmeCore belongs to AcmeCore"),
        "{}",
        stdout(&out)
    );
}

// Each entry is a mod folder and the addons its CfgPatches class lists.
fn write_tree(scratch: &Scratch, mods: &[(&str, &[&str])]) {
    for (folder, addons) in mods {
        let class = folder.rsplit('/').next().unwrap();
        let list: Vec<String> = addons.iter().map(|addon| format!("\"{addon}\"")).collect();
        scratch.write(
            &format!("{folder}/config.cpp"),
            &format!(
                "class CfgPatches\n{{\n\tclass {class}\n\t{{\n\t\trequiredAddons[] = {{{}}};\n\t}};\n}};\n",
                list.join(", ")
            ),
        );
    }
}

// Each mod's folder and the addons its config requires.
type Mods = &'static [(&'static str, &'static [&'static str])];

#[test]
fn names_come_from_the_family_file() {
    let cases: [(&str, &str, Mods, i32, &str); 11] = [
        (
            "core lists its framework addon",
            ACME_FAMILY,
            &[
                ("mods/AcmeCore", &["KIT_Scripts"]),
                ("mods/AcmeMarket", &["AcmeCore", "KIT_GUI"]),
            ],
            0,
            "OK",
        ),
        (
            "core lists one of the framework's addons",
            ACME_FAMILY,
            &[
                ("mods/AcmeCore", &["KIT_GUI"]),
                ("mods/AcmeMarket", &["AcmeCore"]),
            ],
            0,
            "OK",
        ),
        (
            "core without its framework fails",
            ACME_FAMILY,
            &[("mods/AcmeCore", &[])],
            1,
            "AcmeCore lists neither KIT_Scripts nor KIT_GUI: a mod requires only Kit and AcmeCore",
        ),
        (
            "core naming outside addons fails",
            ACME_FAMILY,
            &[("mods/AcmeCore", &["OUTSIDE_Scripts", "OUTSIDE_GUI"])],
            1,
            "AcmeCore lists neither KIT_Scripts nor KIT_GUI",
        ),
        (
            "feature mod without the core fails",
            ACME_FAMILY,
            &[
                ("mods/AcmeCore", &["KIT_Scripts"]),
                ("mods/AcmeMarket", &["KIT_GUI"]),
            ],
            1,
            "AcmeMarket does not list AcmeCore",
        ),
        (
            "sibling addon fails",
            ACME_FAMILY,
            &[
                ("mods/AcmeCore", &["KIT_Scripts"]),
                ("mods/AcmeMarket", &["AcmeCore"]),
                ("mods/AcmeGroups", &["AcmeCore", "AcmeMarket"]),
            ],
            1,
            "lists sibling AcmeMarket: a mod requires only Kit and AcmeCore",
        ),
        (
            "outside core name is an unknown addon",
            ACME_FAMILY,
            &[
                ("mods/AcmeCore", &["KIT_Scripts"]),
                ("mods/AcmeMarket", &["AcmeCore", "OutsideCore"]),
            ],
            1,
            "lists unknown addon OutsideCore",
        ),
        (
            "Forge addon is an unknown addon",
            ACME_FAMILY,
            &[
                ("mods/AcmeCore", &["KIT_Scripts"]),
                ("mods/AcmeMarket", &["AcmeCore", "FORGE_GUI"]),
            ],
            1,
            "lists unknown addon FORGE_GUI",
        ),
        (
            "three framework addons join with commas",
            "prefix = Acme\ncore = AcmeCore\nframework = Kit KIT_Scripts KIT_GUI KIT_Net\n",
            &[("mods/AcmeCore", &[])],
            1,
            "AcmeCore lists neither KIT_Scripts, KIT_GUI nor KIT_Net",
        ),
        (
            "test mod with its target passes",
            ACME_FAMILY,
            &[
                ("mods/AcmeCore", &["KIT_Scripts"]),
                ("mods/AcmeMarket", &["AcmeCore"]),
                ("tests/AcmeMarketTests", &["AcmeMarket"]),
            ],
            0,
            "OK",
        ),
        (
            "test mod without its target fails",
            ACME_FAMILY,
            &[
                ("mods/AcmeCore", &["KIT_Scripts"]),
                ("mods/AcmeMarket", &["AcmeCore"]),
                ("tests/AcmeMarketTests", &["AcmeCore"]),
            ],
            1,
            "AcmeMarketTests does not list AcmeMarket",
        ),
    ];
    for (name, conf, mods, expected, needle) in cases {
        let scratch = Scratch::new("dependencies-family");
        write_family(&scratch, conf);
        write_tree(&scratch, mods);
        let out = checks(&[
            "check-dependencies",
            "--root",
            scratch.path().to_str().unwrap(),
        ]);
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
