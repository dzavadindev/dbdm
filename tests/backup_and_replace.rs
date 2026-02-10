use dbdm::backup_and_replace;

#[test]
fn backs_up_directory_target_into_source_dir() {
    let temp = tempfile::tempdir().expect("tempdir should be created");
    let from_dir = temp.path().join("dotfiles/nvim");
    let to_dir = temp.path().join("config/nvim");

    std::fs::create_dir_all(&from_dir).expect("from dir should be created");
    std::fs::create_dir_all(&to_dir).expect("to dir should be created");

    let to_file = to_dir.join("init.lua");
    std::fs::write(&to_file, "old config").expect("write should succeed");

    backup_and_replace(&from_dir, &to_dir).expect("backup should succeed");

    let backup_path = from_dir.join("nvim.bak.dbdm");
    let backup_file = backup_path.join("init.lua");
    let backup_contents = std::fs::read_to_string(&backup_file).expect("backup should exist");
    assert_eq!(backup_contents, "old config");

    let meta = std::fs::symlink_metadata(&to_dir).expect("metadata should exist");
    assert!(meta.file_type().is_symlink());

    let target = std::fs::read_link(&to_dir).expect("read_link should succeed");
    assert_eq!(target, from_dir);
}

#[test]
fn backs_up_file_target_into_source_parent() {
    let temp = tempfile::tempdir().expect("tempdir should be created");
    let from_file = temp.path().join("dotfiles/.gitconfig");
    let to_file = temp.path().join("home/.gitconfig");

    std::fs::create_dir_all(from_file.parent().expect("from parent")).expect("mkdir");
    std::fs::create_dir_all(to_file.parent().expect("to parent")).expect("mkdir");

    std::fs::write(&from_file, "source").expect("write should succeed");
    std::fs::write(&to_file, "old").expect("write should succeed");

    backup_and_replace(&from_file, &to_file).expect("backup should succeed");

    let backup_path = from_file
        .parent()
        .expect("from parent")
        .join(".gitconfig.bak.dbdm");
    let backup_contents = std::fs::read_to_string(&backup_path).expect("backup should exist");
    assert_eq!(backup_contents, "old");

    let meta = std::fs::symlink_metadata(&to_file).expect("metadata should exist");
    assert!(meta.file_type().is_symlink());

    let target = std::fs::read_link(&to_file).expect("read_link should succeed");
    assert_eq!(target, from_file);
}
