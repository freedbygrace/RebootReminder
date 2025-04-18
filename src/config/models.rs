use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Log level
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Error level
    Error,
    /// Warning level
    Warn,
    /// Info level
    Info,
    /// Debug level
    Debug,
    /// Trace level
    Trace,
}

impl FromStr for LogLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "error" => Ok(LogLevel::Error),
            "warn" => Ok(LogLevel::Warn),
            "warning" => Ok(LogLevel::Warn),
            "info" => Ok(LogLevel::Info),
            "debug" => Ok(LogLevel::Debug),
            "trace" => Ok(LogLevel::Trace),
            _ => Err(format!("Unknown log level: {}", s)),
        }
    }
}

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    /// Service configuration
    pub service: ServiceConfig,

    /// Notification configuration
    pub notification: NotificationConfig,

    /// Reboot configuration
    pub reboot: RebootConfig,

    /// Database configuration
    pub database: DatabaseConfig,

    /// Logging configuration
    pub logging: LoggingConfig,

    /// Watchdog configuration
    #[serde(default)]
    pub watchdog: WatchdogConfig,
}

/// Service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceConfig {
    /// Service name
    pub name: String,

    /// Service display name
    pub display_name: String,

    /// Service description
    pub description: String,

    /// Configuration refresh interval in minutes
    pub config_refresh_minutes: u32,
}

/// Notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationConfig {
    /// Notification type (tray, toast, or both) - Legacy field
    #[serde(rename = "type", default = "default_notification_type", skip_serializing_if = "Option::is_none")]
    pub notification_type: Option<NotificationType>,

    /// Show toast notifications
    #[serde(default = "default_show_toast")]
    pub show_toast: bool,

    /// Show tray notifications
    #[serde(default = "default_show_tray")]
    pub show_tray: bool,

    /// Show balloon notifications
    #[serde(default = "default_show_balloon")]
    pub show_balloon: bool,

    /// Branding configuration
    pub branding: BrandingConfig,

    /// Message templates
    pub messages: MessagesConfig,

    /// Quiet hours configuration
    pub quiet_hours: QuietHoursConfig,
}

/// Notification type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NotificationType {
    /// Tray notifications only
    Tray,

    /// Toast notifications only
    Toast,

    /// Both tray and toast notifications
    Both,
}

/// Branding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrandingConfig {
    /// Notification title
    pub title: String,

    /// Path to icon file
    pub icon_path: String,

    /// Company name
    pub company: String,
}

/// Message templates
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MessagesConfig {
    /// Message shown when a reboot is required
    pub reboot_required: String,

    /// Message shown when a reboot is recommended
    pub reboot_recommended: String,

    /// Message shown when a reboot is scheduled
    pub reboot_scheduled: String,

    /// Message shown when a reboot is in progress
    pub reboot_in_progress: String,

    /// Message shown when a reboot is cancelled
    pub reboot_cancelled: String,

    /// Message shown when a reboot is postponed
    pub reboot_postponed: String,

    /// Message shown when a reboot is completed
    pub reboot_completed: String,

    /// Action message for required reboots
    pub action_required: String,

    /// Action message for recommended reboots
    pub action_recommended: String,

    /// Action message when no reboot is required
    pub action_not_required: String,

    /// Action message when reboot options are not available
    pub action_not_available: String,
}

/// Quiet hours configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct QuietHoursConfig {
    /// Whether quiet hours are enabled
    pub enabled: bool,

    /// Start time of quiet hours (HH:MM)
    pub start_time: String,

    /// End time of quiet hours (HH:MM)
    pub end_time: String,

    /// Days of the week when quiet hours are active (0 = Sunday, 6 = Saturday)
    pub days_of_week: Vec<u8>,
}

/// Reboot configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RebootConfig {
    /// Reboot timeframes
    pub timeframes: Vec<TimeframeConfig>,

    /// Reboot detection methods
    pub detection_methods: DetectionMethodsConfig,

    /// System reboot options
    #[serde(default = "default_system_reboot_config")]
    pub system_reboot: SystemRebootConfig,
}

/// Timeframe configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeframeConfig {
    /// Minimum hours since reboot required (legacy)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_hours: Option<u32>,

    /// Maximum hours since reboot required (None = no maximum) (legacy)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_hours: Option<u32>,

    /// Minimum time since reboot required as a timespan string (e.g., "24h")
    #[serde(rename = "min", default, skip_serializing_if = "Option::is_none")]
    pub min_timespan: Option<String>,

    /// Maximum time since reboot required as a timespan string (e.g., "48h")
    #[serde(rename = "max", default, skip_serializing_if = "Option::is_none")]
    pub max_timespan: Option<String>,

    /// Reminder interval in hours (legacy)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reminder_interval_hours: Option<u32>,

    /// Reminder interval in minutes (legacy)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reminder_interval_minutes: Option<u32>,

    /// Reminder interval as a timespan string (e.g., "1h30m")
    #[serde(rename = "reminderInterval", default, skip_serializing_if = "Option::is_none")]
    pub reminder_interval: Option<String>,

    /// Deferral options (e.g., "1h", "30m")
    pub deferrals: Vec<String>,
}

/// Detection methods configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DetectionMethodsConfig {
    /// Whether to check Windows Update for pending reboots
    pub windows_update: bool,

    /// Whether to check SCCM for pending reboots
    pub sccm: bool,

    /// Whether to check registry for pending reboots
    pub registry: bool,

    /// Whether to check for pending file operations
    pub pending_file_operations: bool,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseConfig {
    /// Path to database file
    pub path: String,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoggingConfig {
    /// Path to log file
    pub path: String,

    /// Log level (trace, debug, info, warn, error)
    pub level: String,

    /// Maximum number of log files to keep
    pub max_files: u32,

    /// Maximum size of each log file in MB
    pub max_size: u32,
}

/// Watchdog configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WatchdogConfig {
    /// Whether the watchdog is enabled
    #[serde(default = "default_watchdog_enabled")]
    pub enabled: bool,

    /// Interval in seconds between health checks (legacy)
    #[serde(default = "default_watchdog_check_interval_option", skip_serializing_if = "Option::is_none")]
    pub check_interval_seconds: Option<u64>,

    /// Check interval as a timespan string (e.g., "5m")
    #[serde(rename = "checkInterval", default, skip_serializing_if = "Option::is_none")]
    pub check_interval: Option<String>,

    /// Maximum number of restart attempts
    #[serde(default = "default_watchdog_max_restart_attempts")]
    pub max_restart_attempts: u32,

    /// Delay in seconds between restart attempts (legacy)
    #[serde(default = "default_watchdog_restart_delay_option", skip_serializing_if = "Option::is_none")]
    pub restart_delay_seconds: Option<u64>,

    /// Delay between restart attempts as a timespan string (e.g., "30s")
    #[serde(rename = "restartDelay", default, skip_serializing_if = "Option::is_none")]
    pub restart_delay: Option<String>,

    /// Path to the main service executable
    #[serde(default)]
    pub service_path: String,

    /// Name of the main service
    #[serde(default = "default_watchdog_service_name")]
    pub service_name: String,
}

/// Default value for watchdog enabled
fn default_watchdog_enabled() -> bool {
    true
}

/// Default value for watchdog check interval (unused)
#[allow(dead_code)]
fn default_watchdog_check_interval() -> u64 {
    60
}

/// Default value for watchdog check interval as Option
fn default_watchdog_check_interval_option() -> Option<u64> {
    Some(60)
}

/// Default value for watchdog max restart attempts
fn default_watchdog_max_restart_attempts() -> u32 {
    3
}

/// Default value for watchdog restart delay (unused)
#[allow(dead_code)]
fn default_watchdog_restart_delay() -> u64 {
    10
}

/// Default value for watchdog restart delay as Option
fn default_watchdog_restart_delay_option() -> Option<u64> {
    Some(10)
}

/// Default value for watchdog service name
fn default_watchdog_service_name() -> String {
    "RebootReminder".to_string()
}

/// System reboot configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemRebootConfig {
    /// Whether to allow users to initiate system reboots
    #[serde(default = "default_system_reboot_enabled")]
    pub enabled: bool,

    /// Countdown duration in seconds before reboot (legacy)
    #[serde(default = "default_system_reboot_countdown_option", skip_serializing_if = "Option::is_none")]
    pub countdown_seconds: Option<u32>,

    /// Countdown as a timespan string (e.g., "30s")
    #[serde(rename = "countdown", default, skip_serializing_if = "Option::is_none")]
    pub countdown: Option<String>,

    /// Whether to show a confirmation dialog
    #[serde(default = "default_system_reboot_confirmation")]
    pub show_confirmation: bool,

    /// Confirmation message
    #[serde(default = "default_system_reboot_message")]
    pub confirmation_message: String,

    /// Confirmation title
    #[serde(default = "default_system_reboot_title")]
    pub confirmation_title: String,
}

/// Default value for system reboot config
pub fn default_system_reboot_config() -> SystemRebootConfig {
    SystemRebootConfig {
        enabled: true,
        countdown_seconds: Some(30),
        countdown: Some("30s".to_string()),
        show_confirmation: true,
        confirmation_message: "The system needs to restart. Do you want to restart now?".to_string(),
        confirmation_title: "System Restart Required".to_string(),
    }
}

/// Default value for system reboot enabled
fn default_system_reboot_enabled() -> bool {
    true
}

/// Default value for system reboot countdown (unused)
#[allow(dead_code)]
fn default_system_reboot_countdown() -> u32 {
    30
}

/// Default value for system reboot countdown as Option
fn default_system_reboot_countdown_option() -> Option<u32> {
    Some(30)
}

/// Default value for system reboot confirmation
fn default_system_reboot_confirmation() -> bool {
    true
}

/// Default value for system reboot message
fn default_system_reboot_message() -> String {
    "The system needs to restart. Do you want to restart now?".to_string()
}

/// Default value for system reboot title
fn default_system_reboot_title() -> String {
    "System Restart Required".to_string()
}

/// Default notification type
pub fn default_notification_type() -> Option<NotificationType> {
    None
}

/// Default show toast
pub fn default_show_toast() -> bool {
    true
}

/// Default show tray
pub fn default_show_tray() -> bool {
    true
}

/// Default show balloon
pub fn default_show_balloon() -> bool {
    false
}
