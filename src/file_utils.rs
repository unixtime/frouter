use crate::hash_compute::compute_sha256;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use toml::Value;

mod error;

use crate::logging::log_file_event;
use crate::Configuration;

pub fn ensure_config_exists(home_config_path: &Path) {
    if !home_config_path.exists() {
        println!(
            "Config file not found at {}. Attempting to create...",
            home_config_path.display()
        );

        if let Some(home_config_dir) = home_config_path.parent() {
            if !home_config_dir.exists() {
                println!(
                    "Config directory {} doesn't exist. Creating...",
                    home_config_dir.display()
                );
                fs::create_dir_all(home_config_dir).expect("Failed to create config directory");
            }
        }

        let default_config_content = r#"
[directories]
downloads = "~/Downloads"

[[extensions]]
name = "pdf"
path = "~/Downloads/PDF"
enabled = true

[[extensions]]
name = "jpg"
path = "~/Downloads/IMAGES/JPG"
enabled = true

[[extensions]]
name = "png"
path = "~/Downloads/IMAGES/PNG"
enabled = true
"#;
        fs::write(home_config_path, default_config_content)
            .expect("Failed to write default config");
        println!(
            "Default config successfully written to {}",
            home_config_path.display()
        );
    } else {
        println!(
            "Config file already exists at {}",
            home_config_path.display()
        );
    }
}

fn get_unique_target(original: &Path, target_dir: &Path) -> Result<PathBuf, std::io::Error> {
    let mut target = target_dir.join(original.file_name().unwrap());
    let original_extension = original.extension().unwrap_or_default();
    let original_stem = original.file_stem().unwrap_or_default();

    if target.exists() {
        let original_hash = compute_sha256(original)?;
        let target_hash = compute_sha256(&target)?;

        if original_hash == target_hash {
            return Ok(target);
        }

        let mut counter = 1;
        loop {
            target = target_dir.join(format!(
                "{}_{}.{}",
                original_stem.to_string_lossy(),
                counter,
                original_extension.to_string_lossy()
            ));
            if !target.exists() || compute_sha256(&target)? == original_hash {
                break;
            }
            counter += 1;
        }
    }

    Ok(target)
}

pub fn process_existing_files_in_dir(directory: &str, config: &Configuration) {
    // Attempt to read directory entries
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(e) => {
            println!("Failed to read directory {}: {}", directory, e);
            return;
        }
    };

    for entry_result in entries {
        match entry_result {
            Ok(entry) => {
                let path = entry.path();
                if path.is_file() {
                    if let Some(extension) = get_extension_from_config(&path, &config.extensions) {
                        let target_dir = Path::new(&extension.path);

                        // Use the `get_unique_target` function
                        let unique_target_path = get_unique_target(&path, target_dir)
                            .expect("Failed to get a unique target path");

                        // Compute the hash before moving the file
                        let sha256_hash = match compute_sha256(&path) {
                            Ok(hash) => hash,
                            Err(e) => {
                                println!(
                                    "Failed to compute SHA256 hash for {}: {}",
                                    path.display(),
                                    e
                                );
                                continue; // skip to next iteration
                            }
                        };

                        if let Err(e) = copy_then_delete(&path, &unique_target_path) {
                            println!(
                                "Failed to move pre-existing file from {} to {}. Error: {}",
                                path.display(),
                                unique_target_path.display(),
                                e
                            );
                        } else {
                            log_file_event(&path, &unique_target_path, &sha256_hash);
                            // Log the file event using the unique target path
                        }
                    }
                }
            }
            Err(e) => {
                println!("Failed to process an entry: {}", e);
            }
        }
    }
}

pub fn copy_then_delete<P: AsRef<Path>, Q: AsRef<Path>>(
    source: P,
    unique_target: Q,
) -> std::io::Result<()> {
    // Copy and delete using the provided unique target.
    fs::copy(&source, &unique_target)?;
    fs::remove_file(source)?;
    Ok(())
}

pub fn ensure_directory_exists<P: AsRef<Path>>(dir: P) -> std::io::Result<()> {
    if !dir.as_ref().exists() {
        fs::create_dir_all(&dir)
    } else {
        Ok(())
    }
}

fn expand_home(path: &str) -> Option<PathBuf> {
    if let Some(without_tilde) = path.strip_prefix('~') {
        dirs::home_dir().map(|home| home.join(without_tilde.trim_start_matches('/')))
    } else {
        Some(PathBuf::from(path))
    }
}

pub fn load_config(home_config_path: &Path) -> Result<Configuration, Box<dyn std::error::Error>> {
    // Load the configuration.
    let content = fs::read_to_string(home_config_path)?;
    let value: Value = toml::from_str(&content)?;

    let directories = value["directories"].as_table().unwrap().clone();
    let mut expanded_directories = HashMap::new();
    for (k, v) in directories.iter() {
        if k.ends_with("_enabled") {
            continue;
        }

        let key_enabled = format!("{}_enabled", k);
        if directories
            .get(&key_enabled)
            .map_or(false, |val| val.as_bool().unwrap_or(false))
        {
            if let Some(expanded_path) = expand_home(v.as_str().unwrap()) {
                expanded_directories.insert(k.clone(), expanded_path.to_string_lossy().to_string());
            }
        }
    }

    let mut extensions: Vec<crate::FileExtension> = Vec::new();
    for extension in value["extensions"].as_array().unwrap() {
        if extension
            .get("enabled")
            .map_or(false, |val| val.as_bool().unwrap_or(false))
        {
            let name = extension["name"].as_str().unwrap().to_string();
            let path = if let Some(expanded_path) = expand_home(extension["path"].as_str().unwrap())
            {
                expanded_path.to_string_lossy().to_string()
            } else {
                extension["path"].as_str().unwrap().to_string()
            };
            extensions.push(crate::FileExtension { name, path });
        }
    }

    Ok(Configuration {
        directories: expanded_directories,
        extensions,
    })
}

pub fn get_extension_from_config<'a>(
    path: &Path,
    extensions: &'a [crate::FileExtension],
) -> Option<&'a crate::FileExtension> {
    // Convert the file extension to lowercase for case-insensitive comparison
    path.extension().and_then(|os_str| {
        os_str.to_str().and_then(|ext| {
            let ext_lower = ext.to_lowercase(); // Convert to lowercase
            extensions.iter().find(|file_ext| file_ext.name.to_lowercase() == ext_lower)
        })
    })
}
