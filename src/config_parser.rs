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

#[derive(Debug)]
pub struct Link {
    pub from: PathBuf,
    pub to: PathBuf,
    pub sudo: bool,
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
    let (mut text_kind, mut text_params) = match line.split_once('=') {
        Some((a, b)) => (a, b),
        None => return Err(format!("Invalid syntax on line {}", idx)),
    };
    (text_kind, text_params) = (text_kind.trim(), text_params.trim());

    // Check the link type
    let use_sudo: bool = match text_kind {
        "link" => false,
        "sudolink" => true,
        _ => {
            return Err(format!(
                "Config only supports 'link' and 'sudolink'. Invalid kind '{}' on line {}",
                text_kind, idx
            ));
        }
    };

    // Before applying regex, check if there is a need to match
    if text_params.is_empty() {
        return Err(format!(
            "Invalid number of values on line {}. The supported syntax is '<kind> = <from> <to>'. Found 0 args",
            idx
        ));
    }

    if let Some(caps) = PARAMS_REGEXP.captures(text_params) {
        let from = caps.name("from").unwrap().as_str();
        let to = caps.name("to").unwrap().as_str();

        let from = expand_keywords(from).map_err(|err| format!("{} on line {}", err, idx))?;
        let to = expand_keywords(to).map_err(|err| format!("{} on line {}", err, idx))?;

        return Ok(Link {
            sudo: use_sudo,
            from: PathBuf::from(&from),
            to: PathBuf::from(&to),
        });
    }

    let arg_count = text_params.split_whitespace().count();
    if arg_count != 2 {
        return Err(format!(
            "Invalid number of values on line {}. The supported syntax is '<kind> = <from> <to>'. Found {} args",
            idx, arg_count
        ));
    }

    Err(format!(
        "Invalid path syntax on line {}. The supported syntax is '<kind> = <from> <to>'",
        idx
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
