mod common;

use common::{checks, code, stdout, Scratch, ACME_FAMILY};

fn show(conf: Option<&str>) -> std::process::Output {
    let scratch = Scratch::new("family");
    if let Some(text) = conf {
        scratch.write("tools/family.conf", text);
    }
    checks(&["family", "--root", scratch.path().to_str().unwrap()])
}

#[test]
fn a_full_file_is_shown_back_normalised() {
    let out = show(Some(
        "# Acme's values\n\
         prefix = Acme\n\
         core = AcmeCore\n\
         \n\
         framework = Kit KIT_Scripts\n\
         framework = Forge   FORGE_GUI FORGE_Scripts\n\
         reference = /refs/Other\n\
         vanilla = /refs/dayz\n",
    ));
    assert_eq!(code(&out), Some(0), "{}", stdout(&out));
    assert_eq!(
        stdout(&out),
        "prefix = Acme\ncore = AcmeCore\nframework = Kit KIT_Scripts\n\
         framework = Forge FORGE_GUI FORGE_Scripts\nreference = /refs/Other\nvanilla = /refs/dayz\n"
    );
}

#[test]
fn a_home_path_is_expanded() {
    let home = std::env::var("HOME").unwrap();
    let out = show(Some(
        "prefix = Acme\ncore = AcmeCore\nvanilla = ~/refs/dayz\n",
    ));
    assert_eq!(code(&out), Some(0), "{}", stdout(&out));
    assert!(
        stdout(&out).contains(&format!("vanilla = {home}/refs/dayz\n")),
        "{}",
        stdout(&out)
    );
}

#[test]
fn broken_files_cannot_run() {
    let cases = [
        ("no file", None, "tools/family.conf not found"),
        (
            "no prefix",
            Some("core = AcmeCore\n"),
            "tools/family.conf: no prefix",
        ),
        (
            "no core",
            Some("prefix = Acme\n"),
            "tools/family.conf: no core",
        ),
        (
            "line without =",
            Some("prefix Acme\n"),
            "tools/family.conf:1: not key = value",
        ),
        (
            "unknown key",
            Some("prefix = Acme\ncore = AcmeCore\ncolour = red\n"),
            "tools/family.conf:3: unknown key colour",
        ),
        (
            "key twice",
            Some("prefix = Acme\nprefix = Acme\ncore = AcmeCore\n"),
            "tools/family.conf:2: prefix given twice",
        ),
        (
            "prefix not one token",
            Some("prefix = Ac_me\ncore = Ac_meCore\n"),
            "tools/family.conf:1: prefix must be letters and digits, starting with a capital",
        ),
        (
            "core outside the prefix",
            Some("prefix = Acme\ncore = OtherCore\n"),
            "tools/family.conf:2: core must be the prefix Acme followed by letters and digits",
        ),
        (
            "framework with no addon",
            Some("prefix = Acme\ncore = AcmeCore\nframework = Kit\n"),
            "tools/family.conf:3: framework needs a name and at least one addon",
        ),
        (
            "empty path",
            Some("prefix = Acme\ncore = AcmeCore\nvanilla =\n"),
            "tools/family.conf:3: vanilla is empty",
        ),
    ];
    for (name, conf, needle) in cases {
        let out = show(conf);
        assert_eq!(code(&out), Some(2), "{name}: {}", stdout(&out));
        assert_eq!(stdout(&out), format!("cannot run: {needle}\n"), "{name}");
    }
}

#[test]
fn the_acme_values_are_shown() {
    let out = show(Some(ACME_FAMILY));
    assert_eq!(code(&out), Some(0), "{}", stdout(&out));
    assert_eq!(
        stdout(&out),
        "prefix = Acme\ncore = AcmeCore\nframework = Kit KIT_Scripts KIT_GUI\n"
    );
}

#[test]
fn awkward_files() {
    let scratch = Scratch::new("family-awkward");
    let root = scratch.path().to_str().unwrap().to_string();
    let run = |bytes: &[u8], home: Option<&str>| {
        std::fs::create_dir_all(scratch.join("tools")).unwrap();
        std::fs::write(scratch.join("tools/family.conf"), bytes).unwrap();
        let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_checks"));
        command.args(["family", "--root", &root]);
        match home {
            Some(home) => command.env("HOME", home),
            None => command.env_remove("HOME"),
        };
        command.output().unwrap()
    };
    let base = "prefix = Acme\r\ncore = AcmeCore\r\n";
    let cases = [
        (
            "byte order mark",
            [b"\xEF\xBB\xBF".as_slice(), base.as_bytes()].concat(),
            Some("/h"),
            0,
            "prefix = Acme\n".into(),
        ),
        (
            "path with a space and =",
            format!("{base}vanilla = /refs/a b=c\r\n").into_bytes(),
            Some("/h"),
            0,
            "vanilla = /refs/a b=c\n".into(),
        ),
        (
            "relative path is under the root",
            format!("{base}vanilla = refs/dayz\n").into_bytes(),
            Some("/h"),
            0,
            format!("vanilla = {root}/refs/dayz\n"),
        ),
        (
            "home alone and doubled slash",
            format!("{base}reference = ~\nvanilla = ~//x\n").into_bytes(),
            Some("/h"),
            0,
            "reference = /h\nvanilla = /h/x\n".into(),
        ),
        (
            "home unset",
            format!("{base}vanilla = ~/x\n").into_bytes(),
            None,
            2,
            "cannot run: tools/family.conf:3: vanilla starts with ~ but HOME is not set\n".into(),
        ),
        (
            "framework twice",
            format!("{base}framework = Kit A\nframework = Kit B\n").into_bytes(),
            Some("/h"),
            2,
            "cannot run: tools/family.conf:4: framework Kit given twice\n".into(),
        ),
        (
            "not UTF-8",
            b"prefix = Acme\xff\n".to_vec(),
            Some("/h"),
            2,
            "cannot run: cannot read tools/family.conf: stream did not contain valid UTF-8\n"
                .into(),
        ),
    ];
    for (name, bytes, home, exit, needle) in cases {
        let out = run(&bytes, home);
        assert_eq!(code(&out), Some(exit), "{name}: {}", stdout(&out));
        assert!(stdout(&out).contains(&needle), "{name}: {}", stdout(&out));
    }
}
