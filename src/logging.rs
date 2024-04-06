use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use crate::db_utils::Logger;

#[derive(Debug, Serialize)]
struct ErrorLog {
    timestamp: String,
    error_type: String,
    message: String,
}

impl ErrorLog {
    fn new(error_type: &str, message: &str) -> Self {
        Self {
            timestamp: format!("{}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S")),
            error_type: error_type.to_string(),
            message: message.to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileEventLog {
    source_path: PathBuf,
    target_path: PathBuf,
    file_name: PathBuf,
    timestamp: String,
    filehash: String,
}

/*
Set this to true to enable JSON logging for troubleshooting.
*/
const LOG_TO_JSON: bool = false;

// Assume these paths are obtained from a configuration file or environment variables
static ERROR_LOG_PATH: &str = "/usr/local/var/logs/error.log";
static FILE_EVENT_LOG_PATH: &str = "/usr/local/var/logs/file_event_log.json";

pub fn log_error_to_file(error_type: &str, message: &str) -> std::io::Result<()> {
    let log = ErrorLog::new(error_type, message);
    let error_string = serde_json::to_string_pretty(&log)?;

    let mut file = OpenOptions::new().append(true).open(ERROR_LOG_PATH)?;
    file.write_all(error_string.as_bytes())?;
    file.write_all(b"\n")?;

    Ok(())
}

pub fn log_file_event(source_path: &Path, target_path: &Path, filehash: &str) {
    let current_time = format!("{}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));

    if LOG_TO_JSON {
        let log = FileEventLog {
            source_path: source_path.to_path_buf(),
            target_path: target_path.to_path_buf(),
            file_name: source_path.file_name().unwrap().into(),
            timestamp: current_time.clone(),
            filehash: filehash.to_string(),
        };

        // Use FILE_EVENT_LOG_PATH instead of hard-coded path
        if let Err(e) = append_log_to_json(FILE_EVENT_LOG_PATH, &log) {
            eprintln!("Failed to append log to JSON: {}", e);
        }
    } else {
        let logger = Logger::new();
        if let Err(e) = logger.insert_log_without_commit(
            source_path.to_str().unwrap(),
            target_path.to_str().unwrap(),
            source_path.file_name().unwrap().to_str().unwrap(),
            &current_time,
            filehash,
        ) {
            eprintln!("Failed to insert log without commit: {}", e);
        }
    }
}

fn append_log_to_json<P: AsRef<Path>>(path: P, log: &FileEventLog) -> std::io::Result<()> {
    let mut logs = if path.as_ref().exists() {
        let file_content = fs::read_to_string(&path)?;
        serde_json::from_str::<Vec<FileEventLog>>(&file_content).unwrap_or_else(|_| vec![])
    } else {
        vec![]
    };

    logs.push(log.clone());

    let json_string = serde_json::to_string_pretty(&logs)?;
    // Use the path parameter, which is now FILE_EVENT_LOG_PATH from the caller
    fs::write(path, json_string)?;

    Ok(())
}
