use dbdm::unique_backup_path;

#[test]
fn increments_backup_suffix_when_conflict_exists() {
    let temp = tempfile::tempdir().expect("tempdir should be created");
    let dir = temp.path();

    let base_path = dir.join("nvim.bak.dbdm");
    std::fs::write(&base_path, "existing").expect("write should succeed");

    let candidate = unique_backup_path(dir, "nvim");
    assert_eq!(candidate, dir.join("nvim.bak.dbdm.1"));

    std::fs::write(&candidate, "existing").expect("write should succeed");
    let next_candidate = unique_backup_path(dir, "nvim");
    assert_eq!(next_candidate, dir.join("nvim.bak.dbdm.2"));
}
