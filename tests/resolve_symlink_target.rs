use dbdm::resolve_symlink_target;
use std::path::Path;

#[test]
fn resolves_relative_target_from_symlink_parent() {
    let temp = tempfile::tempdir().expect("tempdir should be created");
    let link_path = temp.path().join("links/config");
    let target = Path::new("../dotfiles/nvim");

    let expected = link_path
        .parent()
        .expect("link path should have parent")
        .join(target);

    let resolved = resolve_symlink_target(&link_path, target);
    assert_eq!(resolved, expected);
}

#[test]
fn leaves_absolute_targets_unchanged() {
    let temp = tempfile::tempdir().expect("tempdir should be created");
    let link_path = temp.path().join("links/config");
    let target = temp.path().join("dotfiles/nvim");

    let resolved = resolve_symlink_target(&link_path, &target);
    assert_eq!(resolved, target);
}
