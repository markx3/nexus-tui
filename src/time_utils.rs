use std::time::SystemTime;

/// Convert epoch seconds to civil date (year, month, day) using the Hinnant algorithm.
/// This is more accurate than the iterative approach used elsewhere.
pub fn epoch_to_ymd(secs: u64) -> (i64, u64, u64) {
    let days = (secs / 86400) as i64;
    let z = days + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// Convert epoch seconds to an ISO 8601 UTC string.
pub fn epoch_to_iso(secs: u64) -> String {
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;
    let (year, month, day) = epoch_to_ymd(secs);
    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}Z")
}

/// Get the current epoch time in seconds.
pub fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Parse an ISO 8601 timestamp and return seconds since that time.
/// Returns `None` if the timestamp can't be parsed.
pub fn seconds_ago(iso_ts: &str) -> Option<i64> {
    let ts = iso_ts.trim();
    if ts.len() < 19 {
        return None;
    }

    let year: i64 = ts.get(0..4)?.parse().ok()?;
    let month: i64 = ts.get(5..7)?.parse().ok()?;
    let day: i64 = ts.get(8..10)?.parse().ok()?;
    let hour: i64 = ts.get(11..13)?.parse().ok()?;
    let min: i64 = ts.get(14..16)?.parse().ok()?;
    let sec: i64 = ts.get(17..19)?.parse().ok()?;

    let ts_epoch = utc_to_epoch(year, month, day, hour, min, sec);
    let now = now_epoch() as i64;
    Some(now - ts_epoch)
}

/// Convert a UTC date/time to a Unix epoch timestamp.
fn utc_to_epoch(year: i64, month: i64, day: i64, hour: i64, min: i64, sec: i64) -> i64 {
    let mut days: i64 = 0;
    for y in 1970..year {
        days += if is_leap_year(y) { 366 } else { 365 };
    }
    let days_in_months = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for m in 1..month {
        days += days_in_months[(m - 1) as usize];
        if m == 2 && is_leap_year(year) {
            days += 1;
        }
    }
    days += day - 1;
    days * 86400 + hour * 3600 + min * 60 + sec
}

fn is_leap_year(y: i64) -> bool {
    (y.rem_euclid(4) == 0 && y.rem_euclid(100) != 0) || y.rem_euclid(400) == 0
}

/// Format an ISO timestamp into a human-readable relative time string.
pub fn relative_time(iso_ts: &str) -> String {
    let Some(secs) = seconds_ago(iso_ts) else {
        return "?".to_string();
    };
    if secs < 0 {
        return "future".to_string();
    }
    let minutes = secs / 60;
    let hours = secs / 3600;
    let days = secs / 86400;
    let months = days / 30;

    if minutes < 1 {
        "just now".to_string()
    } else if hours < 1 {
        format!("{}m", minutes)
    } else if days < 1 {
        format!("{}h", hours)
    } else if days < 30 {
        format!("{}d", days)
    } else if months < 12 {
        format!("{}mo", months)
    } else {
        format!("{}y", days / 365)
    }
}

/// Check if a timestamp is stale (more than `threshold_secs` seconds old).
/// Default threshold is 7 days (604800 seconds).
pub fn is_stale(iso_ts: &str, threshold_secs: u64) -> bool {
    seconds_ago(iso_ts)
        .map(|secs| secs > threshold_secs as i64)
        .unwrap_or(false)
}

/// Format epoch seconds as "YYYY.MM.DD".
pub fn epoch_to_date_display(secs: u64) -> String {
    let (y, m, d) = epoch_to_ymd(secs);
    format!("{y:04}.{m:02}.{d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_epoch_to_ymd_epoch_start() {
        assert_eq!(epoch_to_ymd(0), (1970, 1, 1));
    }

    #[test]
    fn test_epoch_to_ymd_known_date() {
        // 2000-01-01 = 946684800
        let (y, m, d) = epoch_to_ymd(946684800);
        assert_eq!((y, m, d), (2000, 1, 1));
    }

    #[test]
    fn test_epoch_to_iso() {
        assert_eq!(epoch_to_iso(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn test_relative_time_just_now() {
        let now = now_epoch();
        let ts = epoch_to_iso(now);
        assert_eq!(relative_time(&ts), "just now");
    }

    #[test]
    fn test_relative_time_minutes() {
        let now = now_epoch();
        let ts = epoch_to_iso(now - 300);
        assert_eq!(relative_time(&ts), "5m");
    }

    #[test]
    fn test_relative_time_hours() {
        let now = now_epoch();
        let ts = epoch_to_iso(now - 7200);
        assert_eq!(relative_time(&ts), "2h");
    }

    #[test]
    fn test_relative_time_days() {
        let now = now_epoch();
        let ts = epoch_to_iso(now - 86400 * 3);
        assert_eq!(relative_time(&ts), "3d");
    }

    #[test]
    fn test_is_stale_recent() {
        let now = now_epoch();
        let ts = epoch_to_iso(now - 86400);
        assert!(!is_stale(&ts, 7 * 86400));
    }

    #[test]
    fn test_is_stale_old() {
        let now = now_epoch();
        let ts = epoch_to_iso(now - 86400 * 10);
        assert!(is_stale(&ts, 7 * 86400));
    }

    #[test]
    fn test_seconds_ago_valid() {
        let now = now_epoch();
        let ts = epoch_to_iso(now - 60);
        let secs = seconds_ago(&ts).unwrap();
        assert!((secs - 60).abs() <= 1);
    }

    #[test]
    fn test_seconds_ago_invalid() {
        assert!(seconds_ago("not-a-date").is_none());
        assert!(seconds_ago("short").is_none());
    }
}
