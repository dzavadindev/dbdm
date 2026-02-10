use dbdm::replace_link;

#[test]
fn replaces_existing_file_with_symlink() {
    let temp = tempfile::tempdir().expect("tempdir should be created");
    let from = temp.path().join("source.conf");
    let to = temp.path().join("target.conf");

    std::fs::write(&from, "source").expect("write should succeed");
    std::fs::write(&to, "old").expect("write should succeed");

    replace_link(&from, &to).expect("replace should succeed");

    let meta = std::fs::symlink_metadata(&to).expect("metadata should exist");
    assert!(meta.file_type().is_symlink());

    let target = std::fs::read_link(&to).expect("read_link should succeed");
    assert_eq!(target, from);
}
