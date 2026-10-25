use std::fs;
use std::path::Path;
use std::time::Duration;

use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::timer::TimerCore;

pub struct Store {
    conn: Mutex<Connection>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionRecord {
    pub id: i64,
    pub planned_seconds: u64,
    pub actual_seconds: u64,
    pub started_at_ms: i64,
    pub ended_at_ms: i64,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct NewSession {
    pub planned_seconds: u64,
    pub actual_seconds: u64,
    pub started_at_ms: i64,
    pub ended_at_ms: i64,
    pub status: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TodoRecord {
    pub id: i64,
    pub text: String,
    pub done: bool,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

impl Store {
    pub fn open(dir: &Path) -> Result<Self, String> {
        fs::create_dir_all(dir).map_err(|e| format!("could not create data dir: {e}"))?;
        let path = dir.join("tempo.db");
        let conn = Connection::open(&path).map_err(|e| format!("could not open database: {e}"))?;
        conn.busy_timeout(Duration::from_secs(2))
            .map_err(|e| e.to_string())?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             CREATE TABLE IF NOT EXISTS meta (
                 key TEXT PRIMARY KEY,
                 value TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS sessions (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 planned_seconds INTEGER NOT NULL,
                 actual_seconds INTEGER NOT NULL,
                 started_at_ms INTEGER NOT NULL,
                 ended_at_ms INTEGER NOT NULL,
                 status TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_sessions_ended
                 ON sessions (ended_at_ms DESC);
             CREATE TABLE IF NOT EXISTS todos (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 text TEXT NOT NULL,
                 done INTEGER NOT NULL DEFAULT 0,
                 created_at_ms INTEGER NOT NULL,
                 updated_at_ms INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_todos_order
                 ON todos (done, created_at_ms DESC);",
        )
        .map_err(|e| e.to_string())?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn get_meta<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        let conn = self.conn.lock();
        let raw: Option<String> = conn
            .query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
                row.get(0)
            })
            .optional()
            .ok()
            .flatten();
        raw.and_then(|value| serde_json::from_str(&value).ok())
    }

    pub fn set_meta<T: Serialize>(&self, key: &str, value: &T) -> Result<(), String> {
        let raw = serde_json::to_string(value).map_err(|e| e.to_string())?;
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO meta (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, raw],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn load_timer(&self) -> Option<TimerCore> {
        self.get_meta("timer")
    }

    pub fn save_timer(&self, core: &TimerCore) -> Result<(), String> {
        self.set_meta("timer", core)
    }

    pub fn insert_session(&self, session: &NewSession) -> Result<(), String> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO sessions
             (planned_seconds, actual_seconds, started_at_ms, ended_at_ms, status)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                session.planned_seconds as i64,
                session.actual_seconds as i64,
                session.started_at_ms,
                session.ended_at_ms,
                session.status
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_todos(&self) -> Result<Vec<TodoRecord>, String> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT id, text, done, created_at_ms, updated_at_ms
                 FROM todos ORDER BY done ASC, created_at_ms DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], todo_from_row)
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn add_todo(&self, text: &str, now_ms: u64) -> Result<TodoRecord, String> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO todos (text, done, created_at_ms, updated_at_ms)
             VALUES (?1, 0, ?2, ?2)",
            params![text, now_ms as i64],
        )
        .map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT id, text, done, created_at_ms, updated_at_ms
             FROM todos WHERE id = ?1",
            [conn.last_insert_rowid()],
            todo_from_row,
        )
        .map_err(|e| e.to_string())
    }

    pub fn set_todo_done(
        &self,
        id: i64,
        done: bool,
        now_ms: u64,
    ) -> Result<TodoRecord, String> {
        let conn = self.conn.lock();
        let changed = conn
            .execute(
                "UPDATE todos SET done = ?1, updated_at_ms = ?2 WHERE id = ?3",
                params![done as i64, now_ms as i64, id],
            )
            .map_err(|e| e.to_string())?;
        if changed == 0 {
            return Err("Task not found".to_string());
        }
        conn.query_row(
            "SELECT id, text, done, created_at_ms, updated_at_ms
             FROM todos WHERE id = ?1",
            [id],
            todo_from_row,
        )
        .map_err(|e| e.to_string())
    }

    pub fn update_todo_text(
        &self,
        id: i64,
        text: &str,
        now_ms: u64,
    ) -> Result<TodoRecord, String> {
        let conn = self.conn.lock();
        let changed = conn
            .execute(
                "UPDATE todos SET text = ?1, updated_at_ms = ?2 WHERE id = ?3",
                params![text, now_ms as i64, id],
            )
            .map_err(|e| e.to_string())?;
        if changed == 0 {
            return Err("Task not found".to_string());
        }
        conn.query_row(
            "SELECT id, text, done, created_at_ms, updated_at_ms
             FROM todos WHERE id = ?1",
            [id],
            todo_from_row,
        )
        .map_err(|e| e.to_string())
    }

    pub fn delete_todo(&self, id: i64) -> Result<(), String> {
        let conn = self.conn.lock();
        let changed = conn
            .execute("DELETE FROM todos WHERE id = ?1", [id])
            .map_err(|e| e.to_string())?;
        if changed == 0 {
            return Err("Task not found".to_string());
        }
        Ok(())
    }

    pub fn list_sessions(&self, limit: u32) -> Result<Vec<SessionRecord>, String> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT id, planned_seconds, actual_seconds, started_at_ms, ended_at_ms, status
                 FROM sessions ORDER BY ended_at_ms DESC LIMIT ?1",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([limit as i64], |row| {
                Ok(SessionRecord {
                    id: row.get(0)?,
                    planned_seconds: row.get::<_, i64>(1)? as u64,
                    actual_seconds: row.get::<_, i64>(2)? as u64,
                    started_at_ms: row.get(3)?,
                    ended_at_ms: row.get(4)?,
                    status: row.get(5)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }
}

fn todo_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TodoRecord> {
    Ok(TodoRecord {
        id: row.get(0)?,
        text: row.get(1)?,
        done: row.get::<_, i64>(2)? != 0,
        created_at_ms: row.get(3)?,
        updated_at_ms: row.get(4)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store(name: &str) -> (Store, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "tempo-store-test-{}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        (Store::open(&dir).expect("store opens"), dir)
    }

    #[test]
    fn todos_support_crud_and_order() {
        let (store, dir) = temp_store("crud");

        let first = store.add_todo("first task", 1_000).expect("add first");
        assert_eq!(first.text, "first task");
        assert!(!first.done);

        let second = store.add_todo("second task", 2_000).expect("add second");
        let list = store.list_todos().expect("list");
        assert_eq!(
            list.iter().map(|todo| todo.text.as_str()).collect::<Vec<_>>(),
            vec!["second task", "first task"]
        );

        let done = store
            .set_todo_done(first.id, true, 3_000)
            .expect("toggle done");
        assert!(done.done);
        assert_eq!(done.updated_at_ms, 3_000);

        let renamed = store
            .update_todo_text(first.id, "renamed task", 4_000)
            .expect("rename");
        assert_eq!(renamed.text, "renamed task");
        assert_eq!(renamed.updated_at_ms, 4_000);

        let list = store.list_todos().expect("list after toggle");
        assert_eq!(list[0].id, second.id);
        assert_eq!(list[1].id, first.id);

        store.delete_todo(second.id).expect("delete");
        let list = store.list_todos().expect("list after delete");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, first.id);

        assert!(store.delete_todo(second.id).is_err());
        assert!(store.set_todo_done(second.id, false, 5_000).is_err());

        drop(store);
        let _ = fs::remove_dir_all(dir);
    }
}
