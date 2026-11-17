use crate::enforce::{
    self, after_word_or_dot, anywhere, bare_word, bounded_captures, call_args, line_of,
    resolve_name, script_texts, Name, Names, ScriptText,
};
use crate::family;
use crate::repo;
use regex::Regex;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

const FRAME_ROOTS: [&str; 7] = [
    "OnUpdate",
    "OnTick",
    "EOnFrame",
    "EOnPostFrame",
    "OnUpdateTimer",
    "CommandHandler",
    "OnScheduledTick",
];
const HANDLER_FALLBACK: &str = "OnRPC";
const FILE_WORDS: [&str; 14] = [
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
    "FileExist",
];
const CAPS: [&str; 3] = ["FitsString", "FitsCount", "FitsRange"];
const SKIPPED: [&str; 9] = [
    "if", "for", "foreach", "while", "switch", "catch", "return", "new", "delete",
];
const NUMBERS: [&str; 3] = ["int", "float", "bool"];

static CONSTANT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\bconst\s+string\s+([A-Za-z_]\w*)\s*=\s*"([^"\n]*)"\s*;"#).unwrap()
});
static CALL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:([A-Za-z_]\w*)\s*\.\s*)?([A-Za-z_]\w*)\s*\(").unwrap());
static CLASS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bclass\s+([A-Za-z_]\w*)").unwrap());
static PARAM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bParam\d+\s*<((?:[^<>]|<[^<>]*>)*)>\s+([A-Za-z_]\w*)").unwrap());
static PLAIN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(?:int|float|bool)\s+([A-Za-z_]\w*)\s*[;=]").unwrap());
static READ: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\.\s*Read\s*\(\s*([A-Za-z_]\w*)\s*\)").unwrap());
static LOOP: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b(while|for)\s*\(").unwrap());
static NAME_END: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"([A-Za-z_]\w*)\s*$").unwrap());
static CAP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!("(?:{})", CAPS.join("|"))).unwrap());

struct Method {
    name: String,
    start: usize,
    end: usize,
}

type Methods = HashMap<String, Vec<Method>>;

struct Source {
    rel: String,
    code: String,
    mask: String,
    methods: Methods,
    unclosed: Vec<(String, usize)>,
    constants: Names,
    classes: HashSet<String>,
}

pub fn run(args: &[String]) -> u8 {
    let parsed = match enforce::parse_options(
        args,
        "usage: checks check-perf-static [--root ROOT]",
        &["--root"],
        0,
    ) {
        Ok(parsed) => parsed,
        Err(code) => return code,
    };
    let root = parsed
        .value("--root")
        .map_or_else(repo::root, PathBuf::from);
    if !root.is_dir() {
        println!("cannot run: root not found: {}", root.display());
        return 2;
    }
    let family = match family::load_or_report(&root) {
        Ok(family) => family,
        Err(code) => return code,
    };
    if let Some(code) = enforce::skip_without_mods(&root) {
        return code;
    }
    let files = match sources(&root) {
        Ok(files) => files,
        Err(error) => {
            println!("cannot run: {error}");
            return 2;
        }
    };
    let (analysis, frames, handlers) = analyse(
        &files,
        &file_word(&family.core),
        &enforce::rpc_register(&family.core),
    );
    let odd: Vec<String> = files
        .iter()
        .flat_map(|file| {
            enforce::odd_lines(&file.code, &file.mask)
                .into_iter()
                .map(|line| {
                    format!(
                        "{}:{line}: a character outside ASCII or from 0x1c to 0x1f in code",
                        file.rel
                    )
                })
        })
        .collect();
    let problems: Vec<String> = odd.into_iter().chain(analysis).collect();
    if let Some(code) = super::failed(&problems) {
        return code;
    }
    println!(
        "OK: {}, {}",
        enforce::count(frames, "frame root", "frame roots"),
        enforce::count(handlers, "handler root", "handler roots")
    );
    0
}

fn sources(root: &Path) -> Result<Vec<Source>, String> {
    let scripts = script_texts(
        &root.join("mods"),
        &root.display().to_string(),
        false,
        false,
    )?;
    Ok(scripts
        .texts
        .into_iter()
        .map(
            |ScriptText {
                 rel, code, mask, ..
             }| {
                let (methods, unclosed) = methods(&mask);
                Source {
                    rel,
                    constants: CONSTANT
                        .captures_iter(&code)
                        .map(|caps| (caps[1].to_string(), Some(Name::Text(caps[2].to_string()))))
                        .collect(),
                    classes: CLASS
                        .captures_iter(&mask)
                        .map(|caps| caps[1].to_string())
                        .collect(),
                    code,
                    mask,
                    methods,
                    unclosed,
                }
            },
        )
        .collect())
}

// The methods of a file by name, and the ones whose braces never close, with their open brace.
fn methods(mask: &str) -> (Methods, Vec<(String, usize)>) {
    let mut found: Methods = HashMap::new();
    let mut unclosed = Vec::new();
    let mut index = 0;
    while let Some(offset) = mask[index..].find('{') {
        let open = index + offset;
        index = open + 1;
        let before = &mask[..open];
        let start = [before.rfind(';'), before.rfind('}'), before.rfind('{')]
            .into_iter()
            .flatten()
            .max()
            .map_or(0, |at| at + 1);
        let Some(name) = method_name(mask, start, open) else {
            continue;
        };
        if SKIPPED.contains(&name.as_str()) {
            continue;
        }
        match enforce::block(mask, open) {
            Some(end) => found.entry(name.clone()).or_default().push(Method {
                name,
                start: open + 1,
                end,
            }),
            None => unclosed.push((name, open)),
        }
    }
    (found, unclosed)
}

// The name before the parameter list that closes just before open, when that list is what the
// head of the definition ends with.
fn method_name(mask: &str, start: usize, open: usize) -> Option<String> {
    let head = mask[start..open].trim_end();
    if !head.ends_with(')') {
        return None;
    }
    let mut depth = 0;
    for (index, c) in head.char_indices().rev() {
        match c {
            ')' => depth += 1,
            '(' => {
                depth -= 1;
                if depth == 0 {
                    return NAME_END
                        .captures(&head[..index])
                        .map(|caps| caps[1].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

// The methods a frame or handler reaches inside its own file: bare calls, calls through this, and
// calls through a class declared in the file. Breadth first, each body read once.
fn reach(source: &Source, root: &Method) -> Vec<Method> {
    let mut found = Vec::new();
    let mut seen = HashSet::new();
    let mut queue = VecDeque::from([clone_method(root)]);
    while let Some(method) = queue.pop_front() {
        if !seen.insert(method.start) {
            continue;
        }
        for caps in bounded_captures(
            &CALL,
            &source.mask,
            method.start,
            method.end,
            after_word_or_dot,
        ) {
            let owner = caps.get(1).map(|owner| owner.as_str());
            let name = &caps[2];
            let through_this_or_class =
                owner.is_none_or(|owner| owner == "this" || source.classes.contains(owner));
            if !SKIPPED.contains(&name) && through_this_or_class {
                queue.extend(
                    source
                        .methods
                        .get(name)
                        .into_iter()
                        .flatten()
                        .map(clone_method),
                );
            }
        }
        found.push(method);
    }
    found
}

fn report(
    source: &Source,
    root: &str,
    position: usize,
    word: &str,
    text: &str,
    seen: &mut HashSet<(String, usize, String)>,
    problems: &mut Vec<String>,
) {
    if !seen.insert((source.rel.clone(), position, word.to_string())) {
        return;
    }
    problems.push(format!(
        "{}:{}: {root} {text}",
        source.rel,
        line_of(&source.code, position)
    ));
}

fn empty_condition(mask: &str, start: usize, end: usize) -> bool {
    let mut depth = 0;
    let mut cuts = Vec::new();
    for (offset, byte) in mask.as_bytes()[start..end].iter().enumerate() {
        match byte {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b';' if depth == 0 => cuts.push(start + offset),
            _ => {}
        }
    }
    cuts.len() >= 2 && matches!(mask[cuts[0] + 1..cuts[1]].trim(), "" | "true" | "1")
}

fn loop_problems(
    source: &Source,
    root: &str,
    reached: &[Method],
    seen: &mut HashSet<(String, usize, String)>,
    problems: &mut Vec<String>,
) {
    for method in reached {
        for caps in bounded_captures(&LOOP, &source.mask, method.start, method.end, anywhere) {
            let whole = caps.get(0).unwrap();
            if &caps[1] == "while" {
                report(
                    source,
                    root,
                    whole.start(),
                    "while",
                    "holds a while loop: a frame body has no unbounded loop",
                    seen,
                    problems,
                );
                continue;
            }
            let Some(close) = enforce::paren(&source.mask, whole.end() - 1) else {
                continue;
            };
            if !empty_condition(&source.mask, whole.end(), close) {
                continue;
            }
            report(
                source,
                root,
                whole.start(),
                "for",
                "holds a for with no condition: a frame body has no unbounded loop",
                seen,
                problems,
            );
        }
    }
}

fn top_level_kinds(text: &str) -> Vec<&str> {
    let mut kinds = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    for (index, c) in text.char_indices() {
        match c {
            '<' => depth += 1,
            '>' => depth -= 1,
            ',' if depth == 0 => {
                kinds.push(text[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    kinds.push(text[start..].trim());
    kinds
}

// The names a handler declares as a number: a bare int, float or bool, or a ParamN of only numbers.
fn numeric_names(source: &Source, reached: &[Method]) -> HashSet<String> {
    let mut names = HashSet::new();
    for method in reached {
        let body = &source.mask[method.start..method.end];
        for caps in PLAIN.captures_iter(body) {
            names.insert(caps[1].to_string());
        }
        for caps in PARAM.captures_iter(body) {
            if top_level_kinds(&caps[1])
                .iter()
                .all(|kind| NUMBERS.contains(kind))
            {
                names.insert(caps[2].to_string());
            }
        }
    }
    names
}

// Anything not provably a number counts, so a read this can't type fails rather than passes.
fn payload_problems(
    source: &Source,
    root: &str,
    reached: &[Method],
    seen: &mut HashSet<(String, usize, String)>,
    problems: &mut Vec<String>,
) {
    let capped = reached.iter().any(|method| {
        !bounded_captures(&CAP, &source.mask, method.start, method.end, bare_word).is_empty()
    });
    if capped {
        return;
    }
    let numbers = numeric_names(source, reached);
    for method in reached {
        for caps in bounded_captures(&READ, &source.mask, method.start, method.end, anywhere) {
            if numbers.contains(&caps[1]) {
                continue;
            }
            report(
                source,
                root,
                caps.get(0).unwrap().start(),
                "Read",
                "reads a payload with no FitsString, FitsCount or FitsRange: a handler caps what it reads",
                seen,
                problems,
            );
            return;
        }
    }
}

fn file_word(core: &str) -> Regex {
    // Longest first, so a word that is the start of another is tried after it.
    let mut words: Vec<String> = FILE_WORDS.iter().map(|word| word.to_string()).collect();
    words.push(format!("{core}Files"));
    words.sort_by_key(|word| std::cmp::Reverse(word.len()));
    Regex::new(&format!("(?:{})", words.join("|"))).unwrap()
}

fn file_problems(
    source: &Source,
    root: &str,
    reached: &[Method],
    file_word: &Regex,
    seen: &mut HashSet<(String, usize, String)>,
    problems: &mut Vec<String>,
) {
    for method in reached {
        for caps in bounded_captures(file_word, &source.mask, method.start, method.end, bare_word) {
            let whole = caps.get(0).unwrap();
            report(
                source,
                root,
                whole.start(),
                whole.as_str(),
                &format!(
                    "reaches {}: no file work in a frame or an RPC handler",
                    whole.as_str()
                ),
                seen,
                problems,
            );
        }
    }
}

fn registered_handlers(
    files: &[Source],
    register: &Regex,
    problems: &mut Vec<String>,
) -> Vec<String> {
    let mut constants = HashMap::new();
    for source in files {
        constants.extend(source.constants.clone());
    }
    let mut names = Vec::new();
    for source in files {
        for caps in bounded_captures(
            register,
            &source.mask,
            0,
            source.mask.len(),
            after_word_or_dot,
        ) {
            let whole = caps.get(0).unwrap();
            let args = call_args(&source.code, &source.mask, whole.end() - 1);
            match args
                .get(1)
                .and_then(|arg| resolve_name(arg, Some(&constants)))
            {
                Some(Name::Text(name)) => names.push(name),
                _ => problems.push(format!(
                    "{}:{}: cannot tell which handler this Register names: use a literal or a const",
                    source.rel,
                    line_of(&source.code, whole.start())
                )),
            }
        }
    }
    names
}

// Every handler a frame or RPC path can reach: each OnRPC, and each void named by a Register.
fn handler_roots(
    files: &[Source],
    register: &Regex,
    problems: &mut Vec<String>,
) -> Vec<(usize, Method)> {
    let mut roots = Vec::new();
    for (file, source) in files.iter().enumerate() {
        for method in source.methods.get(HANDLER_FALLBACK).into_iter().flatten() {
            roots.push((file, clone_method(method)));
        }
    }
    let mut done = HashSet::new();
    for name in registered_handlers(files, register, problems) {
        if !done.insert(name.clone()) {
            continue;
        }
        let owners: Vec<usize> = (0..files.len())
            .filter(|&file| files[file].methods.contains_key(&name))
            .collect();
        if owners.is_empty() {
            problems.push(format!(
                "{name} is registered and defined nowhere under mods/"
            ));
        }
        for file in owners {
            for method in &files[file].methods[&name] {
                roots.push((file, clone_method(method)));
            }
        }
    }
    roots
}

fn clone_method(method: &Method) -> Method {
    Method {
        name: method.name.clone(),
        start: method.start,
        end: method.end,
    }
}

fn analyse(files: &[Source], file_word: &Regex, register: &Regex) -> (Vec<String>, usize, usize) {
    let mut problems = Vec::new();
    let mut seen = HashSet::new();
    let mut frames = 0;
    for source in files {
        for (name, position) in &source.unclosed {
            problems.push(format!(
                "{}:{}: {name} has braces that never close, so its body can't be read",
                source.rel,
                line_of(&source.code, *position)
            ));
        }
        for name in FRAME_ROOTS {
            for root in source.methods.get(name).into_iter().flatten() {
                frames += 1;
                let reached = reach(source, root);
                file_problems(
                    source,
                    &root.name,
                    &reached,
                    file_word,
                    &mut seen,
                    &mut problems,
                );
                loop_problems(source, &root.name, &reached, &mut seen, &mut problems);
            }
        }
    }
    let handlers = handler_roots(files, register, &mut problems);
    for (file, root) in &handlers {
        let source = &files[*file];
        let reached = reach(source, root);
        file_problems(
            source,
            &root.name,
            &reached,
            file_word,
            &mut seen,
            &mut problems,
        );
        payload_problems(source, &root.name, &reached, &mut seen, &mut problems);
    }
    (problems, frames, handlers.len())
}
