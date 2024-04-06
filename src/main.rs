use crate::logging::{log_error_to_file, log_file_event};
use hash_compute::compute_sha256;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvError};


mod file_utils;
pub mod hash_compute;
mod logging;

use file_utils::*;

mod error;
use error::FileRouterError;

use std::error::Error;

mod db_utils;
mod test_config;
// db_utils.rs

use db_utils::Logger;

use std::collections::HashSet;
use std::time::{Duration, Instant};

 /*
Automatically implement `fmt::Debug` for this struct.
*/
#[derive(Debug)]
struct RecentlyProcessed {
    files: HashSet<String>,
    timestamps: HashMap<String, Instant>,
}

// Default implementation for RecentlyProcessed struct to keep track of recently processed files.
impl RecentlyProcessed {
    const DEBOUNCE_DURATION: Duration = Duration::from_secs(10);

    /*
    Create a new RecentlyProcessed struct.
    */
    fn new() -> Self {
        Self {
            files: HashSet::new(),
            timestamps: HashMap::new(),
        }
    }
    
    /*
    Add a file to the set of recently processed files.
    */
    fn add(&mut self, filename: &str) {
        self.files.insert(filename.to_string());
        self.timestamps.insert(filename.to_string(), Instant::now());
    }

    /*
    Remove old files from the set of recently processed files.
    */
    fn remove_old(&mut self) {
        let old_files: Vec<_> = self
            .timestamps
            .iter()
            .filter(|(_, &v)| v.elapsed() > Self::DEBOUNCE_DURATION)
            .map(|(k, _)| k.clone())
            .collect();

        for file in old_files {
            self.files.remove(&file);
            self.timestamps.remove(&file);
        }
    }

    /*
    Check if a file is in the set of recently processed files.
    */
    fn contains(&self, filename: &str) -> bool {
        self.files.contains(filename)
    }
}

impl Default for RecentlyProcessed {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct Configuration {
    directories: HashMap<String, String>,
    extensions: Vec<FileExtension>,
}

#[derive(Debug)]
pub struct FileExtension {
    name: String,
    path: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut recently_processed = RecentlyProcessed::new();

    let home_config_path = get_home_config_path()?;

    /*
    Ensure config file exists
    */
    ensure_config_exists(&home_config_path);

    let mut config = load_config(&home_config_path)?;

    /*
    Process existing files in directories
    */
    process_observed_directory(&config);

    println!("{:?}", config); // Print the parsed configuration.

    let (tx, rx) = mpsc::channel();

    // Ensure directories exist
    let directories_to_ensure: Vec<_> = config
        .directories
        .values()
        .chain(config.extensions.iter().map(|e| &e.path))
        .collect();
    for dir in directories_to_ensure {
        if let Err(e) = ensure_directory_exists(dir) {
            match e.kind() {
                std::io::ErrorKind::PermissionDenied => {
                    log_error_to_file(
                        "Directory Permission Denied",
                        &format!(
                            "Permission denied when trying to ensure {} directory exists.",
                            dir
                        ),
                    )?;
                }
                _ => {
                    log_error_to_file(
                        "Directory Error",
                        &format!("Failed to ensure {} directory exists. Error: {}", dir, e),
                    )?;
                }
            }
        }
    }

    let mut watched_dirs: Vec<String> = Vec::new();

    let mut watcher: RecommendedWatcher = Watcher::new(tx, Config::default()).unwrap();
    // Watch directories
    setup_directory_watchers(&config, &mut watcher, &mut watched_dirs);
    watcher
        .watch(&home_config_path, RecursiveMode::NonRecursive)
        .unwrap();

    // Use a Vec or any other collection to store detected file paths
    let mut accumulated_events: Vec<PathBuf> = Vec::new();
    let delay_duration = Duration::from_secs(10); // Adjust as necessary
    let mut last_event_time = Instant::now();

    // Process events received from the watcher channel and handle errors.
    loop {
        match rx.recv() {
            Ok(Ok(Event { paths, .. })) => {
                let event_path = &paths[0];

                if !recently_processed.contains(event_path.to_str().unwrap()) {
                    accumulated_events.push(event_path.clone());
                    recently_processed.add(event_path.to_str().unwrap());
                }
                recently_processed.remove_old();

                last_event_time = Instant::now();

                // Check if the changed file is the config file
                if event_path == &home_config_path {
                    process_observed_directory(&config);
                    println!("Config file changed. Reloading...");
                    match load_config(&home_config_path) {
                        Ok(new_config) => {
                            config = new_config;
                            println!("Config reloaded successfully.");

                            // Unwatch previous directories.
                            for dir in &watched_dirs {
                                if let Err(e) = watcher.unwatch(Path::new(dir)) {
                                    let _ = log_error_to_file(
                                        "Unwatch Directory Error",
                                        &format!("Failed to unwatch directory {}: {}", dir, e),
                                    );
                                }
                            }
                            watched_dirs.clear();

                            // Watch new directories.
                            setup_directory_watchers(&config, &mut watcher, &mut watched_dirs);
                        }
                        Err(e) => {
                            let _ = log_error_to_file("Config Load Error", &format!("{:?}", e));
                        }
                    }
                } else {
                    handle_directory_event(event_path, &config);
                }
            }
            Ok(Err(e)) => {
                log_error_to_file("Watch Error", &format!("{:?}", e))?;
            }
            Err(RecvError) => {
                log_error_to_file("Watch Error", "Failed to receive event")?;
            }
        }
        // Check if the delay duration has elapsed and process the accumulated events.
        if last_event_time.elapsed() > delay_duration && !accumulated_events.is_empty() {
            for path in &accumulated_events {
                handle_directory_event(path, &config);
            }
            accumulated_events.clear();
        }
    }
}

// Process existing files in a directory and log them.
fn process_observed_directory(config: &Configuration) {
    let logger = Logger::new();
    match logger.start_transaction() {
        Ok(_) => {
            for dir in config.directories.values() {
                process_existing_files_in_dir(dir, config);
            }
            if let Err(e) = logger.end_transaction() {
                eprintln!("Failed to commit transaction: {}", e);
            }
        }
        Err(e) => {
            eprintln!("Failed to start transaction: {}", e);
        }
    }
}

// Watch a directory for changes and process them.
fn setup_directory_watchers(
    config: &Configuration,
    watcher: &mut RecommendedWatcher,
    watched_dirs: &mut Vec<String>,
) {
    for dir in config.directories.values() {
        if let Err(e) = watcher.watch(Path::new(dir), RecursiveMode::NonRecursive) {
            let _ = log_error_to_file(
                "Directory Watch Error",
                &format!("Failed to watch directory {}: {}", dir, e),
            );
        } else {
            println!("Watching directory {}", dir);
            watched_dirs.push(dir.clone());
        }
    }
}

// Handle a directory event by moving the file to the appropriate directory.
fn handle_directory_event(path: &Path, config: &Configuration) {
    if path.exists() {
        if let Some(extension) = get_extension_from_config(path, &config.extensions) {
            let target_dir = Path::new(&extension.path);
            let target = target_dir.join(path.file_name().unwrap());
            let sha256_hash = compute_sha256(path).expect("Failed to compute SHA256 hash");
            log_file_event(path, &target, &sha256_hash); // Modified function call

            if !target_dir.exists() {
                fs::create_dir_all(target_dir)
                    .expect("Failed to create directory for file extension");
            }

            if let Err(e) = copy_then_delete(path, target) {
                let _ = log_error_to_file(
                    "File Move Error",
                    &format!("Failed to move file. Error: {}", e),
                );
            }
        }
    }
}

// Get the extension of a file from the configuration file.
fn get_home_config_path() -> Result<PathBuf, FileRouterError> {
    let home_dir = dirs::home_dir()
        .ok_or_else(|| FileRouterError::ConfigError("Failed to fetch home directory".into()))?;
    Ok(home_dir
        .join(".config")
        .join("frouter")
        .join("config.toml"))
}
