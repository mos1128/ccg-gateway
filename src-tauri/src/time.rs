use chrono::{Local, TimeZone};

pub fn now_timestamp() -> i64 {
    Local::now().timestamp()
}

pub fn today_local_date() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

pub fn local_date_from_timestamp(timestamp: i64) -> String {
    Local
        .timestamp_opt(timestamp, 0)
        .single()
        .unwrap_or_else(Local::now)
        .format("%Y-%m-%d")
        .to_string()
}

pub fn local_compact_datetime() -> String {
    Local::now().format("%Y%m%d_%H%M%S").to_string()
}

pub fn local_datetime_millis() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string()
}
