use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

/// Return the path to the log directory (sibling of the config file).
pub fn logs_path(config_path: &Path) -> PathBuf {
    let state_dir = config_path.parent().unwrap_or(Path::new("."));
    state_dir.join("logs")
}

/// Return the last `lines` lines from the most recent log file.
pub fn logs_tail(config_path: &Path, lines: usize) -> Result<String> {
    let log_dir = logs_path(config_path);
    if !log_dir.exists() {
        bail!("no log directory at {}", log_dir.display());
    }

    let latest = latest_log_file(&log_dir)?;
    let file = fs::File::open(&latest)?;
    let reader = BufReader::new(file);
    let all_lines: Vec<String> = reader.lines().map_while(Result::ok).collect();
    let start = all_lines.len().saturating_sub(lines);
    Ok(all_lines[start..].join("\n"))
}

/// Delete all log files and return the number removed.
pub fn logs_clear(config_path: &Path) -> Result<usize> {
    let log_dir = logs_path(config_path);
    if !log_dir.exists() {
        return Ok(0);
    }

    let mut removed = 0;
    for entry in fs::read_dir(&log_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            fs::remove_file(entry.path())?;
            removed += 1;
        }
    }
    Ok(removed)
}

/// List log file names sorted by name.
pub fn logs_list(config_path: &Path) -> Result<Vec<String>> {
    let log_dir = logs_path(config_path);
    if !log_dir.exists() {
        return Ok(Vec::new());
    }

    let mut files: Vec<String> = fs::read_dir(&log_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    files.sort();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logs_path_is_sibling_logs_dir() {
        let p = logs_path(Path::new("/state/config.toml"));
        assert_eq!(p, PathBuf::from("/state/logs"));
    }

    #[test]
    fn logs_list_empty_when_dir_absent() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("nonexistent").join("config.toml");
        let result = logs_list(&cfg).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn logs_list_returns_sorted_names() {
        let dir = tempfile::tempdir().unwrap();
        let log_dir = dir.path().join("logs");
        fs::create_dir_all(&log_dir).unwrap();
        fs::write(log_dir.join("2026-08-21.log"), "c").unwrap();
        fs::write(log_dir.join("2026-08-19.log"), "a").unwrap();
        fs::write(log_dir.join("2026-08-20.log"), "b").unwrap();
        let cfg = dir.path().join("config.toml");
        let mut names = logs_list(&cfg).unwrap();
        names.sort();
        assert_eq!(names, vec!["2026-08-19.log", "2026-08-20.log", "2026-08-21.log"]);
    }

    #[test]
    fn logs_clear_removes_all_files_and_returns_count() {
        let dir = tempfile::tempdir().unwrap();
        let log_dir = dir.path().join("logs");
        fs::create_dir_all(&log_dir).unwrap();
        fs::write(log_dir.join("a.log"), "x").unwrap();
        fs::write(log_dir.join("b.log"), "y").unwrap();
        let cfg = dir.path().join("config.toml");
        let removed = logs_clear(&cfg).unwrap();
        assert_eq!(removed, 2);
        assert_eq!(logs_list(&cfg).unwrap().len(), 0);
    }

    #[test]
    fn logs_tail_returns_last_n_lines() {
        let dir = tempfile::tempdir().unwrap();
        let log_dir = dir.path().join("logs");
        fs::create_dir_all(&log_dir).unwrap();
        let content = (1..=10).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n");
        fs::write(log_dir.join("run.log"), content).unwrap();
        let cfg = dir.path().join("config.toml");
        let tail = logs_tail(&cfg, 3).unwrap();
        assert_eq!(tail, "line 8\nline 9\nline 10");
    }
}

fn latest_log_file(log_dir: &Path) -> Result<PathBuf> {
    let mut entries: Vec<_> = fs::read_dir(log_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .collect();

    entries.sort_by_key(|e| e.file_name());
    let entry = entries
        .last()
        .ok_or_else(|| anyhow::anyhow!("no log files in {}", log_dir.display()))?;
    Ok(entry.path())
}
