mod common;

use common::{
    checks, code, stderr, stdout, write_family, write_file, Scratch, ACME_FAMILY,
    ACME_FAMILY_TWO_FRAMEWORKS,
};
use std::process::Command;

const CORE_CONFIG: &str = "class CfgMods\n{\n    class AcmeCore\n    {\n        type = \"mod\";\n        dir = \"AcmeCore\";\n        inputs = \"AcmeCore/Data/inputs.xml\";\n    };\n};\n";
const SORT_CONFIG: &str = "class CfgMods\n{\n    class AcmeSort\n    {\n        type = \"mod\";\n        dir = \"AcmeSort\";\n        inputs = \"AcmeSort/Data/inputs.xml\";\n    };\n};\n";
const QUIET_CONFIG: &str = "class CfgMods\n{\n    class AcmeQuiet\n    {\n        type = \"mod\";\n        dir = \"AcmeQuiet\";\n    };\n};\n";
const CORE_INPUTS: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\" ?>\n<modded_inputs>\n    <inputs>\n        <actions>\n            <input name=\"UAAcmeCoreOpenCommand\" loc=\"STR_ACME_CORE_INPUT_OPEN_COMMAND\" />\n        </actions>\n        <sorting name=\"acme\" loc=\"STR_ACME_CORE_INPUT_GROUP\">\n            <input name=\"UAAcmeCoreOpenCommand\" />\n        </sorting>\n    </inputs>\n    <preset>\n        <input name=\"UAAcmeCoreOpenCommand\">\n            <btn name=\"kSlash\" />\n        </input>\n    </preset>\n</modded_inputs>\n";
const SORT_INPUTS: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\" ?>\n<modded_inputs>\n    <inputs>\n        <actions>\n            <input name=\"UAAcmeSortAutoSort\" loc=\"STR_ACME_CORE_INPUT_OPEN_COMMAND\" />\n        </actions>\n        <sorting name=\"acme\" loc=\"STR_ACME_CORE_INPUT_GROUP\">\n            <input name=\"UAAcmeSortAutoSort\" />\n        </sorting>\n    </inputs>\n    <preset>\n        <input name=\"UAAcmeSortAutoSort\">\n            <btn name=\"kF6\" />\n        </input>\n    </preset>\n</modded_inputs>\n";
const CORE_TABLE: &str = "\"Language\",\"original\",\"english\",\n\"STR_ACME_CORE_INPUT_GROUP\",\"Acme\",\"Acme\",\n\"STR_ACME_CORE_INPUT_OPEN_COMMAND\",\"Open chat for a command\",\"Open chat for a command\",\n";
const CORE_SCRIPT: &str = "class AcmeCoreProbe\n{\n    void Look()\n    {\n        UAInput input = GetUApi().GetInputByName(\"UAAcmeCoreOpenCommand\");\n        if (input)\n        {\n            input.LocalPress();\n        }\n    }\n}\n";
const SORT_SCRIPT: &str = "class AcmeSortProbe\n{\n    void Look()\n    {\n        UAInput input = GetUApi().GetInputByName(\"UAAcmeSortAutoSort\");\n        if (input)\n        {\n            input.LocalPress();\n        }\n    }\n}\n";
const CHECK_SCRIPT: &str = "class AcmeCoreTestsProbe\n{\n    void Look()\n    {\n        if (KeyState(KeyCode.KC_SLASH) != 0)\n        {\n            return;\n        }\n    }\n}\n";
const QUIET_SCRIPT: &str = "class AcmeQuietProbe\n{\n    void Look()\n    {\n    }\n}\n";

const FILES: [(&str, &str); 10] = [
    ("mods/AcmeCore/config.cpp", CORE_CONFIG),
    ("mods/AcmeCore/Data/inputs.xml", CORE_INPUTS),
    ("mods/AcmeCore/stringtable.csv", CORE_TABLE),
    ("mods/AcmeCore/Scripts/3_Game/AcmeCoreProbe.c", CORE_SCRIPT),
    ("mods/AcmeSort/config.cpp", SORT_CONFIG),
    ("mods/AcmeSort/Data/inputs.xml", SORT_INPUTS),
    ("mods/AcmeSort/Scripts/3_Game/AcmeSortProbe.c", SORT_SCRIPT),
    ("mods/AcmeQuiet/config.cpp", QUIET_CONFIG),
    (
        "mods/AcmeQuiet/Scripts/3_Game/AcmeQuietProbe.c",
        QUIET_SCRIPT,
    ),
    ("tests/AcmeCoreTests/Scripts/3_Game/Probe.c", CHECK_SCRIPT),
];

fn tree() -> Scratch {
    let scratch = Scratch::new("inputs");
    for (rel, text) in FILES {
        scratch.write(rel, text);
    }
    write_family(&scratch, ACME_FAMILY_TWO_FRAMEWORKS);
    scratch
}

// The same tree with the one-framework family.
// The same tree for a family with another prefix, core and framework.
fn zed_tree() -> Scratch {
    let zed = |text: &str| {
        text.replace("Acme", "Zed")
            .replace("ACME", "ZED")
            .replace("acme", "zed")
    };
    let scratch = Scratch::new("inputs-zed");
    for (rel, text) in FILES {
        scratch.write(&zed(rel), &zed(text));
    }
    write_family(&scratch, &zed(ACME_FAMILY));
    scratch
}

type Edit = (&'static str, &'static str, &'static str);

fn edited(scratch: Scratch, edits: &[Edit], remove: &[&str]) -> Scratch {
    for (rel, old, new) in edits {
        scratch.edit(rel, old, new);
    }
    for rel in remove {
        let path = scratch.join(rel);
        if path.is_dir() {
            std::fs::remove_dir_all(path).unwrap();
        } else {
            std::fs::remove_file(path).unwrap();
        }
    }
    scratch
}

fn build(edits: &[Edit], remove: &[&str]) -> Scratch {
    edited(tree(), edits, remove)
}

fn check(edits: &[Edit], remove: &[&str]) -> std::process::Output {
    let scratch = build(edits, remove);
    checks(&["check-inputs", scratch.path().to_str().unwrap()])
}

fn assert_problem(name: &str, out: &std::process::Output, needle: &str) {
    assert_eq!(code(out), Some(1), "{name}: {}{}", stdout(out), stderr(out));
    assert!(stdout(out).contains(needle), "{name}: {}", stdout(out));
}

#[test]
fn the_cases_pass() {
    let out = check(&[], &[]);
    assert_eq!(code(&out), Some(0), "{}{}", stdout(&out), stderr(&out));
    assert!(
        stdout(&out).contains("OK: 2 inputs files, 2 actions, 3 scripts"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn a_relative_root_passes() {
    let scratch = tree();
    let out = Command::new(env!("CARGO_BIN_EXE_checks"))
        .args(["check-inputs", "."])
        .current_dir(scratch.path())
        .output()
        .unwrap();
    assert_eq!(code(&out), Some(0), "{}{}", stdout(&out), stderr(&out));
    assert!(stdout(&out).contains("OK:"), "{}", stdout(&out));
}

#[test]
fn problems_are_reported() {
    let sort = "mods/AcmeSort/Data/inputs.xml";
    let cases: Vec<(&str, Vec<Edit>, Vec<&str>, &str)> = vec![
        (
            "a missing file",
            vec![],
            vec![sort],
            "inputs names AcmeSort/Data/inputs.xml which is not there",
        ),
        (
            "bad XML",
            vec![(sort, "<actions>", "<actions")],
            vec![],
            "is not valid XML",
        ),
        (
            "a wrong root",
            vec![
                (sort, "<modded_inputs>", "<inputs>"),
                (sort, "</modded_inputs>", "</inputs>"),
            ],
            vec![],
            "root is inputs, not modded_inputs",
        ),
        (
            "an action not in the sorting",
            vec![(
                sort,
                "            <input name=\"UAAcmeSortAutoSort\" />\n        </sorting>",
                "        </sorting>",
            )],
            vec![],
            "action UAAcmeSortAutoSort is not in a sorting block",
        ),
        (
            "a sorting group not called acme",
            vec![(sort, "name=\"acme\"", "name=\"sorting\"")],
            vec![],
            "sorting name is sorting, not acme",
        ),
        (
            "a loc key in no stringtable",
            vec![(
                sort,
                "loc=\"STR_ACME_CORE_INPUT_OPEN_COMMAND\"",
                "loc=\"STR_ACME_SORT_MISSING\"",
            )],
            vec![],
            "STR_ACME_SORT_MISSING is in no stringtable",
        ),
        (
            "a sorting loc key in no stringtable",
            vec![(
                sort,
                "loc=\"STR_ACME_CORE_INPUT_GROUP\"",
                "loc=\"STR_ACME_SORT_MISSING_GROUP\"",
            )],
            vec![],
            "STR_ACME_SORT_MISSING_GROUP is in no stringtable",
        ),
        (
            "a file outside the mod",
            vec![(
                "mods/AcmeSort/config.cpp",
                "inputs = \"AcmeSort/Data/inputs.xml\";",
                "inputs = \"AcmeSort/../AcmeCore/Data/inputs.xml\";",
            )],
            vec![],
            "is outside mods/AcmeSort",
        ),
        (
            "an inputs line with no semicolon",
            vec![(
                "mods/AcmeSort/config.cpp",
                "inputs = \"AcmeSort/Data/inputs.xml\";",
                "inputs = \"AcmeSort/Data/inputs.xml\"",
            )],
            vec![],
            "the inputs line has no ; after it",
        ),
        (
            "an action in two mods",
            vec![(
                sort,
                "<input name=\"UAAcmeSortAutoSort\" loc=",
                "<input name=\"UAAcmeCoreOpenCommand\" loc=",
            )],
            vec![],
            "UAAcmeCoreOpenCommand is also declared in mods/AcmeCore/Data/inputs.xml",
        ),
        (
            "an action without the mod id",
            vec![(sort, "UAAcmeSortAutoSort", "UASortAutoSort")],
            vec![],
            "action UASortAutoSort is not named UAAcmeSort",
        ),
        (
            "an input looked up but never declared",
            vec![(
                "mods/AcmeSort/Scripts/3_Game/AcmeSortProbe.c",
                "\"UAAcmeSortAutoSort\"",
                "\"UAAcmeSortGone\"",
            )],
            vec![],
            "GetInputByName names UAAcmeSortGone, which no inputs file declares",
        ),
        (
            "a preset for an undeclared action",
            vec![(
                sort,
                "<input name=\"UAAcmeSortAutoSort\">",
                "<input name=\"UAAcmeSortTypo\">",
            )],
            vec![],
            "preset names UAAcmeSortTypo, which no action declares",
        ),
        (
            "a sorting entry for an undeclared action",
            vec![(
                sort,
                "<input name=\"UAAcmeSortAutoSort\" />",
                "<input name=\"UAAcmeSortAutoSort\" /><input name=\"UAAcmeSortTypo\" />",
            )],
            vec![],
            "sorting names UAAcmeSortTypo, which no action declares",
        ),
        (
            "an action with no loc",
            vec![(sort, " loc=\"STR_ACME_CORE_INPUT_OPEN_COMMAND\"", "")],
            vec![],
            "action UAAcmeSortAutoSort has no loc",
        ),
        (
            "a sorting group with no loc",
            vec![(sort, " loc=\"STR_ACME_CORE_INPUT_GROUP\"", "")],
            vec![],
            "sorting acme has no loc",
        ),
        (
            "actions outside the inputs block",
            vec![
                (sort, "<inputs>", "<wrong>"),
                (sort, "</inputs>", "</wrong>"),
            ],
            vec![],
            "declares no actions",
        ),
        (
            "an inputs path with no mod folder",
            vec![(
                "mods/AcmeSort/config.cpp",
                "inputs = \"AcmeSort/Data/inputs.xml\";",
                "inputs = \"Data/inputs.xml\";",
            )],
            vec![],
            "inputs names Data/inputs.xml, which doesn't start with AcmeSort/",
        ),
        (
            "a raw key",
            vec![(
                "mods/AcmeSort/Scripts/3_Game/AcmeSortProbe.c",
                "input.LocalPress();",
                "if (KeyCode.KC_F6) { input.LocalPress(); }",
            )],
            vec![],
            "KeyCode is not allowed",
        ),
    ];
    for (name, edits, remove, needle) in cases {
        assert_problem(name, &check(&edits, &remove), needle);
    }
}

#[test]
fn another_family_is_checked_by_its_own_names() {
    let sort = "mods/ZedSort/Data/inputs.xml";
    let probe = "mods/ZedSort/Scripts/3_Game/ZedSortProbe.c";
    let base = edited(zed_tree(), &[], &[]);
    let out = checks(&["check-inputs", base.path().to_str().unwrap()]);
    assert_eq!(code(&out), Some(0), "{}{}", stdout(&out), stderr(&out));
    assert!(
        stdout(&out).contains("OK: 2 inputs files, 2 actions, 3 scripts"),
        "{}",
        stdout(&out)
    );
    let cases: Vec<(&str, Vec<Edit>, Vec<&str>, &str)> = vec![
        (
            "a sorting group not called zed",
            vec![(sort, "name=\"zed\"", "name=\"sorting\"")],
            vec![],
            "sorting name is sorting, not zed",
        ),
        (
            "a sorting group named for outside",
            vec![(sort, "name=\"zed\"", "name=\"outside\"")],
            vec![],
            "sorting name is outside, not zed",
        ),
        (
            "a loc key in no stringtable",
            vec![(
                sort,
                "loc=\"STR_ZED_CORE_INPUT_OPEN_COMMAND\"",
                "loc=\"STR_ZED_SORT_MISSING\"",
            )],
            vec![],
            "STR_ZED_SORT_MISSING is in no stringtable",
        ),
        (
            "an action without the mod id",
            vec![(sort, "UAZedSortAutoSort", "UASortAutoSort")],
            vec![],
            "action UASortAutoSort is not named UAZedSort",
        ),
        (
            "an input looked up but never declared",
            vec![(probe, "\"UAZedSortAutoSort\"", "\"UAZedSortGone\"")],
            vec![],
            "GetInputByName names UAZedSortGone, which no inputs file declares",
        ),
        (
            "a raw key",
            vec![(
                probe,
                "input.LocalPress();",
                "if (KeyCode.KC_F6) { input.LocalPress(); }",
            )],
            vec![],
            "KeyCode is not allowed",
        ),
    ];
    for (name, edits, remove, needle) in cases {
        let scratch = edited(zed_tree(), &edits, &remove);
        let out = checks(&["check-inputs", scratch.path().to_str().unwrap()]);
        assert_problem(name, &out, needle);
    }
    let outside_lookup = [(probe, "\"UAZedSortAutoSort\"", "\"UAOutsideSortGone\"")];
    let scratch = edited(zed_tree(), &outside_lookup, &[]);
    let out = checks(&["check-inputs", scratch.path().to_str().unwrap()]);
    assert_eq!(code(&out), Some(0), "{}{}", stdout(&out), stderr(&out));

    let raw_key = "int key = KeyCode.KC_F6;\n";
    let allowed = zed_tree();
    allowed.write("mods/ZedCore/Scripts/5_Mission/ZedCoreKeys.c", raw_key);
    let out = checks(&["check-inputs", allowed.path().to_str().unwrap()]);
    assert_eq!(code(&out), Some(0), "{}{}", stdout(&out), stderr(&out));
    let stray = zed_tree();
    stray.write("mods/ZedCore/Scripts/5_Mission/OutsideCoreKeys.c", raw_key);
    let out = checks(&["check-inputs", stray.path().to_str().unwrap()]);
    assert_problem(
        "a raw key in an outside-named key file",
        &out,
        "KeyCode is not allowed",
    );
}

#[test]
fn a_vanilla_action_passes() {
    let probe = "mods/AcmeSort/Scripts/3_Game/AcmeSortProbe.c";
    for call in ["GetInputByName(\"UAUIBack\")", "GetInputByID(UAUIBack)"] {
        let out = check(
            &[(probe, "GetInputByName(\"UAAcmeSortAutoSort\")", call)],
            &[],
        );
        assert_eq!(
            code(&out),
            Some(0),
            "{call}: {}{}",
            stdout(&out),
            stderr(&out)
        );
    }
}

#[test]
fn a_mod_with_no_inputs_line_passes() {
    let out = check(&[], &[]);
    assert_eq!(code(&out), Some(0), "{}{}", stdout(&out), stderr(&out));
    assert!(
        stdout(&out).contains("OK: 2 inputs files"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn tests_are_not_scanned() {
    let out = check(&[], &[]);
    assert_eq!(code(&out), Some(0), "{}{}", stdout(&out), stderr(&out));
    assert!(!stdout(&out).contains("KeyCode"), "{}", stdout(&out));
}

#[test]
fn a_stringtable_that_is_not_utf8_is_a_problem() {
    let scratch = tree();
    std::fs::write(
        scratch.join("mods/AcmeCore/stringtable.csv"),
        b"\"Language\"\xff,\"original\"\n",
    )
    .unwrap();
    let out = checks(&["check-inputs", scratch.path().to_str().unwrap()]);
    assert_eq!(code(&out), Some(1), "{}{}", stdout(&out), stderr(&out));
    assert!(
        stdout(&out).contains("cannot read: not UTF-8"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn a_comment_is_not_an_inputs_line() {
    let out = check(
        &[(
            "mods/AcmeQuiet/config.cpp",
            "        dir = \"AcmeQuiet\";",
            "        dir = \"AcmeQuiet\";\n        // inputs = \"AcmeQuiet/Data/missing.xml\";",
        )],
        &[],
    );
    assert_eq!(code(&out), Some(0), "{}{}", stdout(&out), stderr(&out));
}

#[test]
fn no_mods_cannot_run() {
    let scratch = Scratch::new("inputs-no-mods");
    write_file(&scratch.join("tests/.keep"), "");
    let out = checks(&["check-inputs", scratch.path().to_str().unwrap()]);
    assert_eq!(code(&out), Some(2), "{}{}", stdout(&out), stderr(&out));
    assert!(stdout(&out).contains("no mods under"), "{}", stdout(&out));
}

#[test]
fn a_broken_xml_declaration_is_not_valid_xml() {
    let out = check(
        &[(
            "mods/AcmeSort/Data/inputs.xml",
            "<?xml version=",
            "<?xml ve;rsion=",
        )],
        &[],
    );
    assert_eq!(code(&out), Some(1), "{}{}", stdout(&out), stderr(&out));
    assert!(
        stdout(&out)
            .contains("is not valid XML: XML declaration not well-formed: line 1, column 6"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn xml_errors_sit_where_expat_puts_them() {
    let cases: &[(&str, &str, &str)] = &[
        (
            "btn name=\"kF6\"",
            "btn name)\"kF6\"",
            "not well-formed (invalid token): line 13, column 21",
        ),
        (
            "        </input>",
            "        </inpt>",
            "mismatched tag: line 14, column 10",
        ),
        (
            "        </input>",
            "        </in;ut>",
            "not well-formed (invalid token): line 14, column 12",
        ),
        (
            "name=\"kF6\" />",
            "name=\"kF6\"/ >",
            "not well-formed (invalid token): line 13, column 28",
        ),
        (
            "btn name=\"kF6\"",
            "btn name \"kF6\"",
            "not well-formed (invalid token): line 13, column 22",
        ),
        (
            "        </input>",
            "        ]]></input>",
            "not well-formed (invalid token): line 14, column 10",
        ),
        (
            "</modded_inputs>\n",
            "",
            "no element found: line 16, column 0",
        ),
        (
            "<btn name=\"kF6\" />",
            "<btn name=\"kF6",
            "not well-formed (invalid token): line 14, column 8",
        ),
        (
            "<btn name=\"kF6\" />",
            "<btn name=\"kF6\" = />",
            "not well-formed (invalid token): line 13, column 28",
        ),
        (
            "<btn name=\"kF6\" />",
            "<btn name=\"kF6&nope;\" />",
            "undefined entity: line 13, column 12",
        ),
        (
            "<btn name=\"kF6\" />",
            "<btn name=\"kF6&amp\" />",
            "not well-formed (invalid token): line 13, column 30",
        ),
        (
            "        </input>",
            "        <!-- a -- b --></input>",
            "not well-formed (invalid token): line 14, column 17",
        ),
        (
            "        </input>",
            "        <![CDATA[ x </input>",
            "unclosed CDATA section: line 17, column 0",
        ),
        (
            "</modded_inputs>",
            "<!x></modded_inputs>",
            "not well-formed (invalid token): line 16, column 2",
        ),
        (
            "        </input>",
            "        <!--\u{fffe}--></input>",
            "not well-formed (invalid token): line 14, column 12",
        ),
        (
            "btn name=\"kF6\"",
            "btn name=\"kF6\u{fffe}\"",
            "not well-formed (invalid token): line 13, column 26",
        ),
        (
            "        </input>",
            "        <![CDATA[\u{ffff}]]></input>",
            "not well-formed (invalid token): line 14, column 17",
        ),
        (
            "<btn name=\"kF6\" />",
            "<:btn name=\"kF6\" />",
            "not well-formed (invalid token): line 13, column 13",
        ),
        (
            "<btn name=\"kF6\" />",
            "<btn: name=\"kF6\" />",
            "not well-formed (invalid token): line 13, column 17",
        ),
        (
            "        </input>",
            "        <?x`ml?></input>",
            "not well-formed (invalid token): line 14, column 11",
        ),
    ];
    let declarations: &[(&str, &str, &str)] = &[
        (
            "encoding=\"UTF-8\"",
            "encoding=\"(TF-8\"",
            "XML declaration not well-formed: line 1, column 30",
        ),
        (
            "encoding=\"UTF-8\"",
            "encoding=\"UTF 8\"",
            "XML declaration not well-formed: line 1, column 33",
        ),
        (
            "standalone=\"yes\" ?>",
            "standalone=\"yes\" ",
            "unclosed token: line 1, column 0",
        ),
        (
            "encoding=\"UTF-8\"",
            "enco\nding=\"UTF-8\"",
            "XML declaration not well-formed: line 2, column 0",
        ),
        (
            "standalone=\"yes\"",
            "standalone=\"y;es\"",
            "XML declaration not well-formed: line 1, column 50",
        ),
        (
            "version=\"1.0\"",
            "version=\"1.0;\"",
            "XML declaration not well-formed: line 1, column 18",
        ),
    ];
    for (old, new, expected) in cases.iter().chain(declarations) {
        let out = check(&[("mods/AcmeSort/Data/inputs.xml", old, new)], &[]);
        assert_eq!(
            code(&out),
            Some(1),
            "{new}: {}{}",
            stdout(&out),
            stderr(&out)
        );
        let message = format!("is not valid XML: {expected}");
        assert!(stdout(&out).contains(&message), "{new}: {}", stdout(&out));
    }
}

#[test]
fn a_separator_control_character_is_whitespace_in_the_inputs_key() {
    let out = check(
        &[
            ("mods/AcmeSort/config.cpp", "inputs = ", "inputs\u{1d}= "),
            (
                "mods/AcmeSort/Data/inputs.xml",
                "<input name=\"UAAcmeSortAutoSort\" loc=\"STR_ACME_CORE_INPUT_OPEN_COMMAND\" />",
                "<input name=\"UAAcmeSortAutoSort\" />",
            ),
        ],
        &[],
    );
    assert_eq!(code(&out), Some(1), "{}{}", stdout(&out), stderr(&out));
    assert!(
        stdout(&out).contains("action UAAcmeSortAutoSort has no loc"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn a_stringtable_field_over_the_csv_limit_cannot_be_read() {
    let scratch = tree();
    scratch.write(
        "mods/AcmeCore/stringtable.csv",
        &format!(
            "\"Language\",\"original\",\"english\",\n\"{}\",\"x\",\"x\",\n",
            "x".repeat(131_073)
        ),
    );
    let out = checks(&["check-inputs", scratch.path().to_str().unwrap()]);
    assert_eq!(code(&out), Some(1), "{}{}", stdout(&out), stderr(&out));
    assert!(
        stdout(&out).contains("field larger than field limit (131072)"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn a_crlf_inside_an_attribute_value_is_one_space() {
    let out = check(
        &[(
            "mods/AcmeSort/Data/inputs.xml",
            "loc=\"STR_ACME_CORE_INPUT_OPEN_COMMAND\" />",
            "loc=\"STR_ACME_MISSING\r\nNAME\" />",
        )],
        &[],
    );
    assert_eq!(code(&out), Some(1), "{}{}", stdout(&out), stderr(&out));
    assert!(
        stdout(&out).contains("STR_ACME_MISSING NAME is in no stringtable"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn with_no_folder_it_checks_the_current_one() {
    let scratch = tree();
    let out = Command::new(env!("CARGO_BIN_EXE_checks"))
        .arg("check-inputs")
        .current_dir(scratch.path())
        .output()
        .unwrap();
    assert_eq!(code(&out), Some(0), "{}{}", stdout(&out), stderr(&out));
    assert!(
        stdout(&out).contains("OK: 2 inputs files, 2 actions, 3 scripts"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn only_utf8_and_ascii_inputs_files_are_read() {
    let xml = "mods/AcmeSort/Data/inputs.xml";
    let cases: &[(&[Edit], &str)] = &[
        (
            &[(xml, "encoding=\"UTF-8\"", "encoding=\"UTF-9\"")],
            "is not valid XML: unsupported encoding UTF-9 (inputs files are UTF-8): line 1, column 30",
        ),
        (
            &[(xml, "encoding=\"UTF-8\"", "encoding=\"ISO-8859-1\"")],
            "is not valid XML: unsupported encoding ISO-8859-1 (inputs files are UTF-8): line 1, column 30",
        ),
        (
            &[
                (xml, "encoding=\"UTF-8\"", "encoding=\"US-ASCII\""),
                (xml, "<preset>", "<preset><!-- é -->"),
            ],
            "is not valid XML: not ASCII: line 11",
        ),
    ];
    for (edits, expected) in cases {
        let out = check(edits, &[]);
        assert_eq!(code(&out), Some(1), "{}{}", stdout(&out), stderr(&out));
        assert!(stdout(&out).contains(expected), "{}", stdout(&out));
    }
    let ascii = check(&[(xml, "encoding=\"UTF-8\"", "encoding=\"us-ascii\"")], &[]);
    assert_eq!(code(&ascii), Some(0), "{}", stdout(&ascii));
    let bom = check(
        &[
            (xml, "<?xml version", "\u{feff}<?xml version"),
            (xml, "encoding=\"UTF-8\"", "encoding=\"US-ASCII\""),
        ],
        &[],
    );
    assert_eq!(code(&bom), Some(0), "{}", stdout(&bom));
    let earlier = check(
        &[
            (xml, "encoding=\"UTF-8\"", "encoding=\"US-ASCII\""),
            (xml, "<inputs>", "<inputs></actions>"),
            (xml, "<preset>", "<preset><!-- é -->"),
        ],
        &[],
    );
    assert_eq!(code(&earlier), Some(1), "{}", stdout(&earlier));
    assert!(
        stdout(&earlier).contains("mismatched tag: line 3"),
        "{}",
        stdout(&earlier)
    );
}
