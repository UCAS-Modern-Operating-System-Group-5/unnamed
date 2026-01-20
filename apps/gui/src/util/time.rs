use chrono::{DateTime, Local};

pub fn timestamp_to_local_string(timestamp_secs: i64) -> String {
    let datetime = DateTime::from_timestamp(timestamp_secs, 0)
        .unwrap_or_default();
    let local_time = datetime.with_timezone(&Local);
    
    local_time.format("%Y-%m-%d %H:%M:%S").to_string()
}
