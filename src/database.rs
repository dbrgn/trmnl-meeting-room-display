use log::info;
use rusqlite::{Connection, params};
use std::path::Path;
use std::sync::Mutex;

/// Database connection and operations wrapper
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Create a new database connection and initialize tables
    pub fn new(db_path: &str) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(db_path)?;

        // Create devices table if it doesn't exist
        conn.execute(
            "CREATE TABLE IF NOT EXISTS devices (
                id TEXT PRIMARY KEY,
                registered_at INTEGER NOT NULL
            )",
            [],
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Register a new device or update an existing one
    pub fn register_device(&self, device_id: &str) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        conn.execute(
            "INSERT OR REPLACE INTO devices (id, registered_at) VALUES (?1, ?2)",
            params![device_id, now],
        )?;

        Ok(())
    }

    /// Check if a device exists in the database
    pub fn device_exists(&self, device_id: &str) -> Result<bool, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT 1 FROM devices WHERE id = ?1")?;
        let exists = stmt.exists(params![device_id])?;
        Ok(exists)
    }
}

/// Initialize the database
pub fn init_database(db_path: &str) -> std::sync::Arc<Database> {
    if !Path::new(db_path).exists() {
        info!("Creating new database at {}", db_path);
    } else {
        info!("Using existing database at {}", db_path);
    }

    match Database::new(db_path) {
        Ok(db) => std::sync::Arc::new(db),
        Err(e) => {
            panic!("Failed to initialize database: {}", e);
        }
    }
}
