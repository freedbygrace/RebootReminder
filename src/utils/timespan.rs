use anyhow::{Context, Result};
use std::time::Duration;

/// Parse a timespan string into a Duration
/// 
/// Supports the following formats:
/// - 30m: 30 minutes
/// - 2h: 2 hours
/// - 1h30m: 1 hour and 30 minutes
/// 
/// # Arguments
/// 
/// * `timespan` - The timespan string to parse
/// 
/// # Returns
/// 
/// A Duration representing the timespan
/// 
/// # Examples
/// 
/// ```
/// use rebootreminder::utils::timespan::parse_timespan;
/// 
/// let duration = parse_timespan("30m").unwrap();
/// assert_eq!(duration, std::time::Duration::from_secs(30 * 60));
/// 
/// let duration = parse_timespan("2h").unwrap();
/// assert_eq!(duration, std::time::Duration::from_secs(2 * 60 * 60));
/// 
/// let duration = parse_timespan("1h30m").unwrap();
/// assert_eq!(duration, std::time::Duration::from_secs(90 * 60));
/// ```
pub fn parse_timespan(timespan: &str) -> Result<Duration> {
    let mut total_seconds = 0;
    let mut current_number = String::new();
    
    for c in timespan.chars() {
        if c.is_digit(10) {
            current_number.push(c);
        } else if c == 'h' || c == 'H' {
            let hours = current_number.parse::<u64>()
                .context(format!("Failed to parse hours from '{}'", current_number))?;
            total_seconds += hours * 60 * 60;
            current_number.clear();
        } else if c == 'm' || c == 'M' {
            let minutes = current_number.parse::<u64>()
                .context(format!("Failed to parse minutes from '{}'", current_number))?;
            total_seconds += minutes * 60;
            current_number.clear();
        } else {
            return Err(anyhow::anyhow!("Invalid character in timespan: '{}'", c));
        }
    }
    
    // If there are any remaining digits without a unit, assume minutes
    if !current_number.is_empty() {
        let minutes = current_number.parse::<u64>()
            .context(format!("Failed to parse minutes from '{}'", current_number))?;
        total_seconds += minutes * 60;
    }
    
    Ok(Duration::from_secs(total_seconds))
}

/// Format a Duration as a timespan string
/// 
/// # Arguments
/// 
/// * `duration` - The Duration to format
/// 
/// # Returns
/// 
/// A string representing the timespan
/// 
/// # Examples
/// 
/// ```
/// use rebootreminder::utils::timespan::format_timespan;
/// 
/// let timespan = format_timespan(std::time::Duration::from_secs(30 * 60));
/// assert_eq!(timespan, "30m");
/// 
/// let timespan = format_timespan(std::time::Duration::from_secs(2 * 60 * 60));
/// assert_eq!(timespan, "2h");
/// 
/// let timespan = format_timespan(std::time::Duration::from_secs(90 * 60));
/// assert_eq!(timespan, "1h30m");
/// ```
pub fn format_timespan(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    let hours = total_seconds / (60 * 60);
    let minutes = (total_seconds / 60) % 60;
    
    if hours > 0 {
        if minutes > 0 {
            format!("{}h{}m", hours, minutes)
        } else {
            format!("{}h", hours)
        }
    } else {
        format!("{}m", minutes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_timespan() {
        // Test minutes
        assert_eq!(parse_timespan("30m").unwrap(), Duration::from_secs(30 * 60));
        assert_eq!(parse_timespan("45M").unwrap(), Duration::from_secs(45 * 60));
        
        // Test hours
        assert_eq!(parse_timespan("2h").unwrap(), Duration::from_secs(2 * 60 * 60));
        assert_eq!(parse_timespan("3H").unwrap(), Duration::from_secs(3 * 60 * 60));
        
        // Test combined
        assert_eq!(parse_timespan("1h30m").unwrap(), Duration::from_secs(90 * 60));
        assert_eq!(parse_timespan("2H15M").unwrap(), Duration::from_secs(135 * 60));
        
        // Test without unit (should default to minutes)
        assert_eq!(parse_timespan("30").unwrap(), Duration::from_secs(30 * 60));
        
        // Test invalid
        assert!(parse_timespan("invalid").is_err());
        assert!(parse_timespan("30x").is_err());
    }
    
    #[test]
    fn test_format_timespan() {
        // Test minutes
        assert_eq!(format_timespan(Duration::from_secs(30 * 60)), "30m");
        assert_eq!(format_timespan(Duration::from_secs(45 * 60)), "45m");
        
        // Test hours
        assert_eq!(format_timespan(Duration::from_secs(2 * 60 * 60)), "2h");
        assert_eq!(format_timespan(Duration::from_secs(3 * 60 * 60)), "3h");
        
        // Test combined
        assert_eq!(format_timespan(Duration::from_secs(90 * 60)), "1h30m");
        assert_eq!(format_timespan(Duration::from_secs(135 * 60)), "2h15m");
    }
}
