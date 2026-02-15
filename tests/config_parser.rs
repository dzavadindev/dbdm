use dbdm::config_parser::{Link, read_config};
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn parse_valid_config_file_to_config() {
    let tmp = tempdir().expect("tempdir");
    let root_dir = tmp.path().join("root");
    let db_dir = root_dir.join("db");
    let notes_dir = root_dir.join("notes");

    fs::create_dir_all(&db_dir).expect("create db dir");
    fs::create_dir_all(&notes_dir).expect("create notes dir");

    let config_path = root_dir.join("dbdm.conf");
    let config_contents = format!(
        "link = {} {}\nlink = {} {}\n",
        db_dir.display(),
        notes_dir.display(),
        notes_dir.display(),
        db_dir.display()
    );
    fs::write(&config_path, config_contents).expect("write config");

    let config = read_config(&config_path).expect("read config");

    let expected_links = vec![
        Link {
            from: PathBuf::from(&db_dir),
            to: PathBuf::from(&notes_dir),
        },
        Link {
            from: PathBuf::from(&notes_dir),
            to: PathBuf::from(&db_dir),
        },
    ];

    assert_eq!(config.links, expected_links);
}

#[test]
fn parsing_config_with_invalid_kind() {
    let tmp = tempdir().expect("tempdir");
    let root_dir = tmp.path().join("root");
    let db_dir = root_dir.join("db");
    let notes_dir = root_dir.join("notes");

    fs::create_dir_all(&db_dir).expect("create db dir");
    fs::create_dir_all(&notes_dir).expect("create notes dir");

    let config_path = root_dir.join("dbdm.conf");
    let config_contents = format!("lonk = {} {}\n", db_dir.display(), notes_dir.display());
    fs::write(&config_path, config_contents).expect("write config");

    let err = read_config(&config_path).expect_err("read config");
    assert_eq!(
        err,
        "Invalid path syntax on line 0. The supported syntax is '<kind> = <from> <to>'"
    )
}

#[test]
fn parsing_config_with_more_than_2_arguments() {
    let tmp = tempdir().expect("tempdir");
    let root_dir = tmp.path().join("root");
    let db_dir = root_dir.join("db");
    let notes_dir = root_dir.join("notes");
    let extra_dir = root_dir.join("extra");

    fs::create_dir_all(&db_dir).expect("create db dir");
    fs::create_dir_all(&notes_dir).expect("create notes dir");
    fs::create_dir_all(&extra_dir).expect("create extra dir");

    let config_path = root_dir.join("dbdm.conf");
    let config_contents = format!(
        "link = {} {} {}\n",
        db_dir.display(),
        notes_dir.display(),
        extra_dir.display()
    );
    fs::write(&config_path, config_contents).expect("write config");

    let err = read_config(&config_path).expect_err("read config");
    assert_eq!(
        err,
        "Invalid number of values on line 0. The supported syntax is '<kind> = <from> <to>'. Found 3 args"
    );
}

#[test]
fn parsing_config_with_less_than_2_arguments() {
    let tmp = tempdir().expect("tempdir");
    let root_dir = tmp.path().join("root");
    let db_dir = root_dir.join("db");
    let notes_dir = root_dir.join("notes");

    fs::create_dir_all(&db_dir).expect("create db dir");
    fs::create_dir_all(&notes_dir).expect("create notes dir");

    let config_path = root_dir.join("dbdm.conf");
    let config_contents = format!("link = {}\n", db_dir.display(),);
    fs::write(&config_path, config_contents).expect("write config");

    let err = read_config(&config_path).expect_err("read config");
    assert_eq!(
        err,
        "Invalid number of values on line 0. The supported syntax is '<kind> = <from> <to>'. Found 1 args"
    );
}

#[test]
fn keywords_are_expanded_correctly() {
    let tmp = tempdir().expect("tempdir");
    let root_dir = tmp.path().join("root");
    let here_dir = root_dir.join("here");
    let xdg_conf_dir = root_dir.join("xdg");
    let home_dir = root_dir.join("home");

    fs::create_dir_all(&here_dir).expect("create here dir");
    fs::create_dir_all(&xdg_conf_dir).expect("create xdg dir");
    fs::create_dir_all(&home_dir).expect("create home dir");

    let prev_dir = std::env::current_dir().expect("current dir");

    temp_env::with_vars(
        [
            ("XDG_CONFIG_HOME", Some(xdg_conf_dir.as_os_str())),
            ("HOME", Some(home_dir.as_os_str())),
        ],
        || {
            std::env::set_current_dir(&here_dir).expect("set current dir");

            let config_path = here_dir.join("dbdm.conf");
            let config_contents =
                "link = !xdg_conf !home\nlink = !here !xdg_conf\nlink = !home !here\n";
            fs::write(&config_path, config_contents).expect("write config");

            let config = read_config(&config_path).expect("read config");

            let expected_links = vec![
                Link {
                    from: PathBuf::from(&xdg_conf_dir),
                    to: PathBuf::from(&home_dir),
                },
                Link {
                    from: PathBuf::from(&here_dir),
                    to: PathBuf::from(&xdg_conf_dir),
                },
                Link {
                    from: PathBuf::from(&home_dir),
                    to: PathBuf::from(&here_dir),
                },
            ];

            assert_eq!(config.links, expected_links);

            std::env::set_current_dir(&prev_dir).expect("restore dir");
        },
    );
}
