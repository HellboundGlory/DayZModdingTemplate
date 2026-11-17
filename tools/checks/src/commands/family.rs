use crate::{enforce, family, repo};
use std::path::PathBuf;

pub fn run(args: &[String]) -> u8 {
    let parsed =
        match enforce::parse_options(args, "usage: checks family [--root ROOT]", &["--root"], 0) {
            Ok(parsed) => parsed,
            Err(code) => return code,
        };
    let root = parsed
        .value("--root")
        .map(PathBuf::from)
        .unwrap_or_else(repo::root);
    let family = match family::load_or_report(&root) {
        Ok(family) => family,
        Err(code) => return code,
    };
    println!("prefix = {}", family.prefix);
    println!("core = {}", family.core);
    for framework in &family.frameworks {
        println!(
            "framework = {} {}",
            framework.name,
            framework.addons.join(" ")
        );
    }
    if let Some(path) = &family.reference {
        println!("reference = {}", path.display());
    }
    if let Some(path) = &family.vanilla {
        println!("vanilla = {}", path.display());
    }
    0
}
