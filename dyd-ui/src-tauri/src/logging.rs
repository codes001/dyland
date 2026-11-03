use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::paths;

const MAX_LOG_BYTES: u64 = 1_048_576;

static LOG_FILE: OnceLock<Mutex<File>> = OnceLock::new();

pub fn init() {
    let dir = paths::state_dir();
    if let Err(err) = fs::create_dir_all(&dir) {
        eprintln!("dyland: cannot create state dir {}: {err}", dir.display());
        return;
    }
    let path = dir.join("ui.log");
    rotate_if_needed(&path);
    match OpenOptions::new().create(true).append(true).open(&path) {
        Ok(file) => {
            let _ = LOG_FILE.set(Mutex::new(file));
            info(&format!("dyland ui starting pid={}", std::process::id()));
        }
        Err(err) => eprintln!("dyland: cannot open {}: {err}", path.display()),
    }
}

pub fn info(message: &str) {
    write_line("INFO", message);
}

pub fn error(message: &str) {
    write_line("ERROR", message);
}

pub fn write_health(body: &serde_json::Value) {
    let dir = paths::state_dir();
    if fs::create_dir_all(&dir).is_err() {
        return;
    }
    let path = dir.join("ui-health.json");
    let Ok(data) = serde_json::to_vec_pretty(body) else {
        return;
    };
    if let Err(err) = fs::write(&path, data) {
        error(&format!("health file write failed: {err}"));
    }
}

fn write_line(level: &str, message: &str) {
    let line = format!("{} {level} {message}\n", timestamp());
    eprint!("{line}");
    let Some(slot) = LOG_FILE.get() else {
        return;
    };
    let mut file = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let _ = file.write_all(line.as_bytes());
    let _ = file.flush();
}

fn rotate_if_needed(path: &Path) {
    let Ok(meta) = fs::metadata(path) else {
        return;
    };
    if meta.len() < MAX_LOG_BYTES {
        return;
    }
    let rotated = path.with_extension("log.1");
    let _ = fs::rename(path, rotated);
}

fn timestamp() -> String {
    let Ok(elapsed) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return "unknown-time".to_string();
    };
    let secs = elapsed.as_secs();
    let days = (secs / 86_400) as i64;
    let tod = secs % 86_400;
    let hour = tod / 3600;
    let minute = (tod % 3600) / 60;
    let second = tod % 60;
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Howard Hinnant's `civil_from_days`. `days` is days since 1970-01-01.
fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = (mp as i64 + if mp < 10 { 3 } else { -9 }) as u32;
    let year = year + if month <= 2 { 1 } else { 0 };
    (year as i32, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_epoch_is_1970_01_01() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
    }

    #[test]
    fn known_date_2026_11_02() {
        // 2026-11-02 is 20759 days after 1970-01-01.
        assert_eq!(civil_from_days(20_759), (2026, 11, 2));
    }
}
