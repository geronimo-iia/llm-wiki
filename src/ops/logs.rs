use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

pub fn logs_path(config_path: &Path) -> PathBuf {
    let state_dir = config_path.parent().unwrap_or(Path::new("."));
    state_dir.join("logs")
}

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

fn latest_log_file(log_dir: &Path) -> Result<PathBuf> {
    let mut entries: Vec<_> = fs::read_dir(log_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .collect();

    if entries.is_empty() {
        bail!("no log files in {}", log_dir.display());
    }

    entries.sort_by_key(|e| e.file_name());
    Ok(entries.last().unwrap().path())
}
