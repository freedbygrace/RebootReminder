pub mod detector;
pub mod history;
pub mod system;

use crate::config::RebootConfig;
use crate::database::RebootState;
use crate::utils::timespan;
use anyhow::Result;
use chrono::Duration;
// No logging imports needed
use chrono::{DateTime, Utc};

/// Get the appropriate timeframe for a reboot state
pub fn get_timeframe<'a>(config: &'a RebootConfig, state: &RebootState) -> Option<&'a crate::config::TimeframeConfig> {
    // If no reboot is required, return None
    if !state.reboot_required {
        return None;
    }

    // Calculate hours since reboot required
    let hours_since_required = match state.sources.iter().min_by_key(|s| s.detected_at) {
        Some(source) => {
            let now = Utc::now();
            let duration = now.signed_duration_since(source.detected_at);
            duration.num_hours() as u32
        }
        None => 0,
    };

    // Find the appropriate timeframe
    for timeframe in &config.timeframes {
        let min_hours = timeframe.min_hours;
        let max_hours = timeframe.max_hours.unwrap_or(u32::MAX);

        if hours_since_required >= min_hours && hours_since_required < max_hours {
            return Some(timeframe);
        }
    }

    // If no timeframe matches, use the last one
    config.timeframes.last()
}

/// Calculate the next reminder time based on the timeframe
pub fn calculate_next_reminder_time(timeframe: &crate::config::TimeframeConfig, now: DateTime<Utc>) -> DateTime<Utc> {
    // First check if a timespan string is provided
    if let Some(interval) = &timeframe.reminder_interval {
        if let Ok(duration) = timespan::parse_timespan(interval) {
            return now + Duration::seconds(duration.as_secs() as i64);
        }
    }

    // Fall back to the old way
    if let Some(hours) = timeframe.reminder_interval_hours {
        now + Duration::hours(hours as i64)
    } else if let Some(minutes) = timeframe.reminder_interval_minutes {
        now + Duration::minutes(minutes as i64)
    } else {
        // Default to 1 hour if no interval is specified
        now + Duration::hours(1)
    }
}

/// Parse a deferral string (e.g., "1h", "30m") to a duration
pub fn parse_deferral(deferral: &str) -> Result<Duration> {
    // Use the timespan parser
    let std_duration = timespan::parse_timespan(deferral)?;
    Ok(Duration::seconds(std_duration.as_secs() as i64))
}

/// Format a duration in a human-readable format
pub fn format_duration(duration: Duration) -> String {
    // Convert chrono::Duration to std::time::Duration
    let std_duration = std::time::Duration::from_secs(duration.num_seconds().max(0) as u64);

    // Use the timespan formatter for compact representation
    let compact = timespan::format_timespan(std_duration);

    // For human-readable format, we'll still use the old code
    let total_seconds = duration.num_seconds();

    let human_readable = if total_seconds < 60 {
        format!("{} seconds", total_seconds)
    } else if total_seconds < 3600 {
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        if seconds == 0 {
            format!("{} minutes", minutes)
        } else {
            format!("{} minutes, {} seconds", minutes, seconds)
        }
    } else if total_seconds < 86400 {
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        if minutes == 0 {
            format!("{} hours", hours)
        } else {
            format!("{} hours, {} minutes", hours, minutes)
        }
    } else {
        let days = total_seconds / 86400;
        let hours = (total_seconds % 86400) / 3600;
        if hours == 0 {
            format!("{} days", days)
        } else {
            format!("{} days, {} hours", days, hours)
        }
    };

    // Return both formats
    format!("{} ({})", human_readable, compact)
}

/// Format a time in a human-readable format
pub fn format_time(time: DateTime<Utc>) -> String {
    time.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Format the time since last reboot in a human-readable format
pub fn format_time_since_last_reboot(last_reboot_time: Option<DateTime<Utc>>) -> String {
    match last_reboot_time {
        Some(time) => {
            let now = Utc::now();
            let duration = now.signed_duration_since(time);

            if duration.num_seconds() < 0 {
                "in the future (clock mismatch)".to_string()
            } else {
                format!("{} ago", format_duration(duration))
            }
        }
        None => "unknown".to_string(),
    }
}
