use std::path::Path;
use duckdb::{params, Connection};

pub struct Logger {
    conn: Connection,
}

impl Logger {
    // Existing constructor for regular use
    pub fn new() -> Self {
        Self::with_path("/usr/local/var/logs/frouter.db")
    }

    // New constructor for testing or other purposes where a custom path is needed
    pub fn with_path<P: AsRef<Path>>(path: P) -> Self {
        let conn = Connection::open(path).expect("Failed to open database");
        conn.execute(
            "CREATE TABLE IF NOT EXISTS logs (source TEXT, destination TEXT, filename TEXT, timestamp TEXT, filehash TEXT)",
            params![],
        )
            .expect("Failed to create logs table");
        Logger { conn }
    }

    pub fn start_transaction(&self) -> Result<(), duckdb::Error> {
        self.conn.execute("BEGIN", params![])?;
        Ok(())
    }


    pub fn end_transaction(&self) -> Result<(), duckdb::Error> {
        self.conn.execute("COMMIT", params![])?;
        Ok(())
    }

    pub fn insert_log_without_commit(
        &self,
        source: &str,
        dest: &str,
        filename: &str,
        timestamp: &str,
        filehash: &str,
    ) -> Result<(), duckdb::Error> {
        self.conn.execute(
            "INSERT INTO logs VALUES (?, ?, ?, ?, ?)",
            params!(source, dest, filename, timestamp, filehash),
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod db_tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_insert_log() {
        let temp_dir = TempDir::new().expect("Failed to create a temporary directory");
        let db_path = temp_dir.path().join("frouter_test.db");

        // Use Logger::with_path to create a logger instance with a temporary database
        let logger = Logger::with_path(&db_path);

        // Start a transaction
        logger.start_transaction().expect("Failed to start transaction");

        let result = logger.insert_log_without_commit(
            "source/path",
            "destination/path",
            "filename.ext",
            "2023-01-01 12:00:00",
            "hash_value",
        );

        assert!(result.is_ok());

        // Optionally, verify the inserted log...

        // Don't forget to end the transaction or roll it back as needed
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use tempfile::NamedTempFile;
    use crate::test_config::load_config;

    #[test]
    fn test_load_config() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file,
                 r#"
            [directories]
            downloads = "test_downloads"

            [[extensions]]
            name = "txt"
            path = "test_downloads/TXT"
            enabled = true
            "#
        ).unwrap();

        let config_path = temp_file.path();
        match load_config(config_path) {
            Ok(config) => {
                assert_eq!(config.directories.get("downloads").unwrap(), "test_downloads");
                assert_eq!(config.extensions.len(), 1);
                assert_eq!(config.extensions[0].name, "txt");
                assert_eq!(config.extensions[0].path, "test_downloads/TXT");
            },
            Err(e) => panic!("Failed to load config: {:?}", e),
        }
    }
}
