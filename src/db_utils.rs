use duckdb::{params, Connection};

pub struct Logger {
    conn: Connection,
}

impl Logger {
    pub fn new() -> Self {
        let conn = Connection::open("/usr/local/var/logs/frouter.db").unwrap();
        conn.execute("CREATE TABLE IF NOT EXISTS logs (source TEXT, destination TEXT, filename TEXT, timestamp TEXT, filehash TEXT)", params![]).unwrap();
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

    #[test]
    fn test_insert_log() {
        let logger = Logger::new(); // Assumes Logger::new doesn't panic when a DB can't be opened
        logger.start_transaction().unwrap();

        let result = logger.insert_log_without_commit(
            "source/path",
            "destination/path",
            "filename.ext",
            "2023-01-01 12:00:00",
            "hash_value",
        );

        assert!(result.is_ok());

        // Verify entry is inserted correctly, possibly by querying the `logs` table.
        // Cleanup might involve rolling back the transaction or deleting the test database file.
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
