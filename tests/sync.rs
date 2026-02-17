use std::fs;
use tempfile::tempdir;

#[test]
fn perform_sync_when_targets_dont_exist() {
    let workspace = tempdir().expect("create temp workspace");

    let source_file = workspace.path().join("source.txt");
    let source_dir = workspace.path().join("source_dir");
    fs::write(&source_file, "example").expect("write source file");
    fs::create_dir(&source_dir).expect("create source dir");
    fs::write(source_dir.join("nested.txt"), "nested").expect("write nested file");

    let dest_root = workspace.path().join("dest");
    fs::create_dir(&dest_root).expect("create dest root");

    let dest_file = dest_root.join("linked.txt");
    let dest_dir = dest_root.join("linked_dir");
    fs::write(&dest_file, "").expect("create empty dest file");
    fs::create_dir(&dest_dir).expect("create empty dest dir");

    let config_path = workspace.path().join("dbdm.conf");
    let config_contents = format!(
        "link = {} {}\nlink = {} {}\n",
        source_file.display(),
        dest_file.display(),
        source_dir.display(),
        dest_dir.display()
    );
    fs::write(&config_path, config_contents).expect("write config");

    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_dbdm"));
    let mut child = command
        .arg("sync")
        .arg("--test-mode")
        .current_dir(workspace.path())
        .stdin(std::process::Stdio::piped())
        .spawn()
        .expect("spawn dbdm sync");

    let status = child.wait().expect("wait for dbdm sync");
    assert!(status.success());

    let file_meta = fs::symlink_metadata(&dest_file).expect("stat dest file");
    assert!(file_meta.file_type().is_symlink());

    let file_target = fs::read_link(&dest_file).expect("read dest file link");
    let file_target = if file_target.is_relative() {
        dest_file
            .parent()
            .expect("dest file parent")
            .join(file_target)
    } else {
        file_target
    };
    let file_target_full = fs::canonicalize(&file_target).expect("canonicalize file target");
    let source_file_full = fs::canonicalize(&source_file).expect("canonicalize source file");
    assert_eq!(file_target_full, source_file_full);

    let dir_meta = fs::symlink_metadata(&dest_dir).expect("stat dest dir");
    assert!(dir_meta.file_type().is_symlink());

    let dir_target = fs::read_link(&dest_dir).expect("read dest dir link");
    let dir_target = if dir_target.is_relative() {
        dest_dir.parent().expect("dest dir parent").join(dir_target)
    } else {
        dir_target
    };
    let dir_target_full = fs::canonicalize(&dir_target).expect("canonicalize dir target");
    let source_dir_full = fs::canonicalize(&source_dir).expect("canonicalize source dir");
    assert_eq!(dir_target_full, source_dir_full);
}

#[test]
fn perform_sync_when_targets_are_empty() {
    let workspace = tempdir().expect("create temp workspace");

    let source_dir = workspace.path().join("source_dir");
    fs::create_dir(&source_dir).expect("create source dir");
    fs::write(source_dir.join("nested.txt"), "nested").expect("write nested file");

    let dest_root = workspace.path().join("dest");
    fs::create_dir(&dest_root).expect("create dest root");

    let dest_dir = dest_root.join("linked_dir");
    fs::create_dir(&dest_dir).expect("create empty dest dir");
    fs::create_dir(dest_dir.join("empty_child")).expect("create empty nested dir");

    let config_path = workspace.path().join("dbdm.conf");
    let config_contents = format!("link = {} {}\n", source_dir.display(), dest_dir.display());
    fs::write(&config_path, config_contents).expect("write config");

    let status = std::process::Command::new(env!("CARGO_BIN_EXE_dbdm"))
        .arg("sync")
        .arg("--test-mode")
        .current_dir(workspace.path())
        .status()
        .expect("run dbdm sync");
    assert!(status.success());

    let dir_meta = fs::symlink_metadata(&dest_dir).expect("stat dest dir");
    assert!(dir_meta.file_type().is_symlink());

    let dir_target = fs::read_link(&dest_dir).expect("read dest dir link");
    let dir_target = if dir_target.is_relative() {
        dest_dir.parent().expect("dest dir parent").join(dir_target)
    } else {
        dir_target
    };
    let dir_target_full = fs::canonicalize(&dir_target).expect("canonicalize dir target");
    let source_dir_full = fs::canonicalize(&source_dir).expect("canonicalize source dir");
    assert_eq!(dir_target_full, source_dir_full);
}

#[test]
fn perform_sync_when_targets_exist_with_backup() {
    let workspace = tempdir().expect("create temp workspace");

    let source_file = workspace.path().join("source.txt");
    let source_dir = workspace.path().join("source_dir");
    fs::write(&source_file, "example").expect("write source file");
    fs::create_dir(&source_dir).expect("create source dir");
    fs::write(source_dir.join("nested.txt"), "nested").expect("write nested file");

    let dest_root = workspace.path().join("dest");
    fs::create_dir(&dest_root).expect("create dest root");

    let dest_file = dest_root.join("linked.txt");
    let dest_dir = dest_root.join("linked_dir");
    fs::write(&dest_file, "old file").expect("create dest file");
    fs::create_dir(&dest_dir).expect("create dest dir");
    fs::write(dest_dir.join("old.txt"), "old dir").expect("write dest dir file");

    let config_path = workspace.path().join("dbdm.conf");
    let config_contents = format!(
        "link = {} {}\nlink = {} {}\n",
        source_file.display(),
        dest_file.display(),
        source_dir.display(),
        dest_dir.display()
    );
    fs::write(&config_path, config_contents).expect("write config");

    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_dbdm"));
    let mut child = command
        .arg("sync")
        .current_dir(workspace.path())
        .stdin(std::process::Stdio::piped())
        .spawn()
        .expect("spawn dbdm sync");

    {
        let stdin = child.stdin.as_mut().expect("open stdin");
        std::io::Write::write_all(stdin, b"b\nb\ny\n").expect("confirm backup");
    }

    let status = child.wait().expect("wait for dbdm sync");
    assert!(status.success());

    let file_meta = fs::symlink_metadata(&dest_file).expect("stat dest file");
    assert!(file_meta.file_type().is_symlink());

    let file_target = fs::read_link(&dest_file).expect("read dest file link");
    let file_target = if file_target.is_relative() {
        dest_file
            .parent()
            .expect("dest file parent")
            .join(file_target)
    } else {
        file_target
    };
    let file_target_full = fs::canonicalize(&file_target).expect("canonicalize file target");
    let source_file_full = fs::canonicalize(&source_file).expect("canonicalize source file");
    assert_eq!(file_target_full, source_file_full);

    let dir_meta = fs::symlink_metadata(&dest_dir).expect("stat dest dir");
    assert!(dir_meta.file_type().is_symlink());

    let dir_target = fs::read_link(&dest_dir).expect("read dest dir link");
    let dir_target = if dir_target.is_relative() {
        dest_dir.parent().expect("dest dir parent").join(dir_target)
    } else {
        dir_target
    };
    let dir_target_full = fs::canonicalize(&dir_target).expect("canonicalize dir target");
    let source_dir_full = fs::canonicalize(&source_dir).expect("canonicalize source dir");
    assert_eq!(dir_target_full, source_dir_full);

    let file_backup = source_file
        .parent()
        .expect("source file parent")
        .join("linked.txt.bak.dbdm");
    let file_backup_contents = fs::read_to_string(&file_backup).expect("read file backup");
    assert_eq!(file_backup_contents, "old file");

    let dir_backup = source_dir.join("linked_dir.bak.dbdm");
    let dir_backup_file = dir_backup.join("old.txt");
    let dir_backup_contents = fs::read_to_string(&dir_backup_file).expect("read dir backup");
    assert_eq!(dir_backup_contents, "old dir");
}

#[test]
fn perform_sync_when_targets_exist_without_backup() {
    let workspace = tempdir().expect("create temp workspace");

    let source_file = workspace.path().join("source.txt");
    let source_dir = workspace.path().join("source_dir");
    fs::write(&source_file, "example").expect("write source file");
    fs::create_dir(&source_dir).expect("create source dir");
    fs::write(source_dir.join("nested.txt"), "nested").expect("write nested file");

    let dest_root = workspace.path().join("dest");
    fs::create_dir(&dest_root).expect("create dest root");

    let dest_file = dest_root.join("linked.txt");
    let dest_dir = dest_root.join("linked_dir");
    fs::write(&dest_file, "conflict").expect("create conflicting dest file");
    fs::create_dir(&dest_dir).expect("create conflicting dest dir");
    fs::write(dest_dir.join("existing.txt"), "existing").expect("write conflicting dest dir file");

    let config_path = workspace.path().join("dbdm.conf");
    let config_contents = format!(
        "link = {} {}\nlink = {} {}\n",
        source_file.display(),
        dest_file.display(),
        source_dir.display(),
        dest_dir.display()
    );
    fs::write(&config_path, config_contents).expect("write config");

    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_dbdm"));
    let mut child = command
        .arg("sync")
        .arg("--test-mode")
        .current_dir(workspace.path())
        .stdin(std::process::Stdio::piped())
        .spawn()
        .expect("spawn dbdm sync");

    {
        let stdin = child.stdin.as_mut().expect("open stdin");
        std::io::Write::write_all(stdin, b"r\nr\ny\n").expect("select replace option");
    }

    let status = child.wait().expect("wait for dbdm sync");
    assert!(status.success());

    let file_meta = fs::symlink_metadata(&dest_file).expect("stat dest file");
    assert!(file_meta.file_type().is_symlink());
    let file_target = fs::read_link(&dest_file).expect("read dest file link");
    let file_target = if file_target.is_relative() {
        dest_file
            .parent()
            .expect("dest file parent")
            .join(file_target)
    } else {
        file_target
    };
    let file_target_full = fs::canonicalize(&file_target).expect("canonicalize file target");
    let source_file_full = fs::canonicalize(&source_file).expect("canonicalize source file");
    assert_eq!(file_target_full, source_file_full);

    let dir_meta = fs::symlink_metadata(&dest_dir).expect("stat dest dir");
    assert!(dir_meta.file_type().is_symlink());
    let dir_target = fs::read_link(&dest_dir).expect("read dest dir link");
    let dir_target = if dir_target.is_relative() {
        dest_dir.parent().expect("dest dir parent").join(dir_target)
    } else {
        dir_target
    };
    let dir_target_full = fs::canonicalize(&dir_target).expect("canonicalize dir target");
    let source_dir_full = fs::canonicalize(&source_dir).expect("canonicalize source dir");
    assert_eq!(dir_target_full, source_dir_full);
}

#[test]
fn perform_sync_with_force_flag() {
    let workspace = tempdir().expect("create temp workspace");

    let source_file = workspace.path().join("source.txt");
    let source_dir = workspace.path().join("source_dir");
    fs::write(&source_file, "example").expect("write source file");
    fs::create_dir(&source_dir).expect("create source dir");
    fs::write(source_dir.join("nested.txt"), "nested").expect("write nested file");

    let dest_root = workspace.path().join("dest");
    fs::create_dir(&dest_root).expect("create dest root");

    let dest_file = dest_root.join("linked.txt");
    let dest_dir = dest_root.join("linked_dir");
    fs::write(&dest_file, "existing file").expect("create conflicting dest file");
    fs::create_dir(&dest_dir).expect("create conflicting dest dir");
    fs::write(dest_dir.join("old.txt"), "old dir").expect("write conflicting dest dir file");

    let config_path = workspace.path().join("dbdm.conf");
    let config_contents = format!(
        "link = {} {}\nlink = {} {}\n",
        source_file.display(),
        dest_file.display(),
        source_dir.display(),
        dest_dir.display()
    );
    fs::write(&config_path, config_contents).expect("write config");

    let status = std::process::Command::new(env!("CARGO_BIN_EXE_dbdm"))
        .arg("sync")
        .arg("--force")
        .arg("--test-mode")
        .current_dir(workspace.path())
        .status()
        .expect("run dbdm sync --force");
    assert!(status.success());

    let file_meta = fs::symlink_metadata(&dest_file).expect("stat dest file");
    assert!(file_meta.file_type().is_symlink());
    let file_target = fs::read_link(&dest_file).expect("read dest file link");
    let file_target = if file_target.is_relative() {
        dest_file
            .parent()
            .expect("dest file parent")
            .join(file_target)
    } else {
        file_target
    };
    let file_target_full = fs::canonicalize(&file_target).expect("canonicalize file target");
    let source_file_full = fs::canonicalize(&source_file).expect("canonicalize source file");
    assert_eq!(file_target_full, source_file_full);

    let dir_meta = fs::symlink_metadata(&dest_dir).expect("stat dest dir");
    assert!(dir_meta.file_type().is_symlink());
    let dir_target = fs::read_link(&dest_dir).expect("read dest dir link");
    let dir_target = if dir_target.is_relative() {
        dest_dir.parent().expect("dest dir parent").join(dir_target)
    } else {
        dir_target
    };
    let dir_target_full = fs::canonicalize(&dir_target).expect("canonicalize dir target");
    let source_dir_full = fs::canonicalize(&source_dir).expect("canonicalize source dir");
    assert_eq!(dir_target_full, source_dir_full);

    let file_contents = fs::read_to_string(&source_file).expect("read source file");
    assert_eq!(file_contents, "example");
    let dir_contents =
        fs::read_to_string(source_dir.join("nested.txt")).expect("read source dir file");
    assert_eq!(dir_contents, "nested");
}
