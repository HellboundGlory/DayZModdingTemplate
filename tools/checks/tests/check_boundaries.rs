mod common;

use common::{checks, code, fixture, stderr, stdout, ACME_FAMILY};

fn check(case: &str) -> std::process::Output {
    let scratch =
        common::Scratch::copy_of("boundaries-case", &fixture("check_boundaries").join(case));
    common::write_family(&scratch, common::ACME_FAMILY_TWO_FRAMEWORKS);
    checks(&[
        "check-boundaries",
        "--root",
        scratch.path().to_str().unwrap(),
    ])
}

#[test]
fn outcomes() {
    let cases = [
        ("own prefixes and Core pass", "clean", 0, "OK"),
        (
            "sibling name in code fails",
            "sibling_code",
            1,
            "AcmeMarket",
        ),
        (
            "sibling name in a string fails",
            "sibling_string",
            1,
            "AcmeMarket",
        ),
        ("sibling name in a comment passes", "comment_only", 0, "OK"),
        ("longest prefix wins", "longest_prefix", 1, "AcmeCoreTests"),
        ("contract identifier is allowed", "allowlisted", 0, "OK"),
        (
            "only backticked identifiers are allowed",
            "allowlist_exact",
            1,
            "AcmeMarketEntry",
        ),
        (
            "test mod naming a sibling fails",
            "test_sibling",
            1,
            "AcmeMarket",
        ),
        (
            "Core naming a feature mod fails",
            "core_sibling",
            1,
            "AcmeMarket",
        ),
        (
            "sibling name in a layout fails",
            "layout_sibling",
            1,
            "AcmeMarket",
        ),
        (
            "sibling name in config.cpp fails",
            "config_sibling",
            1,
            "AcmeMarket",
        ),
        (
            "a comment after a backslash string is stripped",
            "config_comment",
            0,
            "OK",
        ),
        (
            "upper-case key belonging to a sibling fails",
            "upper_string",
            1,
            "ACME_MARKET",
        ),
        ("ACME_CORE is allowed like AcmeCore", "upper_core", 0, "OK"),
        (
            "a token preceded by a letter is not a mod id",
            "upper_boundary",
            0,
            "OK",
        ),
        (
            "an upper-case key followed by a letter belongs to nobody",
            "upper_key_boundary",
            0,
            "OK",
        ),
        (
            "a multi-word id is one token",
            "upper_multiword",
            1,
            "AcmeMapAssets",
        ),
        (
            "a folder under tests that does not end in Tests is not an owner",
            "owner_scope",
            0,
            "OK",
        ),
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
    let out = check("empty");
    assert_eq!(code(&out), Some(0), "{}{}", stdout(&out), stderr(&out));
    assert!(
        stdout(&out).starts_with("skipped: no mods under"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn a_root_without_a_mods_folder_cannot_run() {
    let scratch = common::Scratch::new("boundaries-no-mods-folder");
    common::write_family(&scratch, common::ACME_FAMILY_TWO_FRAMEWORKS);
    let out = checks(&[
        "check-boundaries",
        "--root",
        scratch.path().to_str().unwrap(),
    ]);
    assert_eq!(code(&out), Some(2), "{}{}", stdout(&out), stderr(&out));
    assert!(stdout(&out).contains("no mods under"), "{}", stdout(&out));
}

fn acme_tree(family: &str, market: &str, contracts: &str) -> std::process::Output {
    let scratch = common::Scratch::new("boundaries-acme");
    common::write_family(&scratch, family);
    scratch.write("mods/AcmeCore/docs/CONTRACTS.md", contracts);
    scratch.write(
        "mods/AcmeCore/scripts/Core.c",
        "class AcmeCoreBase\n{\n};\n",
    );
    scratch.write(
        "mods/AcmeGroups/scripts/Groups.c",
        "class AcmeGroupsThing\n{\n};\n",
    );
    scratch.write(
        "mods/OutsideGroups/scripts/Groups.c",
        "class OutsideGroupsThing\n{\n};\n",
    );
    scratch.write("mods/AcmeMarket/scripts/Market.c", market);
    checks(&[
        "check-boundaries",
        "--root",
        scratch.path().to_str().unwrap(),
    ])
}

#[test]
fn a_family_under_other_names_is_checked_with_its_own_prefix() {
    let two_frameworks = "prefix = Acme\ncore = AcmeCore\nframework = Kit KIT_Scripts\nframework = Other OTHER_Scripts\n";
    let no_framework = "prefix = Acme\ncore = AcmeCore\n";
    let sibling = "class AcmeMarketEntry\n{\n\tAcmeGroupsThing sibling;\n};\n";
    let cases = [
        (
            "sibling name in code fails",
            ACME_FAMILY,
            sibling,
            1,
            "AcmeGroupsThing belongs to AcmeGroups: a mod requires only Kit and AcmeCore",
        ),
        (
            "Outside name is not the family's",
            ACME_FAMILY,
            "class AcmeMarketEntry\n{\n\tOutsideGroupsThing sibling;\n};\n",
            0,
            "OK",
        ),
        (
            "upper-case sibling key fails",
            ACME_FAMILY,
            "class AcmeMarketEntry\n{\n\tconst string Key = \"ACME_GROUPS_KEY\";\n};\n",
            1,
            "ACME_GROUPS_KEY belongs to AcmeGroups",
        ),
        (
            "OUTSIDE_ key is not the family's",
            ACME_FAMILY,
            "class AcmeMarketEntry\n{\n\tconst string Key = \"OUTSIDE_GROUPS_KEY\";\n};\n",
            0,
            "OK",
        ),
        (
            "core name is allowed",
            ACME_FAMILY,
            "class AcmeMarketEntry\n{\n\tAcmeCoreBase base;\n};\n",
            0,
            "OK",
        ),
        (
            "rule names only the core without frameworks",
            no_framework,
            sibling,
            1,
            "a mod requires only AcmeCore",
        ),
        (
            "rule lists every framework",
            two_frameworks,
            sibling,
            1,
            "a mod requires only Kit, Other and AcmeCore",
        ),
    ];
    for (name, family, market, expected, needle) in cases {
        let out = acme_tree(family, market, "");
        assert_eq!(
            code(&out),
            Some(expected),
            "{name}: {}{}",
            stdout(&out),
            stderr(&out)
        );
        assert!(stdout(&out).contains(needle), "{name}: {}", stdout(&out));
    }
    let out = acme_tree(ACME_FAMILY, sibling, "`AcmeGroupsThing`\n");
    assert_eq!(code(&out), Some(0), "{}{}", stdout(&out), stderr(&out));
    assert!(stdout(&out).contains("OK"), "{}", stdout(&out));
}

#[test]
fn a_violation_reports_its_line_and_quotes_the_rule() {
    let out = check("sibling_code");
    assert!(
        stdout(&out).contains(&format!(
            "{}:3:",
            std::path::Path::new("mods/AcmeGroups/scripts/Groups.c").display()
        )),
        "{}",
        stdout(&out)
    );
    assert!(
        stdout(&out).contains("a mod requires only Kit, Forge and AcmeCore"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn links_are_walked_but_a_server_folder_is_not() {
    let scratch = common::Scratch::new("boundaries-links");
    common::write_family(&scratch, common::ACME_FAMILY_TWO_FRAMEWORKS);
    let root = scratch.path().to_path_buf();
    let write = |rel: &str, text: &str| common::write_file(&root.join(rel), text);
    write(
        "mods/AcmeCore/scripts/Core.c",
        "class AcmeCoreBase\n{\n};\n",
    );
    write(
        "mods/AcmeGroups/scripts/Groups.c",
        "class AcmeGroupsThing\n{\n};\n",
    );
    write(
        "mods/AcmeMarket/scripts/Market.c",
        "class AcmeMarketEntry\n{\n\tAcmeGroupsThing sibling;\n};\n",
    );
    write(
        "linked/Use.C",
        "class Thing\n{\n\tAcmeMarketEntry entry;\n};\n",
    );
    write(
        "linked/Upper.C",
        "class Thing\n{\n\tconst string Key = \"ACME_MARKET_KEY\";\n};\n",
    );
    write(
        "server/Sneak.c",
        "class Thing\n{\n\tAcmeMarketEntry entry;\n};\n",
    );
    write(
        "elsewhere/AcmeMap/scripts/Map.c",
        "class Thing\n{\n\tAcmeMarketEntry entry;\n};\n",
    );
    let link = |target: &str, at: &str| {
        std::os::unix::fs::symlink(root.join(target), root.join(at)).unwrap()
    };
    link("linked", "mods/AcmeGroups/scripts/ext");
    link("server", "mods/AcmeGroups/server");
    link("elsewhere/AcmeMap", "mods/AcmeMap");
    link("mods/AcmeMarket", "mods/AcmeGroups/scripts/alias");
    link("server", "mods/AcmeGroups/Server");
    std::os::unix::fs::symlink(
        root.join("missing.c"),
        root.join("mods/AcmeGroups/scripts/Dead.c"),
    )
    .unwrap();

    let out = checks(&["check-boundaries", "--root", root.to_str().unwrap()]);
    assert_eq!(code(&out), Some(1), "{}{}", stdout(&out), stderr(&out));
    let text = stdout(&out);
    for needle in [
        "Use.C",
        "Upper.C",
        &std::path::Path::new("AcmeMap/scripts/Map.c")
            .display()
            .to_string(),
        &std::path::Path::new("AcmeMarket/scripts/Market.c")
            .display()
            .to_string(),
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
        let scratch = common::Scratch::new("boundaries-cycle");
        common::write_family(&scratch, common::ACME_FAMILY_TWO_FRAMEWORKS);
        let folder = scratch.join("mods/AcmeGroups");
        common::write_file(
            &folder.join("scripts/Groups.c"),
            "class AcmeGroupsThing\n{\n};\n",
        );
        let target = if target == "FOLDER" {
            folder.clone()
        } else {
            std::path::PathBuf::from(target)
        };
        std::os::unix::fs::symlink(&target, folder.join("scripts").join(at)).unwrap();
        let out = checks(&[
            "check-boundaries",
            "--root",
            scratch.path().to_str().unwrap(),
        ]);
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
fn an_unreadable_folder_cannot_run() {
    use std::os::unix::fs::PermissionsExt;
    let scratch = common::Scratch::new("boundaries-unreadable");
    common::write_family(&scratch, common::ACME_FAMILY_TWO_FRAMEWORKS);
    let folder = scratch.join("mods/AcmeGroups");
    common::write_file(&folder.join("Groups.c"), "class AcmeGroupsThing\n{\n};\n");
    std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o0)).unwrap();
    let out = checks(&[
        "check-boundaries",
        "--root",
        scratch.path().to_str().unwrap(),
    ]);
    std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(code(&out), Some(2), "{}{}", stdout(&out), stderr(&out));
    assert!(stdout(&out).contains("cannot read"), "{}", stdout(&out));
}
