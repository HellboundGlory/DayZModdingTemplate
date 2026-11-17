mod common;

use common::{
    checks, code, fixture, stderr, stdout, write_family, Scratch, ACME_FAMILY,
    ACME_FAMILY_TWO_FRAMEWORKS,
};

const ALLOWLIST: &str = "tools/modded-allowlist.txt";
const CAR: &str = "mods/AcmeCore/Scripts/4_World/CarScript.c";
const CAR_ROW: &str = "AcmeCore\tCarScript\tvanilla\ta car keeps its owner across restarts\n";
const HEADER: &str = "// Why: a boat keeps its owner across restarts.\n\
// Removing this loses the owner.\n\
// Layer: 4_World, because BoatScript is defined here.\n\
// Other mods patch BoatScript too: see the conflict register.\n\n";

type Edit = (&'static str, &'static str, &'static str);
type Case = (
    &'static str,
    Vec<Edit>,
    Vec<(&'static str, String)>,
    Vec<(&'static str, &'static str)>,
    &'static str,
);

// A copy of the good tree with the given edits, new files and appended text, run through the check.
fn check_with(
    edits: &[Edit],
    adds: &[(&str, String)],
    append: &[(&str, &str)],
) -> std::process::Output {
    let scratch = Scratch::copy_of("modded-case", &fixture("check_modded/good"));
    write_family(&scratch, ACME_FAMILY_TWO_FRAMEWORKS);
    for (rel, old, new) in edits {
        scratch.edit(rel, old, new);
    }
    for (rel, text) in adds {
        scratch.write(rel, text);
    }
    for (rel, text) in append {
        let path = scratch.join(rel);
        let mut body = std::fs::read_to_string(&path).unwrap();
        body.push_str(text);
        std::fs::write(&path, body).unwrap();
    }
    checks(&["check-modded", "--root", scratch.path().to_str().unwrap()])
}

#[test]
fn the_tree_passes() {
    let scratch = Scratch::copy_of("modded", &fixture("check_modded/good"));
    write_family(&scratch, ACME_FAMILY_TWO_FRAMEWORKS);
    let out = checks(&["check-modded", "--root", scratch.path().to_str().unwrap()]);
    assert_eq!(code(&out), Some(0), "{}{}", stdout(&out), stderr(&out));
    assert!(
        stdout(&out).contains("OK: 2 patches, 2 rows, 2 mods"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn the_test_mod_is_not_scanned() {
    let out = check_with(&[], &[], &[]);
    assert_eq!(code(&out), Some(0), "{}{}", stdout(&out), stderr(&out));
    assert!(!stdout(&out).contains("AcmeCoreTests"), "{}", stdout(&out));
}

#[test]
fn a_commented_out_patch_is_not_counted() {
    let ghost = HEADER.replace("BoatScript", "Ghost") + "// modded class Ghost\n// {\n// }\n";
    let scratch = Scratch::copy_of("modded-comment", &fixture("check_modded/good"));
    write_family(&scratch, ACME_FAMILY_TWO_FRAMEWORKS);
    scratch.write("mods/AcmeOther/Scripts/3_Game/Ghost.c", &ghost);
    let out = checks(&["check-modded", "--root", scratch.path().to_str().unwrap()]);
    assert_eq!(code(&out), Some(0), "{}{}", stdout(&out), stderr(&out));
    assert!(stdout(&out).contains("OK: 2 patches"), "{}", stdout(&out));
}

#[test]
fn a_patch_named_inside_a_string_is_not_counted() {
    let out = check_with(
        &[(
            CAR,
            "protected string m_AcmeCoreOwner;",
            "protected string m_AcmeCoreOwner;\n    string m_Note = \"modded class Ghost\";",
        )],
        &[],
        &[],
    );
    assert_eq!(code(&out), Some(0), "{}{}", stdout(&out), stderr(&out));
    assert!(stdout(&out).contains("OK: 2 patches"), "{}", stdout(&out));
}

#[test]
fn a_broken_row_is_not_also_reported_as_missing() {
    let out = check_with(
        &[(ALLOWLIST, CAR_ROW, "AcmeCore\tCarScript\tvanilla\n")],
        &[],
        &[],
    );
    assert_eq!(code(&out), Some(1), "{}{}", stdout(&out), stderr(&out));
    assert!(!stdout(&out).contains("has no row"), "{}", stdout(&out));
}

#[test]
fn a_acme_class_patched_in_another_mod_needs_a_acme_row() {
    let out = check_with(
        &[],
        &[(
            "mods/AcmeOther/Scripts/3_Game/AcmeCoreThing.c",
            HEADER
                .replace("BoatScript", "AcmeCoreThing")
                .replace("a boat", "a core class")
                + "modded class AcmeCoreThing\n{\n    int AcmeOtherValue;\n}\n",
        )],
        &[(
            ALLOWLIST,
            "AcmeOther\tAcmeCoreThing\tacme\tadds a value Core reads\n",
        )],
    );
    assert_eq!(code(&out), Some(0), "{}{}", stdout(&out), stderr(&out));
    assert!(
        stdout(&out).contains("OK: 3 patches, 3 rows"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn an_owner_that_does_not_fit_the_name_fails() {
    let cls = "AcmeCoreThing";
    let out = check_with(
        &[],
        &[(
            &format!("mods/AcmeCore/Scripts/4_World/{cls}.c"),
            format!("{HEADER}modded class {cls}\n{{\n}}\n").replace("BoatScript", cls),
        )],
        &[(ALLOWLIST, &format!("AcmeCore\t{cls}\tvanilla\tbecause\n"))],
    );
    assert_eq!(
        code(&out),
        Some(1),
        "{cls}: {}{}",
        stdout(&out),
        stderr(&out)
    );
    assert!(
        stdout(&out).contains(&format!("{cls} can't have the owner vanilla")),
        "{cls}: {}",
        stdout(&out)
    );
}

#[test]
fn a_scripts_folder_in_either_spelling_is_scanned() {
    let out = check_with(
        &[],
        &[(
            "mods/AcmeCore/scripts/4_World/BoatScript.c",
            format!("{HEADER}modded class BoatScript\n{{\n}}\n"),
        )],
        &[],
    );
    assert_eq!(code(&out), Some(1), "{}{}", stdout(&out), stderr(&out));
    assert!(stdout(&out).contains("BoatScript"), "{}", stdout(&out));
}

#[test]
fn no_mods_cannot_run() {
    let scratch = Scratch::new("modded-no-mods");
    write_family(&scratch, ACME_FAMILY_TWO_FRAMEWORKS);
    scratch.write("tests/.keep", "");
    let out = checks(&["check-modded", "--root", scratch.path().to_str().unwrap()]);
    assert_eq!(code(&out), Some(2), "{}{}", stdout(&out), stderr(&out));
    assert!(stdout(&out).contains("no mods under"), "{}", stdout(&out));
}

#[test]
fn an_empty_mods_folder_is_skipped() {
    let scratch = Scratch::new("modded-empty-mods");
    write_family(&scratch, ACME_FAMILY_TWO_FRAMEWORKS);
    scratch.write("mods/.gitkeep", "");
    let out = checks(&["check-modded", "--root", scratch.path().to_str().unwrap()]);
    assert_eq!(code(&out), Some(0), "{}{}", stdout(&out), stderr(&out));
    assert!(
        stdout(&out).starts_with("skipped: no mods under"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn an_allowlist_with_no_rows_is_accepted() {
    let scratch = Scratch::new("modded-no-rows");
    write_family(&scratch, ACME_FAMILY_TWO_FRAMEWORKS);
    scratch.write(
        "mods/AcmeCore/Scripts/4_World/AcmeCoreThing.c",
        "class AcmeCoreThing\n{\n};\n",
    );
    scratch.write(ALLOWLIST, "# Every modded class, one row each.\n");
    let out = checks(&["check-modded", "--root", scratch.path().to_str().unwrap()]);
    assert_eq!(code(&out), Some(0), "{}{}", stdout(&out), stderr(&out));
}

#[test]
fn an_empty_mods_folder_still_needs_the_family_file() {
    let scratch = Scratch::new("modded-empty-mods-no-family");
    scratch.write("mods/.gitkeep", "");
    let out = checks(&["check-modded", "--root", scratch.path().to_str().unwrap()]);
    assert_eq!(code(&out), Some(2), "{}{}", stdout(&out), stderr(&out));
    assert!(stdout(&out).contains("not found"), "{}", stdout(&out));
}

#[test]
fn problems_are_reported_at_their_place() {
    let boat_row = "AcmeCore\tBoatScript\tvanilla\ta boat keeps its owner\n";
    let cases: Vec<Case> = vec![
        (
            "a patch with no row",
            vec![(ALLOWLIST, CAR_ROW, "")],
            vec![],
            vec![],
            "mods/AcmeCore/Scripts/4_World/CarScript.c:6: CarScript has no row in tools/modded-allowlist.txt",
        ),
        (
            "a row with no patch",
            vec![],
            vec![],
            vec![(ALLOWLIST, boat_row)],
            "tools/modded-allowlist.txt:6: BoatScript in AcmeCore matches no patch",
        ),
        (
            "a row with three columns",
            vec![(ALLOWLIST, CAR_ROW, "AcmeCore\tCarScript\tvanilla\n")],
            vec![],
            vec![],
            "tools/modded-allowlist.txt:4: a row needs 4 tab separated columns, not 3",
        ),
        (
            "a row with an unknown owner",
            vec![(ALLOWLIST, CAR_ROW, "AcmeCore\tCarScript\tengine\ta car keeps its owner across restarts\n")],
            vec![],
            vec![],
            "tools/modded-allowlist.txt:4: engine is not an owner: vanilla, kit, forge or acme",
        ),
        (
            "a row naming Upstream as the owner",
            vec![(ALLOWLIST, CAR_ROW, "AcmeCore\tCarScript\treference\ta car keeps its owner across restarts\n")],
            vec![],
            vec![],
            "tools/modded-allowlist.txt:4: reference is not an owner: vanilla, kit, forge or acme",
        ),
        (
            "a row naming Beacon as the owner",
            vec![(ALLOWLIST, CAR_ROW, "AcmeCore\tCarScript\tbeacon\ta car keeps its owner across restarts\n")],
            vec![],
            vec![],
            "tools/modded-allowlist.txt:4: beacon is not an owner: vanilla, kit, forge or acme",
        ),
        (
            "a row with an empty reason",
            vec![(ALLOWLIST, CAR_ROW, "AcmeCore\tCarScript\tvanilla\t\n")],
            vec![],
            vec![],
            "tools/modded-allowlist.txt:4: the reason for CarScript is empty",
        ),
        (
            "a duplicate row",
            vec![],
            vec![],
            vec![(ALLOWLIST, CAR_ROW)],
            "tools/modded-allowlist.txt:6: CarScript in AcmeCore already has a row",
        ),
        (
            "a acme owner on a vanilla class",
            vec![(ALLOWLIST, CAR_ROW, "AcmeCore\tCarScript\tacme\ta car keeps its owner across restarts\n")],
            vec![],
            vec![],
            "tools/modded-allowlist.txt:4: acme covers only a Acme class, not CarScript",
        ),
        (
            "a acme patch with no row",
            vec![],
            vec![(
                "mods/AcmeOther/Scripts/3_Game/AcmeCoreThing.c",
                HEADER.replace("BoatScript", "AcmeCoreThing") + "modded class AcmeCoreThing\n{\n}\n",
            )],
            vec![],
            "mods/AcmeOther/Scripts/3_Game/AcmeCoreThing.c:6: AcmeCoreThing has no row in tools/modded-allowlist.txt",
        ),
        (
            "two patches in one file",
            vec![(CAR, "modded class CarScript", "modded class CarScript\n{\n}\n\nmodded class CarScript")],
            vec![],
            vec![],
            "mods/AcmeCore/Scripts/4_World/CarScript.c:6: CarScript.c holds 2 modded class lines",
        ),
        (
            "a file not named after its class",
            vec![],
            vec![(
                "mods/AcmeCore/Scripts/4_World/Boat.c",
                HEADER.to_string() + "modded class BoatScript\n{\n}\n",
            )],
            vec![(ALLOWLIST, boat_row)],
            "mods/AcmeCore/Scripts/4_World/Boat.c:6: Boat.c is not named after BoatScript",
        ),
        (
            "a header shorter than four lines",
            vec![(CAR, "// Other mods patch CarScript too: see the conflict register.\n", "")],
            vec![],
            vec![],
            "mods/AcmeCore/Scripts/4_World/CarScript.c:1: the header is shorter than four // lines",
        ),
        (
            "a header line that is not a comment",
            vec![(CAR, "// Other mods patch CarScript too: see the conflict register.", "Other mods patch CarScript too.")],
            vec![],
            vec![],
            "mods/AcmeCore/Scripts/4_World/CarScript.c:4: a header line is not a // comment",
        ),
        (
            "a first header line that is not Why",
            vec![(CAR, "// Why:", "// Because:")],
            vec![],
            vec![],
            "mods/AcmeCore/Scripts/4_World/CarScript.c:1: the first header line does not start with // Why:",
        ),
        (
            "a third header line that is not Layer",
            vec![(CAR, "// Layer:", "// Where:")],
            vec![],
            vec![],
            "mods/AcmeCore/Scripts/4_World/CarScript.c:3: the third header line does not start with // Layer:",
        ),
    ];
    for (name, edits, adds, append, needle) in cases {
        let out = check_with(&edits, &adds, &append);
        assert_eq!(
            code(&out),
            Some(1),
            "{name}: {}{}",
            stdout(&out),
            stderr(&out)
        );
        assert!(
            stdout(&out).contains(needle),
            "{name}: wanted {needle:?} in\n{}",
            stdout(&out)
        );
    }
}

#[test]
fn a_mod_with_no_scripts_folder_is_skipped() {
    assert!(fixture("check_modded/good/mods/AcmeEmpty")
        .join("config.cpp")
        .is_file());
    let out = check_with(&[], &[], &[]);
    assert_eq!(code(&out), Some(0), "{}{}", stdout(&out), stderr(&out));
    assert!(
        stdout(&out).contains("OK: 2 patches, 2 rows, 2 mods"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn a_separator_control_character_is_whitespace_in_a_patch_line() {
    let out = check_with(
        &[],
        &[(
            "mods/AcmeCore/Scripts/3_Game/ZetaScript.c",
            HEADER
                .replace("BoatScript", "ZetaScript")
                .replace("a boat", "a zeta")
                + "modded\u{1d}class ZetaScript\n{\n    int ZetaValue;\n}\n",
        )],
        &[],
    );
    assert_eq!(code(&out), Some(1), "{}{}", stdout(&out), stderr(&out));
    assert!(
        stdout(&out).contains("ZetaScript has no row in tools/modded-allowlist.txt"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn the_family_sets_the_owners_and_the_prefix() {
    // (class, owner in the row, the message the run must print; None for a pass)
    let cases: [(&str, &str, Option<&str>); 7] = [
        ("AcmeCoreThing", "acme", None),
        ("CarScript", "vanilla", None),
        (
            "AcmeCoreThing",
            "kit",
            Some("AcmeCoreThing can't have the owner kit"),
        ),
        (
            "CarScript",
            "acme",
            Some("acme covers only a Acme class, not CarScript"),
        ),
        (
            "OutsideCoreThing",
            "acme",
            Some("acme covers only a Acme class, not OutsideCoreThing"),
        ),
        (
            "OutsideCoreThing",
            "outside",
            Some("outside is not an owner: vanilla, kit or acme"),
        ),
        (
            "CarScript",
            "reference",
            Some("reference is not an owner: vanilla, kit or acme"),
        ),
    ];
    for (cls, owner, message) in cases {
        let scratch = Scratch::new("modded-acme");
        write_family(&scratch, ACME_FAMILY);
        scratch.write(
            &format!("mods/AcmeCore/Scripts/4_World/{cls}.c"),
            &format!("{HEADER}modded class {cls}\n{{\n}}\n").replace("BoatScript", cls),
        );
        scratch.write(ALLOWLIST, &format!("AcmeCore\t{cls}\t{owner}\tbecause\n"));
        let out = checks(&["check-modded", "--root", scratch.path().to_str().unwrap()]);
        match message {
            None => {
                assert_eq!(
                    code(&out),
                    Some(0),
                    "{cls}/{owner}: {}{}",
                    stdout(&out),
                    stderr(&out)
                );
                assert!(
                    stdout(&out).contains("OK: 1 patch, 1 row, 1 mod"),
                    "{cls}/{owner}: {}",
                    stdout(&out)
                );
            }
            Some(needle) => {
                assert_eq!(
                    code(&out),
                    Some(1),
                    "{cls}/{owner}: {}{}",
                    stdout(&out),
                    stderr(&out)
                );
                assert!(
                    stdout(&out).contains(needle),
                    "{cls}/{owner}: {}",
                    stdout(&out)
                );
            }
        }
    }
}
