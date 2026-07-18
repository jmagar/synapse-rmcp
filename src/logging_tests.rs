use super::*;

#[test]
fn log_file_max_bytes_is_10mb() {
    assert_eq!(LOG_FILE_MAX_BYTES, 10 * 1024 * 1024);
}

#[test]
fn rotating_writer_appends_small_entries() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.log").to_path_buf();
    let mut writer = RotatingLogWriter::new(path.clone()).unwrap();
    writer.write_all(b"small content").unwrap();
    writer.flush().unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"small content");
}

#[test]
fn rotating_writer_retains_previous_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.log").to_path_buf();
    let mut writer = RotatingLogWriter::new(path.clone()).unwrap();
    writer
        .write_all(&vec![b'x'; LOG_FILE_MAX_BYTES as usize])
        .unwrap();
    writer.write_all(b"new generation").unwrap();
    writer.flush().unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"new generation");
    assert_eq!(
        std::fs::metadata(path.with_extension("log.1"))
            .unwrap()
            .len(),
        LOG_FILE_MAX_BYTES
    );
}
