// db_utils.rs

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
