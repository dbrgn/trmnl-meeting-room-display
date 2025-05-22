use anyhow::{Context, Result};
use log::info;
use rusqlite::{Connection, params};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Database connection and operations wrapper
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Create a new database connection and initialize tables
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)
            .with_context(|| format!("Failed to open database at {}", db_path))?;

        // Create devices table if it doesn't exist
        conn.execute(
            "CREATE TABLE IF NOT EXISTS devices (
                id TEXT PRIMARY KEY,
                registered_at INTEGER NOT NULL
            )",
            [],
        )
        .context("Failed to create devices table")?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Register a new device or update an existing one
    pub fn register_device(&self, device_id: &str) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire lock on database connection: {}", e))?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .context("Failed to get current timestamp")?
            .as_secs() as i64;

        conn.execute(
            "INSERT OR REPLACE INTO devices (id, registered_at) VALUES (?1, ?2)",
            params![device_id, now],
        )
        .with_context(|| format!("Failed to register device {}", device_id))?;

        Ok(())
    }

    /// Check if a device exists in the database
    pub fn device_exists(&self, device_id: &str) -> Result<bool> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire lock on database connection: {}", e))?;

        let mut stmt = conn
            .prepare("SELECT 1 FROM devices WHERE id = ?1")
            .with_context(|| {
                format!(
                    "Failed to prepare statement to check device existence: {}",
                    device_id
                )
            })?;

        let exists = stmt
            .exists(params![device_id])
            .with_context(|| format!("Failed to check if device exists: {}", device_id))?;

        Ok(exists)
    }

    /// Retrieves a device by its ID
    pub fn get_device(&self, device_id: &str) -> Result<Option<DeviceRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire lock on database connection: {}", e))?;

        let mut stmt = conn
            .prepare("SELECT id, registered_at FROM devices WHERE id = ?1")
            .with_context(|| format!("Failed to prepare statement to get device: {}", device_id))?;

        let mut rows = stmt
            .query(params![device_id])
            .with_context(|| format!("Failed to execute query for device: {}", device_id))?;

        if let Some(row) = rows.next().context("Failed to read database row")? {
            Ok(Some(DeviceRecord {
                id: row.get(0).context("Failed to get ID field from row")?,
                registered_at: row
                    .get(1)
                    .context("Failed to get registered_at field from row")?,
            }))
        } else {
            Ok(None)
        }
    }
}

/// Record of a device in the database
#[derive(Debug, Clone)]
pub struct DeviceRecord {
    /// Device unique identifier (MAC address)
    pub id: String,
    /// Unix timestamp when the device was registered
    pub registered_at: i64,
}

/// Initialize the database with error handling
pub fn init_database(db_path: &str) -> Result<Arc<Database>> {
    if !Path::new(db_path).exists() {
        info!("Creating new database at {}", db_path);
    } else {
        info!("Using existing database at {}", db_path);
    }

    let db = Database::new(db_path)
        .with_context(|| format!("Failed to initialize database at {}", db_path))?;

    Ok(Arc::new(db))
}
