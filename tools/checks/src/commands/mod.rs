pub mod check_boundaries;
pub mod check_dependencies;
pub mod check_inputs;
pub mod check_modded;
pub mod check_pbo;
pub mod check_perf_static;
pub mod check_reference;
pub mod family;
pub mod prune_server_logs;
pub mod sync_version;

pub struct Command {
    pub name: &'static str,
    pub about: &'static str,
    pub run: fn(&[String]) -> u8,
}

// One line per tool, in name order; each tool lives in its own module here.
pub const COMMANDS: &[Command] = &[
    Command {
        name: "check-boundaries",
        about: "check that each mod's code names only its own prefix, Core and its contracts",
        run: check_boundaries::run,
    },
    Command {
        name: "check-dependencies",
        about: "check that each mod's config lists only the addons it may and that its patch classes are its own",
        run: check_dependencies::run,
    },
    Command {
        name: "check-reference",
        about: "check that no mod uses a name declared by the reference mod, the rule that a reference is never a source",
        run: check_reference::run,
    },
    Command {
        name: "check-inputs",
        about: "check the inputs files: their actions, sorting, presets, loc keys and the keys scripts look up",
        run: check_inputs::run,
    },
    Command {
        name: "check-modded",
        about: "check that every modded class has an allow-list row, and that each patch file is named and headed",
        run: check_modded::run,
    },
    Command {
        name: "check-pbo",
        about: "fail when a mod would pack its docs, tools or tests",
        run: check_pbo::run,
    },
    Command {
        name: "check-perf-static",
        about: "check that frame callbacks and RPC handlers do no file work, hold no unbounded loop and cap what they read",
        run: check_perf_static::run,
    },
    Command {
        name: "family",
        about: "show the family's values from tools/family.conf that the checks read",
        run: family::run,
    },
    Command {
        name: "prune-server-logs",
        about: "keep the logs of the last server runs and remove check folders",
        run: prune_server_logs::run,
    },
    Command {
        name: "sync-version",
        about: "check or set the mod versions in step with VERSION",
        run: sync_version::run,
    },
];

pub fn find(name: &str) -> Option<&'static Command> {
    COMMANDS.iter().find(|command| command.name == name)
}

// Prints each problem, then the count line, and gives 1; an empty list gives nothing.
pub fn failed(problems: &[String]) -> Option<u8> {
    if problems.is_empty() {
        return None;
    }
    for problem in problems {
        println!("{problem}");
    }
    println!("FAILED: {} problems", problems.len());
    Some(1)
}

pub fn listing() -> String {
    let mut text = String::from("commands:");
    for command in COMMANDS {
        text.push_str(&format!("\n  {:<22}{}", command.name, command.about));
    }
    text
}
