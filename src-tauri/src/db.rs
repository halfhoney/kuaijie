use rusqlite::{params, Connection};
use serde::Serialize;
use std::path::Path;

#[derive(Serialize, Clone)]
pub struct HistoryItem {
    pub id: i64,
    pub kind: String,
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub created_at: String,
    /// 文件大小（字节），列表展示用
    pub size: u64,
}

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn new(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                kind TEXT NOT NULL,
                path TEXT NOT NULL,
                width INTEGER NOT NULL,
                height INTEGER NOT NULL,
                created_at TEXT NOT NULL
            );",
        )
        .map_err(|e| e.to_string())?;
        Ok(Self { conn })
    }

    pub fn insert(&self, kind: &str, path: &Path, width: u32, height: u32) -> Result<HistoryItem, String> {
        let now = chrono::Local::now().to_rfc3339();
        let path_str = path.to_string_lossy().to_string();
        self.conn
            .execute(
                "INSERT INTO history (kind, path, width, height, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![kind, path_str, width, height, now],
            )
            .map_err(|e| e.to_string())?;
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        Ok(HistoryItem {
            id: self.conn.last_insert_rowid(),
            kind: kind.to_string(),
            path: path_str,
            width,
            height,
            created_at: now,
            size,
        })
    }

    pub fn list(&self) -> Result<Vec<HistoryItem>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, kind, path, width, height, created_at FROM history ORDER BY id DESC LIMIT 200")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                let path: String = row.get(2)?;
                Ok(HistoryItem {
                    id: row.get(0)?,
                    kind: row.get(1)?,
                    size: std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0),
                    path,
                    width: row.get(3)?,
                    height: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row.map_err(|e| e.to_string())?);
        }
        Ok(items)
    }

    /// 删除记录，返回文件路径（用于一并删除文件）
    pub fn delete(&self, id: i64) -> Result<Option<String>, String> {
        let path: Option<String> = self
            .conn
            .query_row("SELECT path FROM history WHERE id = ?1", params![id], |row| row.get(0))
            .ok();
        self.conn
            .execute("DELETE FROM history WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(path)
    }
}
