use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

mod common;
use common::{
    checks, expect, stdout, write_family, Scratch, ACME_FAMILY, ACME_FAMILY_TWO_FRAMEWORKS,
};

const TICKER: &str = "mods/AcmeCore/Scripts/3_Game/AcmeCoreTicker.c";
const PING: &str = "mods/AcmeCore/Scripts/5_Mission/AcmeCorePing.c";
const PERM: &str = "mods/AcmeCore/Scripts/5_Mission/AcmeCorePerm.c";
const ONRPC_FILE: &str = "mods/AcmeCore/Scripts/4_World/AcmeCoreOld.c";

const ONRPC: &str = "class AcmeCoreOld\n{\n    void OnRPC(CallType type, ParamsReadContext ctx, PlayerIdentity sender, Object target)\n    {\n        Param1<string> text = new Param1<string>(\"\");\n        if (!ctx.Read(text))\n        {\n            return;\n        }\n\n        string value = text.param1;\n    }\n}\n";

fn numbers_handler(declare: &str, read: &str) -> String {
    format!(
        "class AcmeCoreNumbers\n{{\n    void Start()\n    {{\n        AcmeCoreRPC.Register(\"M\", \"RPC_Numbers\", this);\n    }}\n\n    void RPC_Numbers(CallType type, ParamsReadContext ctx, PlayerIdentity sender, Object target)\n    {{\n        {declare}\n        ctx.Read({read});\n    }}\n}}\n"
    )
}

fn follow_file(call: &str) -> String {
    format!(
        "class AcmeCoreFollow\n{{\n    void OnTick(float timeslice)\n    {{\n        {call}\n    }}\n\n    void Helper()\n    {{\n        ReadFile(\"x\", \"y\");\n    }}\n}}\n"
    )
}

fn good() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/check_perf_static/good")
}

// A copy of the good tree with the given edits and new files, run through the check.
fn check(edits: &[(&str, &str, &str)], adds: &[(&str, &str)]) -> Output {
    let scratch = Scratch::copy_with("perf-static", &good(), edits, adds, &[]);
    write_family(&scratch, ACME_FAMILY_TWO_FRAMEWORKS);
    checks(&[
        "check-perf-static",
        "--root",
        scratch.path().to_str().unwrap(),
    ])
}

#[test]
fn the_tree_passes() {
    let done = check(&[], &[]);
    assert_eq!(done.status.code(), Some(0), "{}", stdout(&done));
    assert!(
        stdout(&done).contains("OK: 1 frame root, 2 handler roots"),
        "{}",
        stdout(&done)
    );
}

#[test]
fn every_frame_callback_is_a_root() {
    let cases = [
        ("OnUpdate", ""),
        ("OnTick", ""),
        ("EOnFrame", ""),
        ("EOnPostFrame", ""),
        ("OnUpdateTimer", "int timer"),
        ("CommandHandler", "string name"),
        ("OnScheduledTick", "int tick"),
    ];
    for (name, params) in cases {
        let path = format!("mods/AcmeCore/Scripts/3_Game/AcmeCore{name}.c");
        let text = format!(
            "class AcmeCore{name}\n{{\n    void {name}({params})\n    {{\n        ReadFile(\"x\", \"y\");\n    }}\n}}\n"
        );
        expect(
            name,
            &check(&[], &[(&path, &text)]),
            1,
            &format!("{name} reaches ReadFile"),
        );
    }
}

#[test]
fn every_file_word_is_a_file_word() {
    let words = [
        "OpenFile",
        "CloseFile",
        "FPrint",
        "FPrintln",
        "FGets",
        "ReadFile",
        "CopyFile",
        "DeleteFile",
        "MakeDirectory",
        "FindFile",
        "FindNextFile",
        "FileSerializer",
        "JsonFileLoader",
        "AcmeCoreFiles",
        "FileExist",
    ];
    for word in words {
        let text = format!(
            "class AcmeCoreWord\n{{\n    void OnTick(float timeslice)\n    {{\n        {word}(\"x\");\n    }}\n}}\n"
        );
        expect(
            word,
            &check(
                &[],
                &[("mods/AcmeCore/Scripts/3_Game/AcmeCoreWord.c", &text)],
            ),
            1,
            &format!("OnTick reaches {word}"),
        );
    }
}

#[test]
fn the_file_word_is_the_family_core_files() {
    let cases = [
        ("AcmeCoreFiles", 1, "OnTick reaches AcmeCoreFiles"),
        ("OutsideCoreFiles", 0, "OK:"),
    ];
    for (word, exit, needle) in cases {
        let text = format!(
            "class AcmeCoreWord\n{{\n    void OnTick(float timeslice)\n    {{\n        {word}(\"x\");\n    }}\n}}\n"
        );
        let scratch = Scratch::copy_with(
            "perf-static-family",
            &good(),
            &[],
            &[("mods/AcmeCore/Scripts/3_Game/AcmeCoreWord.c", &text)],
            &[],
        );
        write_family(&scratch, ACME_FAMILY);
        let done = checks(&[
            "check-perf-static",
            "--root",
            scratch.path().to_str().unwrap(),
        ]);
        expect(word, &done, exit, needle);
    }
}

#[test]
fn each_problem_is_reported_with_its_place() {
    let frame_read =
        format!("{TICKER}:6: OnTick reaches ReadFile: no file work in a frame or an RPC handler");
    expect(
        "a file word in a frame body",
        &check(
            &[(
                TICKER,
                "DoWork();",
                "DoWork();\n        ReadFile(\"x\", \"y\");",
            )],
            &[],
        ),
        1,
        &frame_read,
    );
    expect(
        "a file word two calls away",
        &check(
            &[(
                TICKER,
                "total = total + i;",
                "total = total + i;\n            FileExist(\"x\");",
            )],
            &[],
        ),
        1,
        "OnTick reaches FileExist",
    );
    expect(
        "a file word in a handler body",
        &check(
            &[(
                PING,
                "string line = field.param1;",
                "string line = field.param1;\n        ReadFile(\"x\", \"y\");",
            )],
            &[],
        ),
        1,
        "RPC_Ping reaches ReadFile: no file work in a frame or an RPC handler",
    );
    expect(
        "a while loop in a frame",
        &check(
            &[(TICKER, "for (int i = 0; i < 10; i++)", "while (true)")],
            &[],
        ),
        1,
        "OnTick holds a while loop: a frame body has no unbounded loop",
    );
    expect(
        "a for loop with no condition in a frame",
        &check(
            &[(
                TICKER,
                "for (int i = 0; i < 10; i++)",
                "for (int i = 0; ; i++)",
            )],
            &[],
        ),
        1,
        "OnTick holds a for with no condition: a frame body has no unbounded loop",
    );
    expect(
        "a handler reading a string field with no cap",
        &check(
            &[(
                PING,
                "if (!AcmeCoreRPC.FitsString(\"line\", line, 64))",
                "if (line == \"\")",
            )],
            &[],
        ),
        1,
        "RPC_Ping reads a payload with no FitsString, FitsCount or FitsRange",
    );
    expect(
        "an OnRPC handler reading a string field with no cap",
        &check(&[], &[(ONRPC_FILE, ONRPC)]),
        1,
        "OnRPC reads a payload with no FitsString, FitsCount or FitsRange",
    );
    expect(
        "an array payload read with no cap",
        &check(
            &[
                (
                    PERM,
                    "Param2<string, int> field = new Param2<string, int>(\"\", 0);",
                    "Param1<array<string>> names = new Param1<array<string>>(null);",
                ),
                (
                    PERM,
                    "int role = field.param2;",
                    "array<string> roles = names.param1;",
                ),
                (
                    PERM,
                    "if (!AcmeCoreRPC.FitsRange(\"perm_role\", role, 0, 7))",
                    "if (false)",
                ),
            ],
            &[],
        ),
        1,
        "RPC_Perm reads a payload with no FitsString, FitsCount or FitsRange",
    );
}

#[test]
fn a_file_word_in_a_string_or_a_comment_is_not_counted() {
    expect(
        "in a string",
        &check(
            &[(
                TICKER,
                "int count = CountThings();",
                "string note = \"ReadFile\";",
            )],
            &[],
        ),
        0,
        "OK",
    );
    expect(
        "in a comment",
        &check(
            &[(
                TICKER,
                "int count = CountThings();",
                "// ReadFile\n        int count = CountThings();",
            )],
            &[],
        ),
        0,
        "OK",
    );
}

#[test]
fn a_while_loop_in_a_handler_is_not_flagged() {
    let loop_body = "string line = field.param1;\n        while (true)\n        {\n            break;\n        }";
    expect(
        "a while loop in a handler",
        &check(&[(PING, "string line = field.param1;", loop_body)], &[]),
        0,
        "OK",
    );
}

#[test]
fn a_for_over_a_count_is_not_flagged() {
    expect(
        "a for over a count",
        &check(
            &[(
                TICKER,
                "for (int i = 0; i < 10; i++)",
                "for (int i = 0; i < items.Count(); i++)",
            )],
            &[],
        ),
        0,
        "OK",
    );
}

#[test]
fn a_handler_reading_only_numbers_needs_no_cap() {
    let cases = [
        (
            "a Param of numbers",
            "Param2<int, bool> field = new Param2<int, bool>(0, false);",
            "field",
        ),
        ("a bare int", "int field;", "field"),
    ];
    for (name, declare, read) in cases {
        let text = numbers_handler(declare, read);
        expect(
            name,
            &check(
                &[],
                &[("mods/AcmeCore/Scripts/4_World/AcmeCoreNumbers.c", &text)],
            ),
            0,
            "OK",
        );
    }
}

#[test]
fn an_uncapped_read_of_anything_else_fails() {
    let cases = [
        ("a bare string", "string field;", "field"),
        ("a bare array", "array<string> field;", "field"),
        ("auto", "auto field = new Param1<string>(\"\");", "field"),
        (
            "a nested generic",
            "Param2<map<int, int>, string> field;",
            "field",
        ),
        (
            "a string beside an int",
            "Param2<string, int> field = new Param2<string, int>(\"\", 0);",
            "field",
        ),
        ("a class field", "", "m_Field"),
    ];
    for (name, declare, read) in cases {
        let text = numbers_handler(declare, read);
        expect(
            name,
            &check(
                &[],
                &[("mods/AcmeCore/Scripts/4_World/AcmeCoreNumbers.c", &text)],
            ),
            1,
            "RPC_Numbers reads a payload with no",
        );
    }
}

#[test]
fn a_call_through_this_or_the_class_is_followed() {
    for (name, call) in [
        ("this", "this.Helper();"),
        ("the class", "AcmeCoreFollow.Helper();"),
    ] {
        let text = follow_file(call);
        expect(
            name,
            &check(
                &[],
                &[("mods/AcmeCore/Scripts/4_World/AcmeCoreFollow.c", &text)],
            ),
            1,
            "OnTick reaches ReadFile",
        );
    }
}

#[test]
fn a_call_on_another_object_is_not_followed() {
    let text = follow_file("m_Other.Helper();");
    expect(
        "another object",
        &check(
            &[],
            &[("mods/AcmeCore/Scripts/4_World/AcmeCoreFollow.c", &text)],
        ),
        0,
        "OK",
    );
}

#[test]
fn a_foreach_is_not_read_as_a_call() {
    let text = "class AcmeCoreLoop\n{\n    void OnTick(float timeslice)\n    {\n        foreach (int value : m_Values)\n        {\n            Print(value);\n        }\n    }\n}\nclass AcmeCoreLoopOther\n{\n    void Save()\n    {\n        foreach (string line : m_Lines)\n        {\n            FPrint(m_File, line);\n        }\n    }\n}\n";
    expect(
        "a foreach",
        &check(
            &[],
            &[("mods/AcmeCore/Scripts/4_World/AcmeCoreLoop.c", text)],
        ),
        0,
        "OK",
    );
}

#[test]
fn a_constant_true_for_condition_is_unbounded() {
    expect(
        "a for with true",
        &check(
            &[(
                TICKER,
                "for (int i = 0; i < 10; i++)",
                "for (int i = 0; true; i++)",
            )],
            &[],
        ),
        1,
        "OnTick holds a for with no condition",
    );
}

#[test]
fn a_register_that_cannot_be_resolved_fails() {
    let text = "class AcmeCoreLoose\n{\n    void Start()\n    {\n        AcmeCoreRPC.Register(\"M\", \"RPC_\" + \"Foo\", this);\n    }\n}\n";
    expect(
        "an expression for the name",
        &check(
            &[],
            &[("mods/AcmeCore/Scripts/4_World/AcmeCoreLoose.c", text)],
        ),
        1,
        "cannot tell which handler this Register names",
    );
}

#[test]
fn a_handler_named_by_a_constant_in_another_file_is_a_root() {
    let names = "class AcmeCoreNames\n{\n    static const string RPC_OVER = \"RPC_Over\";\n}\n";
    let start = "class AcmeCoreStart\n{\n    void Start()\n    {\n        AcmeCoreRPC.Register(\"M\", AcmeCoreNames.RPC_OVER, this);\n    }\n}\n";
    let handler = "class AcmeCoreOver\n{\n    void RPC_Over()\n    {\n        ReadFile(\"x\", \"y\");\n    }\n}\n";
    expect(
        "a constant named handler",
        &check(
            &[],
            &[
                ("mods/AcmeCore/Scripts/3_Game/AcmeCoreNames.c", names),
                ("mods/AcmeCore/Scripts/4_World/AcmeCoreStart.c", start),
                ("mods/AcmeCore/Scripts/4_World/AcmeCoreOver.c", handler),
            ],
        ),
        1,
        "RPC_Over reaches ReadFile",
    );
}

#[test]
fn a_registered_handler_defined_nowhere_fails() {
    let text = "class AcmeCoreLoose\n{\n    void Start()\n    {\n        AcmeCoreRPC.Register(\"M\", \"RPC_Gone\", this);\n    }\n}\n";
    expect(
        "a handler defined nowhere",
        &check(
            &[],
            &[("mods/AcmeCore/Scripts/4_World/AcmeCoreLoose.c", text)],
        ),
        1,
        "RPC_Gone is registered and defined nowhere",
    );
}

#[test]
fn a_method_whose_braces_never_close_fails() {
    let text = "class AcmeCoreOpen\n{\n    void OnTick(float timeslice)\n    {\n#ifdef A\n        if (a)\n        {\n#else\n        if (b)\n        {\n#endif\n    }\n}\n";
    expect(
        "braces that never close",
        &check(
            &[],
            &[("mods/AcmeCore/Scripts/4_World/AcmeCoreOpen.c", text)],
        ),
        1,
        "OnTick has braces that never close",
    );
}

#[test]
fn scripts_folders_in_both_spellings_are_scanned() {
    let text = "class AcmeCoreLower\n{\n    void OnTick(float timeslice)\n    {\n        ReadFile(\"x\", \"y\");\n    }\n}\n";
    expect(
        "a lower-case scripts folder",
        &check(
            &[],
            &[("mods/AcmeCore/scripts/3_Game/AcmeCoreLower.c", text)],
        ),
        1,
        "OnTick reaches ReadFile",
    );
}

#[test]
fn a_recursive_helper_does_not_hang() {
    let recursive = "void Recurse()\n    {\n        Recurse();\n    }\n\n    void DoWork()";
    expect(
        "a recursive helper",
        &check(
            &[
                (TICKER, "DoWork();", "Recurse();"),
                (TICKER, "void DoWork()", recursive),
            ],
            &[],
        ),
        0,
        "OK",
    );
}

#[test]
fn a_root_with_no_mods_cannot_run() {
    let scratch = Scratch::copy_of("perf-static", &good());
    write_family(&scratch, ACME_FAMILY_TWO_FRAMEWORKS);
    fs::remove_dir_all(scratch.path().join("mods")).unwrap();
    let done = checks(&[
        "check-perf-static",
        "--root",
        scratch.path().to_str().unwrap(),
    ]);
    expect("no mods", &done, 2, "no mods under");
}

#[test]
fn an_empty_mods_folder_is_skipped() {
    let scratch = Scratch::copy_of("perf-static", &good());
    write_family(&scratch, ACME_FAMILY_TWO_FRAMEWORKS);
    fs::remove_dir_all(scratch.path().join("mods")).unwrap();
    scratch.write("mods/.gitkeep", "");
    let done = checks(&[
        "check-perf-static",
        "--root",
        scratch.path().to_str().unwrap(),
    ]);
    expect("empty mods", &done, 0, "skipped: no mods under");
}

#[test]
fn a_mods_folder_that_cannot_be_searched_is_no_mods() {
    let root = Command::new("id").arg("-u").output().unwrap();
    if root.stdout == b"0\n" {
        return;
    }
    let scratch = Scratch::copy_of("perf-static", &good());
    write_family(&scratch, ACME_FAMILY_TWO_FRAMEWORKS);
    let mods = scratch.path().join("mods");
    fs::set_permissions(&mods, fs::Permissions::from_mode(0o644)).unwrap();
    let done = checks(&[
        "check-perf-static",
        "--root",
        scratch.path().to_str().unwrap(),
    ]);
    fs::set_permissions(&mods, fs::Permissions::from_mode(0o755)).unwrap();
    expect("unsearchable mods", &done, 2, "no mods under");
}

#[test]
fn a_character_outside_ascii_in_code_is_refused() {
    expect(
        "a combining mark before ReadFile",
        &check(
            &[(
                TICKER,
                "DoWork();",
                "DoWork(); x\u{0345}ReadFile(\"x\", \"y\");",
            )],
            &[],
        ),
        1,
        "outside ASCII or from 0x1c to 0x1f in code",
    );
    expect(
        "a combining mark before while",
        &check(
            &[(TICKER, "DoWork();", "DoWork(); \u{0301}while(true){}")],
            &[],
        ),
        1,
        "outside ASCII or from 0x1c to 0x1f in code",
    );
}

#[test]
fn a_control_character_from_0x1c_to_0x1f_is_refused() {
    expect(
        "a separator after a method's parameters",
        &check(
            &[(
                TICKER,
                "void OnTick(float timeslice)",
                "void OnTick(float timeslice)\u{1c}",
            )],
            &[],
        ),
        1,
        "outside ASCII or from 0x1c to 0x1f in code",
    );
}

#[test]
fn a_helper_shared_by_a_tick_and_a_handler_is_reported_once() {
    let text = "class AcmeCoreShared\n{\n    void OnTick(float timeslice)\n    {\n        Helper();\n    }\n\n    void Start()\n    {\n        AcmeCoreRPC.Register(\"M\", \"RPC_Shared\", this);\n    }\n\n    void RPC_Shared()\n    {\n        Helper();\n    }\n\n    void Helper()\n    {\n        ReadFile(\"x\", \"y\");\n    }\n}\n";
    let done = check(
        &[],
        &[("mods/AcmeCore/Scripts/4_World/AcmeCoreShared.c", text)],
    );
    expect("a shared helper", &done, 1, "OnTick reaches ReadFile");
    assert_eq!(
        stdout(&done).matches("ReadFile").count(),
        1,
        "{}",
        stdout(&done)
    );
}

#[test]
fn a_handler_registered_on_the_family_core_is_a_root() {
    let start = "class ZedCoreStart\n{\n    void Start()\n    {\n        ZedCoreRPC.Register(\"M\", \"RPC_Over\", this);\n    }\n}\n";
    let handler = "class ZedCoreOver\n{\n    void RPC_Over()\n    {\n        ReadFile(\"x\", \"y\");\n    }\n}\n";
    let scratch = Scratch::copy_with(
        "perf-static-zed",
        &good(),
        &[],
        &[
            ("mods/ZedCore/Scripts/4_World/ZedCoreStart.c", start),
            ("mods/ZedCore/Scripts/4_World/ZedCoreOver.c", handler),
        ],
        &[],
    );
    write_family(
        &scratch,
        "prefix = Zed\ncore = ZedCore\nframework = Kit KIT_Scripts KIT_GUI\n",
    );
    let out = checks(&[
        "check-perf-static",
        "--root",
        scratch.path().to_str().unwrap(),
    ]);
    expect(
        "a handler on the family core",
        &out,
        1,
        "RPC_Over reaches ReadFile",
    );
}
