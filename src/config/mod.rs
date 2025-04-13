pub mod models;

use anyhow::{Context, Result};
use log::{debug, info, warn};
use reqwest::blocking::Client;
use std::fs;
use std::path::Path;
use std::time::Duration;
use url::Url;

use crate::utils::expand_env_vars;

pub use models::*;


/// Load configuration from a file or URL
pub fn load<P: AsRef<Path>>(path: P) -> Result<Config> {
    let path = path.as_ref();
    debug!("Loading configuration from {:?}", path);

    // Convert path to string for analysis
    let path_str = path.to_string_lossy();

    let content = if is_url(&path_str) {
        // Load from URL (http/https only)
        info!("Loading configuration from URL: {}", path_str);
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        let response = client
            .get(path_str.as_ref())
            .send()
            .context("Failed to fetch configuration from URL")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to fetch configuration from URL: HTTP {}",
                response.status()
            ));
        }

        response.text().context("Failed to read configuration from URL")?
    } else {
        // Handle UNC paths and local paths
        let path_to_use = if path_str.starts_with("\\\\") {
            // UNC path
            info!("Loading configuration from UNC path: {}", path_str);
            path
        } else if path_str.contains(':') && !path.is_absolute() {
            // Might be a file:// URL or similar that we don't support
            return Err(anyhow::anyhow!("Unsupported URL scheme in path: {}", path_str));
        } else {
            // Regular local path
            info!("Loading configuration from local file: {:?}", path);
            path
        };

        fs::read_to_string(path_to_use).context("Failed to read configuration file")?
    };

    // Determine format based on file extension or content
    let mut config = if path.extension().map_or(false, |ext| ext == "json") || is_json(&content) {
        // Parse JSON
        debug!("Parsing JSON configuration");
        serde_json::from_str::<Config>(&content).context("Failed to parse JSON configuration")?
    } else if path.extension().map_or(false, |ext| ext == "xml") || is_xml(&content) {
        // Parse XML
        debug!("Parsing XML configuration");
        quick_xml::de::from_str::<Config>(&content).context("Failed to parse XML configuration")?
    } else {
        // Try JSON first, then XML
        debug!("Trying to parse configuration as JSON or XML");
        match serde_json::from_str::<Config>(&content) {
            Ok(config) => config,
            Err(json_err) => {
                warn!("Failed to parse as JSON: {}", json_err);
                quick_xml::de::from_str::<Config>(&content)
                    .context("Failed to parse configuration as JSON or XML")?
            }
        }
    };

    // Log the loaded configuration
    info!("Loaded configuration: {}", format_config_summary(&config));

    // Expand environment variables in paths
    expand_env_vars_in_config(&mut config).context("Failed to expand environment variables in configuration")?;

    // Log the expanded configuration paths
    info!("Expanded configuration paths:");
    info!("  Database path: {}", config.database.path);
    info!("  Logging path: {}", config.logging.path);
    info!("  Icon path: {}", config.notification.branding.icon_path);

    // Validate configuration
    validate_config(&config).context("Invalid configuration")?;

    debug!("Configuration loaded successfully");
    Ok(config)
}

/// Save configuration to a file
pub fn save<P: AsRef<Path>>(config: &Config, path: P) -> Result<()> {
    let path = path.as_ref();
    debug!("Saving configuration to {:?}", path);

    // Create parent directory if it doesn't exist
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("Failed to create parent directory")?;
    }

    // Determine format based on file extension
    let content = if path.extension().map_or(false, |ext| ext == "json") {
        // Generate JSON
        debug!("Generating JSON configuration");
        serde_json::to_string_pretty(config).context("Failed to generate JSON configuration")?
    } else if path.extension().map_or(false, |ext| ext == "xml") {
        // Generate XML
        debug!("Generating XML configuration");
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str(
            &quick_xml::se::to_string(config).context("Failed to generate XML configuration")?,
        );
        xml
    } else {
        // Default to JSON
        debug!("Defaulting to JSON configuration");
        serde_json::to_string_pretty(config).context("Failed to generate JSON configuration")?
    };

    // Write to file
    fs::write(path, content).context("Failed to write configuration file")?;
    info!("Configuration saved to {:?}", path);

    Ok(())
}

/// Get default configuration
pub fn default() -> Config {
    Config {
        service: ServiceConfig {
            name: "RebootReminder".to_string(),
            display_name: "Reboot Reminder Service".to_string(),
            description: "Provides notifications when system reboots are necessary".to_string(),
            config_refresh_minutes: 60,
        },
        notification: NotificationConfig {
            notification_type: NotificationType::Both,
            branding: BrandingConfig {
                title: "Reboot Reminder".to_string(),
                icon_path: "icon.ico".to_string(),
                company: "IT Department".to_string(),
            },
            messages: MessagesConfig {
                reboot_required: "Your computer requires a reboot to complete recent updates.".to_string(),
                reboot_recommended: "It is recommended to reboot your computer to apply recent updates.".to_string(),
                reboot_scheduled: "Your computer is scheduled to reboot at %s.".to_string(),
                reboot_in_progress: "Your computer will reboot in %s.".to_string(),
                reboot_cancelled: "The scheduled reboot has been cancelled.".to_string(),
                reboot_postponed: "The reboot has been postponed for %s.".to_string(),
                reboot_completed: "Your computer has been successfully rebooted.".to_string(),
                action_required: "Reboot is required. Click to schedule.".to_string(),
                action_recommended: "Reboot is recommended. Click for options.".to_string(),
                action_not_required: "No reboot is required at this time.".to_string(),
                action_not_available: "Reboot options are not available at this time.".to_string(),
            },
            quiet_hours: QuietHoursConfig {
                enabled: true,
                start_time: "22:00".to_string(),
                end_time: "08:00".to_string(),
                days_of_week: vec![0, 1, 2, 3, 4, 5, 6],
            },
        },
        reboot: RebootConfig {
            timeframes: vec![
                TimeframeConfig {
                    min_hours: 24,
                    max_hours: Some(48),
                    reminder_interval_hours: Some(4),
                    reminder_interval_minutes: None,
                    reminder_interval: Some("4h".to_string()),
                    deferrals: vec!["1h".to_string(), "4h".to_string(), "8h".to_string(), "24h".to_string()],
                },
                TimeframeConfig {
                    min_hours: 49,
                    max_hours: Some(72),
                    reminder_interval_hours: Some(2),
                    reminder_interval_minutes: None,
                    reminder_interval: Some("2h".to_string()),
                    deferrals: vec!["1h".to_string(), "2h".to_string(), "4h".to_string()],
                },
                TimeframeConfig {
                    min_hours: 73,
                    max_hours: None,
                    reminder_interval_hours: None,
                    reminder_interval_minutes: Some(30),
                    reminder_interval: Some("30m".to_string()),
                    deferrals: vec!["30m".to_string(), "1h".to_string()],
                },
            ],
            detection_methods: DetectionMethodsConfig {
                windows_update: true,
                sccm: true,
                registry: true,
                pending_file_operations: true,
            },
            system_reboot: default_system_reboot_config(),
        },
        database: DatabaseConfig {
            path: "rebootreminder.db".to_string(),
        },
        logging: LoggingConfig {
            path: "logs/rebootreminder.log".to_string(),
            level: "info".to_string(),
            max_files: 7,
            max_size: 10,
        },
        watchdog: WatchdogConfig::default(),
    }
}

/// Format a summary of the configuration for logging
fn format_config_summary(config: &Config) -> String {
    let mut summary = String::new();

    // Service info
    summary.push_str(&format!("Service: {}, ", config.service.name));

    // Timeframes info
    summary.push_str(&format!("Timeframes: {}, ", config.reboot.timeframes.len()));

    // First timeframe info
    if let Some(first_timeframe) = config.reboot.timeframes.first() {
        summary.push_str(&format!("First timeframe: {}h-", first_timeframe.min_hours));
        if let Some(max_hours) = first_timeframe.max_hours {
            summary.push_str(&format!("{}", max_hours));
        } else {
            summary.push_str("∞");
        }
        summary.push_str("h, ");

        if let Some(interval) = &first_timeframe.reminder_interval {
            summary.push_str(&format!("Reminder interval: {}, ", interval));
        } else if let Some(hours) = first_timeframe.reminder_interval_hours {
            summary.push_str(&format!("Reminder interval: {}h, ", hours));
        } else if let Some(minutes) = first_timeframe.reminder_interval_minutes {
            summary.push_str(&format!("Reminder interval: {}m, ", minutes));
        }
    }

    // Detection methods
    summary.push_str("Detection methods: ");
    if config.reboot.detection_methods.windows_update {
        summary.push_str("WinUpdate ");
    }
    if config.reboot.detection_methods.sccm {
        summary.push_str("SCCM ");
    }
    if config.reboot.detection_methods.registry {
        summary.push_str("Registry ");
    }
    if config.reboot.detection_methods.pending_file_operations {
        summary.push_str("FileOps ");
    }

    summary
}

/// Validate configuration
fn validate_config(config: &Config) -> Result<()> {
    // Validate service configuration
    if config.service.name.is_empty() {
        return Err(anyhow::anyhow!("Service name cannot be empty"));
    }
    if config.service.display_name.is_empty() {
        return Err(anyhow::anyhow!("Service display name cannot be empty"));
    }
    if config.service.config_refresh_minutes == 0 {
        return Err(anyhow::anyhow!("Config refresh minutes must be greater than 0"));
    }

    // Validate notification configuration
    if config.notification.branding.title.is_empty() {
        return Err(anyhow::anyhow!("Notification title cannot be empty"));
    }
    if config.notification.branding.icon_path.is_empty() {
        return Err(anyhow::anyhow!("Notification icon path cannot be empty"));
    }

    // Validate quiet hours
    if config.notification.quiet_hours.enabled {
        // Validate time format (HH:MM)
        if !is_valid_time_format(&config.notification.quiet_hours.start_time) {
            return Err(anyhow::anyhow!(
                "Invalid quiet hours start time format: {}. Expected HH:MM",
                config.notification.quiet_hours.start_time
            ));
        }
        if !is_valid_time_format(&config.notification.quiet_hours.end_time) {
            return Err(anyhow::anyhow!(
                "Invalid quiet hours end time format: {}. Expected HH:MM",
                config.notification.quiet_hours.end_time
            ));
        }

        // Validate days of week (0-6)
        for day in &config.notification.quiet_hours.days_of_week {
            if *day > 6 {
                return Err(anyhow::anyhow!(
                    "Invalid day of week: {}. Expected 0-6",
                    day
                ));
            }
        }
    }

    // Validate reboot timeframes
    if config.reboot.timeframes.is_empty() {
        return Err(anyhow::anyhow!("At least one reboot timeframe must be defined"));
    }
    for (i, timeframe) in config.reboot.timeframes.iter().enumerate() {
        if timeframe.min_hours >= timeframe.max_hours.unwrap_or(u32::MAX) {
            return Err(anyhow::anyhow!(
                "Timeframe {}: min_hours must be less than max_hours",
                i
            ));
        }
        if timeframe.reminder_interval_hours.is_none() && timeframe.reminder_interval_minutes.is_none() {
            return Err(anyhow::anyhow!(
                "Timeframe {}: Either reminder_interval_hours or reminder_interval_minutes must be specified",
                i
            ));
        }
        if timeframe.deferrals.is_empty() {
            return Err(anyhow::anyhow!(
                "Timeframe {}: At least one deferral option must be defined",
                i
            ));
        }
        for deferral in &timeframe.deferrals {
            if !is_valid_duration_format(deferral) {
                return Err(anyhow::anyhow!(
                    "Timeframe {}: Invalid deferral format: {}. Expected format: 1h, 30m, etc.",
                    i,
                    deferral
                ));
            }
        }
    }

    // Validate database configuration
    if config.database.path.is_empty() {
        return Err(anyhow::anyhow!("Database path cannot be empty"));
    }

    // Validate logging configuration
    if config.logging.path.is_empty() {
        return Err(anyhow::anyhow!("Logging path cannot be empty"));
    }
    if config.logging.max_files == 0 {
        return Err(anyhow::anyhow!("Max log files must be greater than 0"));
    }
    if config.logging.max_size == 0 {
        return Err(anyhow::anyhow!("Max log size must be greater than 0"));
    }
    if !["trace", "debug", "info", "warn", "error"].contains(&config.logging.level.to_lowercase().as_str()) {
        return Err(anyhow::anyhow!(
            "Invalid log level: {}. Expected trace, debug, info, warn, or error",
            config.logging.level
        ));
    }

    Ok(())
}

/// Check if a string is a URL
fn is_url(s: &str) -> bool {
    // Only consider http and https URLs as valid for remote configuration
    if let Ok(url) = Url::parse(s) {
        matches!(url.scheme(), "http" | "https")
    } else {
        false
    }
}

/// Expand environment variables in configuration paths
fn expand_env_vars_in_config(config: &mut Config) -> Result<()> {
    debug!("Expanding environment variables in configuration paths");

    // Expand database path
    if config.database.path.contains('%') {
        config.database.path = expand_env_vars(&config.database.path)?;
        debug!("Expanded database path: {}", config.database.path);
    }

    // Expand logging path
    if config.logging.path.contains('%') {
        config.logging.path = expand_env_vars(&config.logging.path)?;
        debug!("Expanded logging path: {}", config.logging.path);
    }

    // Expand notification icon path
    if config.notification.branding.icon_path.contains('%') {
        config.notification.branding.icon_path = expand_env_vars(&config.notification.branding.icon_path)?;
        debug!("Expanded icon path: {}", config.notification.branding.icon_path);
    }

    // Expand watchdog service path if it's not empty
    if !config.watchdog.service_path.is_empty() && config.watchdog.service_path.contains('%') {
        config.watchdog.service_path = expand_env_vars(&config.watchdog.service_path)?;
        debug!("Expanded watchdog service path: {}", config.watchdog.service_path);
    }

    Ok(())
}

/// Check if content is JSON
fn is_json(content: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(content).is_ok()
}

/// Check if content is XML
fn is_xml(content: &str) -> bool {
    content.trim().starts_with("<?xml") || content.trim().starts_with("<")
}

/// Check if a string is a valid time format (HH:MM)
fn is_valid_time_format(s: &str) -> bool {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return false;
    }

    let hours = parts[0].parse::<u32>();
    let minutes = parts[1].parse::<u32>();

    if let (Ok(h), Ok(m)) = (hours, minutes) {
        h < 24 && m < 60
    } else {
        false
    }
}

/// Check if a string is a valid duration format (e.g., 1h, 30m)
fn is_valid_duration_format(s: &str) -> bool {
    let s = s.trim().to_lowercase();

    // Check for hours format (e.g., 1h)
    if s.ends_with('h') {
        if let Ok(hours) = s[..s.len() - 1].parse::<u32>() {
            return hours > 0;
        }
    }

    // Check for minutes format (e.g., 30m)
    if s.ends_with('m') {
        if let Ok(minutes) = s[..s.len() - 1].parse::<u32>() {
            return minutes > 0;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_is_url() {
        // Valid URLs
        assert!(is_url("http://example.com/config.json"));
        assert!(is_url("https://example.com/config.json"));

        // Invalid or unsupported URLs
        assert!(!is_url("file:///C:/config.json"));
        assert!(!is_url("ftp://example.com/config.json"));
        assert!(!is_url("C:\\Program Files\\config.json"));
        assert!(!is_url("\\\\server\\share\\config.json"));
        assert!(!is_url("config.json"));
    }

    #[test]
    fn test_load_local_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("config.json");

        // Create a simple config file
        let config_content = r#"{
            "service": {
                "name": "TestService",
                "displayName": "Test Service",
                "description": "Test Description",
                "configRefreshMinutes": 60
            },
            "notification": {
                "type": "both",
                "branding": {
                    "title": "Test Title",
                    "iconPath": "icon.ico",
                    "company": "Test Company"
                },
                "messages": {
                    "initialTitle": "Reboot Required",
                    "initialMessage": "A system reboot is required.",
                    "reminderTitle": "Reboot Still Required",
                    "reminderMessage": "A system reboot is still required.",
                    "urgentTitle": "Urgent: Reboot Required",
                    "urgentMessage": "A system reboot is urgently required."
                },
                "quietHours": {
                    "enabled": false,
                    "start": "22:00",
                    "end": "08:00"
                }
            },
            "reboot": {
                "timeframes": [],
                "detectionMethods": {
                    "windowsUpdate": true,
                    "sccm": true,
                    "registry": true,
                    "pendingFileOperations": true
                },
                "systemReboot": {
                    "enabled": true,
                    "countdownSeconds": 60,
                    "forceRebootAfterDeferrals": false
                }
            },
            "database": {
                "path": "test.db"
            },
            "logging": {
                "path": "test.log",
                "level": "info",
                "maxFiles": 5,
                "maxSize": 10
            },
            "watchdog": {
                "enabled": false,
                "checkIntervalSeconds": 60,
                "maxRestartAttempts": 3,
                "restartDelaySeconds": 10,
                "servicePath": "",
                "serviceName": ""
            }
        }"#;

        let mut file = File::create(&file_path).unwrap();
        file.write_all(config_content.as_bytes()).unwrap();

        // Test loading the config
        let result = load(&file_path);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.service.name, "TestService");
        assert_eq!(config.database.path, "test.db");
    }

    #[test]
    fn test_expand_env_vars_in_config() {
        // Create a test configuration with environment variables
        let mut config = Config {
            service: ServiceConfig {
                name: "TestService".to_string(),
                display_name: "Test Service".to_string(),
                description: "Test service description".to_string(),
                config_refresh_minutes: 60,
            },
            notification: NotificationConfig {
                notification_type: NotificationType::Both,
                branding: BrandingConfig {
                    title: "Test Title".to_string(),
                    icon_path: "%WINDIR%\\System32\\test.ico".to_string(),
                    company: "Test Company".to_string(),
                },
                messages: MessagesConfig::default(),
                quiet_hours: QuietHoursConfig::default(),
            },
            reboot: RebootConfig {
                timeframes: vec![],
                detection_methods: DetectionMethodsConfig::default(),
                system_reboot: models::default_system_reboot_config(),
            },
            database: DatabaseConfig {
                path: "%PROGRAMDATA%\\TestApp\\test.db".to_string(),
            },
            logging: LoggingConfig {
                path: "%TEMP%\\TestApp\\logs\\test.log".to_string(),
                level: "info".to_string(),
                max_files: 5,
                max_size: 10,
            },
            watchdog: WatchdogConfig {
                enabled: true,
                check_interval_seconds: 60,
                max_restart_attempts: 3,
                restart_delay_seconds: 10,
                service_path: "%PROGRAMFILES%\\TestApp\\test.exe".to_string(),
                service_name: "TestService".to_string(),
            },
        };

        // Expand environment variables
        let result = expand_env_vars_in_config(&mut config);
        assert!(result.is_ok());

        // Check that environment variables were expanded
        assert!(!config.database.path.contains("%PROGRAMDATA%"));
        assert!(!config.logging.path.contains("%TEMP%"));
        assert!(!config.notification.branding.icon_path.contains("%WINDIR%"));
        assert!(!config.watchdog.service_path.contains("%PROGRAMFILES%"));
    }

    #[test]
    fn test_is_valid_time_format() {
        assert!(is_valid_time_format("12:30"));
        assert!(is_valid_time_format("00:00"));
        assert!(is_valid_time_format("23:59"));

        assert!(!is_valid_time_format("24:00"));
        assert!(!is_valid_time_format("12:60"));
        assert!(!is_valid_time_format("12:30:00"));
        assert!(!is_valid_time_format("12-30"));
        assert!(!is_valid_time_format("abc"));
    }

    #[test]
    fn test_is_valid_duration_format() {
        assert!(is_valid_duration_format("1h"));
        assert!(is_valid_duration_format("24h"));
        assert!(is_valid_duration_format("30m"));
        assert!(is_valid_duration_format("120m"));

        assert!(!is_valid_duration_format("0h"));
        assert!(!is_valid_duration_format("0m"));
        assert!(!is_valid_duration_format("1d"));
        assert!(!is_valid_duration_format("abc"));
    }
}
