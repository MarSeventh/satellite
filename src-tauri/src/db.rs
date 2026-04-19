use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

pub struct Database {
    conn: Mutex<Connection>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HistoryItem {
    pub id: i64,
    pub filename: String,
    pub url: String,
    pub thumbnail: Option<String>,
    pub created_at: String,
}

impl Database {
    pub fn new() -> Result<Self, rusqlite::Error> {
        let db_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("satellite");
        std::fs::create_dir_all(&db_dir).ok();
        let db_path = db_dir.join("history.db");

        let conn = Connection::open(db_path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS history (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                filename    TEXT    NOT NULL,
                url         TEXT    NOT NULL,
                thumbnail   TEXT,
                created_at  TEXT    NOT NULL DEFAULT (datetime('now','localtime'))
            );
            CREATE INDEX IF NOT EXISTS idx_history_created ON history(created_at DESC);",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn insert(
        &self,
        filename: &str,
        url: &str,
        thumbnail: Option<&str>,
    ) -> Result<i64, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO history (filename, url, thumbnail) VALUES (?1, ?2, ?3)",
            params![filename, url, thumbnail],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn query(
        &self,
        page: u32,
        page_size: u32,
    ) -> Result<Vec<HistoryItem>, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let offset = (page.saturating_sub(1)) * page_size;
        let mut stmt = conn.prepare(
            "SELECT id, filename, url, thumbnail, created_at
             FROM history ORDER BY id DESC LIMIT ?1 OFFSET ?2",
        )?;
        let rows = stmt
            .query_map(params![page_size, offset], |row| {
                Ok(HistoryItem {
                    id: row.get(0)?,
                    filename: row.get(1)?,
                    url: row.get(2)?,
                    thumbnail: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn count(&self) -> Result<u32, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM history", [], |row| row.get(0))
    }

    pub fn delete(&self, id: i64) -> Result<bool, rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let affected = conn.execute("DELETE FROM history WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }
}

#[tauri::command]
pub fn get_history(
    db: tauri::State<'_, Database>,
    page: u32,
    page_size: u32,
) -> Result<Vec<HistoryItem>, String> {
    db.query(page, page_size).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_history_count(db: tauri::State<'_, Database>) -> Result<u32, String> {
    db.count().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_history(db: tauri::State<'_, Database>, id: i64) -> Result<bool, String> {
    db.delete(id).map_err(|e| e.to_string())
}
