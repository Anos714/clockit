use std::fs;
use std::path::Path;
use std::time::Duration;

use chrono::{LocalResult, TimeZone, Timelike};
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::timer::TimerCore;

const MAX_APP_ID_CHARS: usize = 128;
const MAX_USAGE_PARTS: usize = 512;

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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageRecord {
    pub id: i64,
    pub app_id: String,
    pub started_at_ms: i64,
    pub ended_at_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageAppTotal {
    pub app_id: String,
    pub total_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageDayTotal {
    pub day: String,
    pub total_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageHourTotal {
    pub hour: u8,
    pub total_ms: u64,
}

struct UsagePart {
    start_ms: i64,
    end_ms: i64,
    day: String,
    hour: u8,
}

impl Store {
    pub fn open(dir: &Path) -> Result<Self, String> {
        fs::create_dir_all(dir).map_err(|e| format!("could not create data dir: {e}"))?;
        let path = dir.join("clockit.db");
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
                 ON todos (done, created_at_ms DESC);
             CREATE TABLE IF NOT EXISTS usage_segments (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 app_id TEXT NOT NULL,
                 started_at_ms INTEGER NOT NULL,
                 ended_at_ms INTEGER NOT NULL,
                 day TEXT NOT NULL,
                 hour INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_usage_day
                 ON usage_segments (day, app_id);
             CREATE INDEX IF NOT EXISTS idx_usage_end
                 ON usage_segments (ended_at_ms DESC);",
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

    pub fn insert_usage(
        &self,
        app_id: &str,
        started_at_ms: u64,
        ended_at_ms: u64,
    ) -> Result<(), String> {
        if ended_at_ms <= started_at_ms {
            return Ok(());
        }
        let app_id: String = app_id.trim().chars().take(MAX_APP_ID_CHARS).collect();
        if app_id.is_empty() {
            return Ok(());
        }
        let parts = split_usage_parts(started_at_ms, ended_at_ms);
        let conn = self.conn.lock();
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        {
            let mut stmt = tx
                .prepare(
                    "INSERT INTO usage_segments
                     (app_id, started_at_ms, ended_at_ms, day, hour)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                )
                .map_err(|e| e.to_string())?;
            for part in parts {
                stmt.execute(params![
                    app_id,
                    part.start_ms,
                    part.end_ms,
                    part.day,
                    part.hour as i64
                ])
                .map_err(|e| e.to_string())?;
            }
        }
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn usage_total(&self, since_day: &str) -> Result<u64, String> {
        self.usage_sum("SELECT COALESCE(SUM(ended_at_ms - started_at_ms), 0)
                        FROM usage_segments WHERE day >= ?1", since_day)
    }

    pub fn usage_today(&self, day: &str) -> Result<u64, String> {
        self.usage_sum("SELECT COALESCE(SUM(ended_at_ms - started_at_ms), 0)
                        FROM usage_segments WHERE day = ?1", day)
    }

    fn usage_sum(&self, sql: &str, day: &str) -> Result<u64, String> {
        let conn = self.conn.lock();
        let total: i64 = conn
            .query_row(sql, [day], |row| row.get(0))
            .map_err(|e| e.to_string())?;
        Ok(total.max(0) as u64)
    }

    pub fn usage_by_app(&self, since_day: &str, limit: u32) -> Result<Vec<UsageAppTotal>, String> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT app_id, SUM(ended_at_ms - started_at_ms) AS total
                 FROM usage_segments WHERE day >= ?1
                 GROUP BY app_id ORDER BY total DESC, app_id ASC LIMIT ?2",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![since_day, limit as i64], |row| {
                Ok(UsageAppTotal {
                    app_id: row.get(0)?,
                    total_ms: row.get::<_, i64>(1)?.max(0) as u64,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn usage_by_day(&self, since_day: &str) -> Result<Vec<UsageDayTotal>, String> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT day, SUM(ended_at_ms - started_at_ms) AS total
                 FROM usage_segments WHERE day >= ?1
                 GROUP BY day ORDER BY day ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([since_day], |row| {
                Ok(UsageDayTotal {
                    day: row.get(0)?,
                    total_ms: row.get::<_, i64>(1)?.max(0) as u64,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn usage_by_hour(&self, day: &str) -> Result<Vec<UsageHourTotal>, String> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT hour, SUM(ended_at_ms - started_at_ms) AS total
                 FROM usage_segments WHERE day = ?1
                 GROUP BY hour ORDER BY hour ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([day], |row| {
                Ok(UsageHourTotal {
                    hour: row.get::<_, i64>(0)?.clamp(0, 23) as u8,
                    total_ms: row.get::<_, i64>(1)?.max(0) as u64,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn usage_recent(&self, limit: u32) -> Result<Vec<UsageRecord>, String> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT id, app_id, started_at_ms, ended_at_ms
                 FROM usage_segments ORDER BY ended_at_ms DESC, id DESC LIMIT ?1",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([limit as i64], |row| {
                Ok(UsageRecord {
                    id: row.get(0)?,
                    app_id: row.get(1)?,
                    started_at_ms: row.get(2)?,
                    ended_at_ms: row.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }

    pub fn usage_first_ms(&self) -> Result<Option<i64>, String> {
        let conn = self.conn.lock();
        conn.query_row("SELECT MIN(started_at_ms) FROM usage_segments", [], |row| {
            row.get::<_, Option<i64>>(0)
        })
        .map_err(|e| e.to_string())
    }

    pub fn clear_usage(&self) -> Result<(), String> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM usage_segments", [])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn prune_usage(&self, before_day: &str) -> Result<(), String> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM usage_segments WHERE day < ?1", [before_day])
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

fn split_usage_parts(started_at_ms: u64, ended_at_ms: u64) -> Vec<UsagePart> {
    let mut parts = Vec::new();
    let mut cursor = started_at_ms as i64;
    let end = ended_at_ms as i64;
    while cursor < end && parts.len() < MAX_USAGE_PARTS {
        let Some(dt) = chrono::Local.timestamp_millis_opt(cursor).single() else {
            break;
        };
        let next = next_hour_boundary(&dt)
            .map(|boundary| boundary.timestamp_millis())
            .filter(|boundary| *boundary > cursor)
            .unwrap_or(cursor + 3_600_000);
        let part_end = end.min(next);
        parts.push(UsagePart {
            start_ms: cursor,
            end_ms: part_end,
            day: dt.format("%Y-%m-%d").to_string(),
            hour: dt.hour() as u8,
        });
        cursor = part_end;
    }
    parts
}

fn next_hour_boundary(dt: &chrono::DateTime<chrono::Local>) -> Option<chrono::DateTime<chrono::Local>> {
    let naive = dt.naive_local();
    let next_naive = if naive.hour() == 23 {
        (naive.date() + chrono::Days::new(1)).and_hms_opt(0, 0, 0)?
    } else {
        naive.date().and_hms_opt(naive.hour() + 1, 0, 0)?
    };
    match chrono::Local.from_local_datetime(&next_naive) {
        LocalResult::Single(boundary) => Some(boundary),
        LocalResult::Ambiguous(first, _) => Some(first),
        LocalResult::None => None,
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
            "clockit-store-test-{}-{name}",
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

    #[test]
    fn usage_splits_at_hour_and_day_boundaries() {
        use chrono::{Local, TimeZone};

        let (store, dir) = temp_store("usage-split");
        let start = Local
            .with_ymd_and_hms(2026, 5, 14, 22, 30, 0)
            .unwrap()
            .timestamp_millis() as u64;
        store
            .insert_usage("firefox", start, start + 3 * 3_600_000)
            .expect("insert usage");

        assert_eq!(store.usage_total("2000-01-01").unwrap(), 10_800_000);
        let days = store.usage_by_day("2000-01-01").unwrap();
        assert_eq!(days.len(), 2);
        assert_eq!(days[0].total_ms, 5_400_000);
        assert_eq!(days[1].total_ms, 5_400_000);

        let hours = store.usage_by_hour(&days[0].day).unwrap();
        assert_eq!(
            hours.iter().map(|bucket| bucket.hour).collect::<Vec<_>>(),
            vec![22, 23]
        );
        assert_eq!(hours[0].total_ms, 1_800_000);
        assert_eq!(hours[1].total_ms, 3_600_000);

        let recent = store.usage_recent(10).unwrap();
        assert_eq!(recent.len(), 4);

        drop(store);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn usage_ignores_empty_ids_and_clamps_long_ones() {
        let (store, dir) = temp_store("usage-ids");
        store.insert_usage("   ", 0, 5_000).expect("empty id");
        store.insert_usage("x", 5_000, 5_000).expect("zero length");
        assert_eq!(store.usage_total("2000-01-01").unwrap(), 0);

        let long_id = "a".repeat(200);
        store.insert_usage(&long_id, 10_000, 11_000).expect("long id");
        let apps = store.usage_by_app("1900-01-01", 5).unwrap();
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].app_id.chars().count(), MAX_APP_ID_CHARS);

        drop(store);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn usage_aggregates_orders_and_prunes() {
        use chrono::{Local, TimeZone};

        let (store, dir) = temp_store("usage-agg");
        let day_ms = 86_400_000i64;
        let base = Local
            .with_ymd_and_hms(2026, 5, 10, 9, 0, 0)
            .unwrap()
            .timestamp_millis();
        // Three 10-minute blocks on three consecutive local days.
        for offset in 0..3 {
            let start = (base + offset * day_ms) as u64;
            store
                .insert_usage("editor", start, start + 600_000)
                .unwrap();
        }
        let start = base as u64;
        store.insert_usage("browser", start, start + 300_000).unwrap();

        let apps = store.usage_by_app("2000-01-01", 5).unwrap();
        assert_eq!(apps[0].app_id, "editor");
        assert_eq!(apps[0].total_ms, 1_800_000);
        assert_eq!(apps[1].app_id, "browser");

        let cutoff = Local
            .with_ymd_and_hms(2026, 5, 11, 0, 0, 0)
            .unwrap()
            .format("%Y-%m-%d")
            .to_string();
        store.prune_usage(&cutoff).unwrap();
        let apps = store.usage_by_app("2000-01-01", 5).unwrap();
        assert_eq!(
            apps.iter().map(|app| app.total_ms).sum::<u64>(),
            1_200_000
        );

        store.clear_usage().unwrap();
        assert_eq!(store.usage_total("2000-01-01").unwrap(), 0);
        assert_eq!(store.usage_first_ms().unwrap(), None);

        drop(store);
        let _ = fs::remove_dir_all(dir);
    }
}
