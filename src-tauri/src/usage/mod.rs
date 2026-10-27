mod protocol;
mod tracker;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use chrono::{Days, Local};
use parking_lot::Mutex;
use serde::Serialize;

use crate::store::{Store, UsageAppTotal, UsageDayTotal, UsageHourTotal, UsageRecord};

const RETENTION_DAYS: u64 = 90;
const RECENT_LIMIT: u32 = 8;
const RECENT_SCAN: u32 = 400;
const TOP_APPS_LIMIT: u32 = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PauseReason {
    Idle,
    Locked,
    Disabled,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageStatus {
    pub available: bool,
    pub reason: Option<String>,
    pub tracking: bool,
    pub paused: bool,
    pub pause_reason: Option<PauseReason>,
    pub active_app: Option<String>,
    pub active_since_ms: Option<i64>,
    pub idle: bool,
    pub locked: bool,
    pub idle_supported: bool,
    pub lock_supported: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSnapshot {
    pub days: u64,
    pub total_ms: u64,
    pub today_ms: u64,
    pub apps: Vec<UsageAppTotal>,
    pub daily: Vec<UsageDayTotal>,
    pub hourly: Vec<UsageHourTotal>,
    pub recent: Vec<UsageRecord>,
    pub tracked_since_ms: Option<i64>,
}

pub(super) enum Control {
    Configure { enabled: bool, idle_ms: u64 },
    Clear(Sender<Result<(), String>>),
    Shutdown(Sender<()>),
}

pub struct UsageService {
    store: Arc<Store>,
    status: Arc<Mutex<UsageStatus>>,
    control: Mutex<Option<Sender<Control>>>,
    thread: Mutex<Option<JoinHandle<()>>>,
    stopped: AtomicBool,
}

impl UsageService {
    pub fn start(store: Arc<Store>, enabled: bool, idle_threshold_ms: u64) -> Self {
        let cutoff = (Local::now().date_naive() - Days::new(RETENTION_DAYS))
            .format("%Y-%m-%d")
            .to_string();
        let _ = store.prune_usage(&cutoff);

        let status = Arc::new(Mutex::new(UsageStatus {
            tracking: enabled,
            ..UsageStatus::default()
        }));
        let (tx, rx) = mpsc::channel();
        let status_for_thread = status.clone();
        let store_for_thread = store.clone();
        let spawned = std::thread::Builder::new()
            .name("clockit-usage".to_string())
            .spawn(move || {
                tracker::run(
                    store_for_thread,
                    status_for_thread,
                    enabled,
                    idle_threshold_ms,
                    rx,
                );
            });

        let thread = match spawned {
            Ok(handle) => Some(handle),
            Err(error) => {
                let mut status = status.lock();
                status.reason = Some(format!("Usage tracker could not start: {error}"));
                None
            }
        };

        Self {
            store,
            status,
            control: Mutex::new(Some(tx)),
            thread: Mutex::new(thread),
            stopped: AtomicBool::new(false),
        }
    }

    pub fn configure(&self, enabled: bool, idle_threshold_secs: u64) {
        self.status.lock().tracking = enabled;
        let idle_ms = idle_threshold_secs.clamp(30, 1800) * 1000;
        self.send(Control::Configure { enabled, idle_ms });
    }

    pub fn status(&self) -> UsageStatus {
        self.status.lock().clone()
    }

    pub fn snapshot(&self, days: u32) -> Result<UsageSnapshot, String> {
        let days = days.clamp(1, 30) as u64;
        let today = Local::now().date_naive();
        let since = today - Days::new(days - 1);
        let today_str = today.format("%Y-%m-%d").to_string();
        let since_str = since.format("%Y-%m-%d").to_string();

        let total_ms = self.store.usage_total(&since_str)?;
        let today_ms = self.store.usage_today(&today_str)?;
        let apps = self.store.usage_by_app(&since_str, TOP_APPS_LIMIT)?;
        let stored_days = self.store.usage_by_day(&since_str)?;
        let mut by_day: HashMap<String, u64> = HashMap::new();
        for entry in stored_days {
            by_day.insert(entry.day, entry.total_ms);
        }
        let daily = (0..days)
            .map(|offset| {
                let day = (since + Days::new(offset)).format("%Y-%m-%d").to_string();
                let total = by_day.get(&day).copied().unwrap_or(0);
                UsageDayTotal { day, total_ms: total }
            })
            .collect();

        let stored_hours = self.store.usage_by_hour(&today_str)?;
        let mut by_hour: HashMap<u8, u64> = HashMap::new();
        for entry in stored_hours {
            by_hour.insert(entry.hour, entry.total_ms);
        }
        let hourly = (0..24u8)
            .map(|hour| UsageHourTotal {
                hour,
                total_ms: by_hour.get(&hour).copied().unwrap_or(0),
            })
            .collect();

        Ok(UsageSnapshot {
            days,
            total_ms,
            today_ms,
            apps,
            daily,
            hourly,
            recent: merge_recent(self.store.usage_recent(RECENT_SCAN)?),
            tracked_since_ms: self.store.usage_first_ms()?,
        })
    }

    pub fn clear_history(&self) -> Result<(), String> {
        let (tx, rx) = mpsc::channel();
        self.send(Control::Clear(tx));
        match rx.recv_timeout(Duration::from_secs(3)) {
            Ok(result) => result,
            Err(_) => Err("Usage tracker did not respond".to_string()),
        }
    }

    pub fn shutdown(&self) {
        if self.stopped.swap(true, Ordering::SeqCst) {
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.send(Control::Shutdown(tx));
        let _ = rx.recv_timeout(Duration::from_secs(2));
        if let Some(handle) = self.thread.lock().take() {
            let _ = handle.join();
        }
    }

    fn send(&self, control: Control) {
        if let Some(tx) = self.control.lock().as_ref() {
            let _ = tx.send(control);
        }
    }
}

impl Drop for UsageService {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn merge_recent(records: Vec<UsageRecord>) -> Vec<UsageRecord> {
    let mut merged: Vec<UsageRecord> = Vec::with_capacity(RECENT_LIMIT as usize);
    for record in records {
        if let Some(last) = merged.last_mut() {
            if last.app_id == record.app_id && record.ended_at_ms >= last.started_at_ms {
                last.started_at_ms = last.started_at_ms.min(record.started_at_ms);
                continue;
            }
        }
        merged.push(record);
    }
    merged.truncate(RECENT_LIMIT as usize);
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: i64, app_id: &str, start: i64, end: i64) -> UsageRecord {
        UsageRecord {
            id,
            app_id: app_id.to_string(),
            started_at_ms: start,
            ended_at_ms: end,
        }
    }

    #[test]
    fn recent_merges_checkpoint_segments_and_keeps_switches_separate() {
        let records = vec![
            record(6, "editor", 90_000, 120_000),
            record(5, "editor", 60_000, 90_000),
            record(4, "browser", 45_000, 60_000),
            record(3, "editor", 30_000, 45_000),
            record(2, "editor", 10_000, 30_000),
            record(1, "term", 0, 10_000),
        ];

        let merged = merge_recent(records);
        assert_eq!(merged.len(), 4);
        assert_eq!(merged[0].app_id, "editor");
        assert_eq!(merged[0].started_at_ms, 60_000);
        assert_eq!(merged[0].ended_at_ms, 120_000);
        assert_eq!(merged[1].app_id, "browser");
        assert_eq!(merged[2].app_id, "editor");
        assert_eq!(merged[2].started_at_ms, 10_000);
        assert_eq!(merged[3].app_id, "term");
    }

    #[test]
    fn recent_merge_stops_at_pause_gaps_and_limits_rows() {
        let mut records: Vec<UsageRecord> = vec![record(17, "browser", 640_000, 650_000)];
        records.extend((0..16).rev().map(|index| {
            let start = index as i64 * 40_000;
            record(index + 1, "editor", start, start + 30_000)
        }));

        let merged = merge_recent(records);
        assert_eq!(merged.len(), RECENT_LIMIT as usize);
        assert_eq!(merged[0].app_id, "browser");
        assert_eq!(merged[1].app_id, "editor");
        assert_eq!(merged[1].started_at_ms, 600_000);
        assert_eq!(merged[1].ended_at_ms, 630_000);
    }
}
