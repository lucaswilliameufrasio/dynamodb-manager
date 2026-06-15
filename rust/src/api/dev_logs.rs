use std::collections::VecDeque;
use std::sync::Mutex;

const MAX_ENTRIES: usize = 300;

#[derive(Debug, Clone)]
pub struct DevLogEntry {
    pub timestamp: String,
    pub level: String,
    pub scope: String,
    pub message: String,
}

static LOG_BUFFER: Mutex<VecDeque<DevLogEntry>> = Mutex::new(VecDeque::new());

fn now_iso() -> String {
    // Simple ISO-like timestamp — good enough for dev logging
    use std::time::{SystemTime, UNIX_EPOCH};
    let d = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = d.as_secs();
    let millis = d.subsec_millis();
    // Convert to local-ish by offsetting — use UTC for simplicity
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let mins = (time_secs % 3600) / 60;
    let secs_remainder = time_secs % 60;

    // Compute year/month/day from days since epoch (rough)
    let mut y = 1970i64;
    let mut remaining = days as i64;
    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }
    let month_days = if is_leap(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m = 1usize;
    for &md in month_days.iter() {
        if remaining < md as i64 {
            break;
        }
        remaining -= md as i64;
        m += 1;
    }
    let d = remaining + 1;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        y, m, d, hours, mins, secs_remainder, millis
    )
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn push(level: &str, scope: &str, message: String) {
    let entry = DevLogEntry {
        timestamp: now_iso(),
        level: level.to_string(),
        scope: scope.to_string(),
        message,
    };
    if let Ok(mut buf) = LOG_BUFFER.lock() {
        if buf.len() >= MAX_ENTRIES {
            buf.pop_front();
        }
        buf.push_back(entry);
    }
}

pub(crate) fn log_info(scope: &str, message: impl Into<String>) {
    push("info", scope, message.into());
}

pub(crate) fn log_warn(scope: &str, message: impl Into<String>) {
    push("warn", scope, message.into());
}

pub(crate) fn log_error(scope: &str, message: impl Into<String>) {
    push("error", scope, message.into());
}

/// Return all buffered dev log entries (newest first).
pub fn get_recent_dev_logs() -> Vec<DevLogEntry> {
    if let Ok(buf) = LOG_BUFFER.lock() {
        buf.iter().rev().cloned().collect()
    } else {
        Vec::new()
    }
}

/// Clear all buffered dev log entries.
pub fn clear_dev_logs() {
    if let Ok(mut buf) = LOG_BUFFER.lock() {
        buf.clear();
    }
}
