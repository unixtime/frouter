use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use toml;

#[derive(Debug, Deserialize, Serialize)]
pub struct Configuration {
    pub directories: HashMap<String, String>,
    pub extensions: Vec<FileExtension>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FileExtension {
    pub name: String,
    pub path: String,
}

#[allow(dead_code)]
pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Configuration, Box<dyn std::error::Error>> {
    let config_str = fs::read_to_string(path)?;
    let config: Configuration = toml::from_str(&config_str)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_load_config() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"
[directories]
downloads = "test_downloads"

[[extensions]]
name = "txt"
path = "test_downloads/TXT"
enabled = true
"#
        ).unwrap();

        let config = load_config(temp_file.path()).unwrap();
        assert_eq!(config.directories.get("downloads").unwrap(), "test_downloads");
        assert_eq!(config.extensions.len(), 1);
        assert_eq!(config.extensions[0].name, "txt");
        assert_eq!(config.extensions[0].path, "test_downloads/TXT");
    }
}
