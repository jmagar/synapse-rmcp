use std::io::Read;

use super::*;

#[test]
fn descriptor_open_reads_regular_file_beneath_root() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("value.txt");
    std::fs::write(&path, "safe").unwrap();
    let mut host = HostConfig::local();
    host.scout_read_roots = vec![dir.path().to_string_lossy().into_owned()];

    let mut content = String::new();
    bind_read_path(&host, path.to_str().unwrap())
        .unwrap()
        .into_file()
        .read_to_string(&mut content)
        .unwrap();
    assert_eq!(content, "safe");
}

#[cfg(unix)]
#[test]
fn descriptor_open_rejects_intermediate_symlink() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("secret.txt"), "secret").unwrap();
    std::os::unix::fs::symlink(outside.path(), root.path().join("link")).unwrap();
    let mut host = HostConfig::local();
    host.scout_read_roots = vec![root.path().to_string_lossy().into_owned()];

    let escaped = root.path().join("link/secret.txt");
    assert!(bind_read_path(&host, escaped.to_str().unwrap()).is_err());
}
