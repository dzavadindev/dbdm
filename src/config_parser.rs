use regex::Regex;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;

static PARAMS_REGEXP: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<from>/?\S+/?)[ \t]+(?P<to>/?\S+/?)[ \t]*$")
        .map_err(|err| format!("Regex init error: {}", err))
        .unwrap()
});
static HOME_DIR: LazyLock<String> = LazyLock::new(|| env::var("HOME").expect("Can't read $HOME"));
static XDG_CONFIG_HOME: LazyLock<String> = LazyLock::new(|| {
    env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| format!("{}/.config", HOME_DIR.as_str()))
});

#[derive(Debug, PartialEq)]
pub struct Link {
    pub from: PathBuf,
    pub to: PathBuf,
}

#[derive(Debug)]
pub struct Config {
    pub links: Vec<Link>,
}

pub fn read_config(path: &PathBuf) -> Result<Config, String> {
    let content = match fs::read_to_string(path) {
        Ok(res) => res,
        Err(err) => {
            return Err(err.to_string());
        }
    };

    let mut links: Vec<Link> = Vec::new();
    for (idx, line) in content.lines().enumerate() {
        let link: Link = match parse_line(line, idx) {
            Ok(res) => res,
            Err(err) => return Err(err),
        };
        links.push(link);
    }

    return Ok(Config { links: links });
}

fn parse_line(line: &str, idx: usize) -> Result<Link, String> {
    // Read split out the line
    let (text_kind, mut text_params) = match line.split_once('=') {
        Some((a, b)) => (a, b),
        None => return Err(format!("Invalid syntax on line {}", idx)),
    };
    text_params = text_params.trim();

    // Before applying regex, check if there is a need to match
    if text_params.is_empty() {
        return Err(format!(
            "Invalid number of values on line {}. The supported syntax is '<kind> = <from> <to>'. Found 0 args",
            idx
        ));
    }

    // Verify its only two arguments
    let arg_count = text_params.split_whitespace().count();
    if arg_count != 2 {
        return Err(format!(
            "Invalid number of values on line {}. The supported syntax is '<kind> = <from> <to>'. Found {} args",
            idx, arg_count
        ));
    }

    if text_kind.trim() != "link" {
        return Err(format!(
            "Invalid path syntax on line {}. The supported syntax is '<kind> = <from> <to>'",
            idx
        ));
    }

    if let Some(caps) = PARAMS_REGEXP.captures(text_params) {
        let from = caps.name("from").unwrap().as_str();
        let to = caps.name("to").unwrap().as_str();

        let from = expand_keywords(from).map_err(|err| format!("{} on line {}", err, idx))?;
        let to = expand_keywords(to).map_err(|err| format!("{} on line {}", err, idx))?;

        let from_path = PathBuf::from(&from);
        let to_path = PathBuf::from(&to);

        if !from_path.exists() {
            return Err(format!(
                "<from> path specified at line {} doest contain any object",
                idx
            ));
        }

        if !to_path.exists() {
            if let Some(parent) = to_path.parent() {
                if !parent.exists() {
                    return Err(format!(
                        "Parent directory does not exist: {}",
                        parent.display()
                    ));
                }
            } else {
                return Err(format!("Path has no parent: {}", to_path.display()));
            }
        }

        return Ok(Link {
            from: PathBuf::from(&from),
            to: PathBuf::from(&to),
        });
    }

    // TODO: Not sure if I am missing a case in which the state can occur here
    Err(format!(
        "Unknown error encountered while parsing line {}",
        idx,
    ))
}

fn expand_keywords(line: &str) -> Result<String, String> {
    if line.contains('!')
        && !line.contains("!here")
        && !line.contains("!home")
        && !line.contains("!xdg_conf")
    {
        return Err(format!("Invalid keyword in {}", line));
    }

    let mut expanded = line.to_string();
    if expanded.contains("!here") {
        let here =
            std::env::current_dir().map_err(|err| format!("Failed to resolve !here: {}", err))?;
        expanded = expanded.replace("!here", &here.to_string_lossy());
    }

    expanded = expanded.replace("!home", HOME_DIR.as_str());
    expanded = expanded.replace("!xdg_conf", XDG_CONFIG_HOME.as_str());
    Ok(expanded)
}
