use chrono::{DateTime, Utc};

#[must_use]
pub fn format_timestamp(seconds: i64) -> String {
    DateTime::<Utc>::from_timestamp(seconds, 0).map_or_else(
        || "Unknown".to_string(),
        |dt| dt.format("%Y-%m-%d %H:%M").to_string(),
    )
}
