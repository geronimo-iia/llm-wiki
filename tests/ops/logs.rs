use llm_wiki_engine::ops;

fn config_path(dir: &std::path::Path) -> std::path::PathBuf {
    dir.join("state").join("config.toml")
}

fn logs_dir(dir: &std::path::Path) -> std::path::PathBuf {
    dir.join("state").join("logs")
}

#[test]
fn logs_list_empty_when_no_log_dir() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config_path(dir.path());
    std::fs::create_dir_all(cfg.parent().unwrap()).unwrap();

    let files = ops::logs_list(&cfg).unwrap();
    assert!(files.is_empty());
}

#[test]
fn logs_list_returns_log_files() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config_path(dir.path());
    let logs = logs_dir(dir.path());
    std::fs::create_dir_all(&logs).unwrap();
    std::fs::write(logs.join("2025-01-01.log"), "line1\n").unwrap();
    std::fs::write(logs.join("2025-01-02.log"), "line2\n").unwrap();

    let files = ops::logs_list(&cfg).unwrap();
    assert_eq!(files.len(), 2);
    assert_eq!(files[0], "2025-01-01.log");
    assert_eq!(files[1], "2025-01-02.log");
}

#[test]
fn logs_tail_errors_when_no_log_dir() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config_path(dir.path());
    std::fs::create_dir_all(cfg.parent().unwrap()).unwrap();

    let result = ops::logs_tail(&cfg, 10);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("no log directory"), "unexpected error: {msg}");
}

#[test]
fn logs_tail_returns_last_n_lines() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config_path(dir.path());
    let logs = logs_dir(dir.path());
    std::fs::create_dir_all(&logs).unwrap();

    let content: String = (1..=10).map(|i| format!("line{i}\n")).collect();
    std::fs::write(logs.join("2025-01-01.log"), &content).unwrap();

    let output = ops::logs_tail(&cfg, 3).unwrap();
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "line8");
    assert_eq!(lines[2], "line10");
}

#[test]
fn logs_clear_removes_all_files() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config_path(dir.path());
    let logs = logs_dir(dir.path());
    std::fs::create_dir_all(&logs).unwrap();
    std::fs::write(logs.join("a.log"), "a").unwrap();
    std::fs::write(logs.join("b.log"), "b").unwrap();
    std::fs::write(logs.join("c.log"), "c").unwrap();

    let removed = ops::logs_clear(&cfg).unwrap();
    assert_eq!(removed, 3);
    assert!(logs.exists(), "log dir should still exist after clear");
    assert_eq!(std::fs::read_dir(&logs).unwrap().count(), 0);
}

#[test]
fn logs_clear_returns_zero_when_no_dir() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config_path(dir.path());
    std::fs::create_dir_all(cfg.parent().unwrap()).unwrap();

    let removed = ops::logs_clear(&cfg).unwrap();
    assert_eq!(removed, 0);
}
