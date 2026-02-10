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
fn parsing_config_with_invalid_kind() {}

#[test]
fn parsing_config_with_more_than_2_arguments() {}

#[test]
fn keywords_are_expanded_correctly() {}
