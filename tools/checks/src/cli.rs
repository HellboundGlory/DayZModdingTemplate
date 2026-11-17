// Argument parsing close to Python's argparse.

pub struct Opt {
    pub name: &'static str,
    pub values: usize,
}

pub struct Spec {
    pub command: &'static str,
    pub usage: &'static str,
    pub options: &'static [Opt],
    pub positional: Option<(&'static str, bool)>,
}

#[derive(Default)]
pub struct Parsed {
    given: Vec<(&'static str, Vec<String>)>,
    pub positionals: Vec<String>,
}

impl Parsed {
    pub fn flag(&self, name: &str) -> bool {
        self.given.iter().any(|(given, _)| *given == name)
    }

    pub fn value(&self, name: &str) -> Option<&str> {
        self.given
            .iter()
            .rev()
            .find(|(given, _)| *given == name)
            .and_then(|(_, values)| values.first())
            .map(String::as_str)
    }

    pub fn pair(&self, name: &str) -> Option<(&str, &str)> {
        let (_, values) = self.given.iter().rev().find(|(given, _)| *given == name)?;
        Some((values.first()?, values.get(1)?))
    }
}

// Err carries the exit code: 0 after help, 2 after a usage error.
pub fn parse(spec: &Spec, args: &[String]) -> Result<Parsed, u8> {
    let mut parsed = Parsed::default();
    let mut positional_only = false;
    let mut i = 0;
    while i < args.len() {
        let token = args[i].as_str();
        i += 1;
        if !positional_only && token == "--" {
            positional_only = true;
        } else if positional_only || !is_option(token) {
            parsed.positionals.push(token.to_string());
        } else if token == "-h" || token == "--help" {
            println!("usage: checks {} {}", spec.command, spec.usage);
            return Err(0);
        } else {
            let long = token.strip_prefix("--").unwrap_or(token);
            let (name, inline) = match long.split_once('=') {
                Some((name, value)) => (name, Some(value.to_string())),
                None => (long, None),
            };
            let Some(option) = spec.options.iter().find(|option| option.name == name) else {
                return Err(fail(spec, &format!("unrecognized arguments: {token}")));
            };
            let mut values = Vec::new();
            match (inline, option.values) {
                (Some(value), 1) => values.push(value),
                (Some(_), _) => {
                    return Err(fail(
                        spec,
                        &format!("argument --{name}: ignored explicit argument"),
                    ));
                }
                (None, count) => {
                    for _ in 0..count {
                        match args.get(i) {
                            Some(value) if !is_option(value) => {
                                values.push(value.clone());
                                i += 1;
                            }
                            _ => {
                                let expected = if count == 2 {
                                    "two arguments"
                                } else {
                                    "one argument"
                                };
                                return Err(fail(
                                    spec,
                                    &format!("argument --{name}: expected {expected}"),
                                ));
                            }
                        }
                    }
                }
            }
            parsed.given.push((option.name, values));
        }
    }
    match spec.positional {
        None => {
            if let Some(extra) = parsed.positionals.first() {
                return Err(fail(spec, &format!("unrecognized arguments: {extra}")));
            }
        }
        Some((name, required)) => {
            if parsed.positionals.len() > 1 {
                let extra = parsed.positionals[1].clone();
                return Err(fail(spec, &format!("unrecognized arguments: {extra}")));
            }
            if required && parsed.positionals.is_empty() {
                return Err(fail(
                    spec,
                    &format!("the following arguments are required: {name}"),
                ));
            }
        }
    }
    Ok(parsed)
}

// A usage error the command itself finds, reported like argparse's parser.error.
pub fn fail(spec: &Spec, message: &str) -> u8 {
    eprintln!("usage: checks {} {}", spec.command, spec.usage);
    eprintln!("checks {}: error: {message}", spec.command);
    2
}

fn is_option(token: &str) -> bool {
    token.starts_with('-') && token != "-" && !is_negative_number(token)
}

fn is_negative_number(token: &str) -> bool {
    let Some(rest) = token.strip_prefix('-') else {
        return false;
    };
    let digits = |text: &str| !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit());
    match rest.split_once('.') {
        None => digits(rest),
        Some((whole, fraction)) => (whole.is_empty() || digits(whole)) && digits(fraction),
    }
}
