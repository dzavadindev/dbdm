use crate::config_parser::Config;

mod config_parser;

fn main() {
    let mut pwd = match std::env::current_dir() {
        Ok(path) => path,
        Err(err) => {
            println!("Could not parse the {}", err.to_string());
            return;
        }
    };

    pwd.push("dbdm.conf");
    if !pwd.exists() {
        let mut path_str = pwd.clone();
        path_str.pop();
        println!(
            "dbdm.conf doesn exist in {}",
            path_str.to_str().expect("Can't parse dir path")
        );
        return;
    }

    let config = match config_parser::read_config(&pwd) {
        Ok(res) => res,
        Err(err) => {
            println!("Error in config:\n\n{}", err);
            return;
        }
    };

    let command = std::env::args().nth(1).unwrap_or(String::from("help"));

    match command.as_str() {
        "check" => check(&config),
        "sync" => sync(&config),
        "help" => help(),
        _ => help(),
    }
}

fn check(config: &Config) {
    for link in &config.links {
        // Get an absolute path to the files
        let from_full = std::fs::canonicalize(&link.from).unwrap_or_else(|_| link.from.clone());
        let to_full = std::fs::canonicalize(&link.to).unwrap_or_else(|_| link.to.clone());

        let is_match = match std::fs::read_link(&link.to) {
            Ok(target) => {
                let target_full = std::fs::canonicalize(&target).unwrap_or(target);
                target_full == from_full
            }
            Err(_) => false,
        };

        if is_match {
            println!(
                "\x1b[32m{} -> {}\x1b[0m",
                from_full.display(),
                to_full.display()
            );
        } else {
            println!(
                "\x1b[31m{} -> {}\x1b[0m",
                from_full.display(),
                to_full.display()
            );
        }
    }
}

fn sync(config: &Config) {
    println!("Not implemented");
}

fn help() {
    println!("dbdm - dotfile link manager");
    println!("\nUsage:");
    println!("  dbdm <command>");
    println!("\nCommands:");
    println!("  check   Validate config and planned links");
    println!("  sync    Apply config links to the filesystem");
    println!("  help    Show this help message");
    println!("\nConfig:");
    println!("  Looks for dbdm.conf in the current directory.");
    println!("  Each line: 'link = <from> <to>' or 'sudolink = <from> <to>'");
}
