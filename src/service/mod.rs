use crate::config::{self, Config};
use crate::database::{self, DbPool, RebootState};
use crate::impersonation::Impersonator;
use crate::notification::NotificationManager;
use crate::reboot::{self, detector::RebootDetector, history::RebootHistoryManager};
use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use log::{debug, error, info, warn};
use std::path::{Path, PathBuf};
use std::ffi::OsString;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time;
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
};

const SERVICE_NAME: &str = "RebootReminder";
// These constants are used when installing the service
#[allow(dead_code)]
const SERVICE_DISPLAY_NAME: &str = "Reboot Reminder Service";
#[allow(dead_code)]
const SERVICE_DESCRIPTION: &str = "Provides notifications when system reboots are necessary";

// Global state
static mut CONFIG_PATH: Option<PathBuf> = None;
static mut SERVICE_RUNNING: bool = false;

/// Set the configuration file path for the service
pub unsafe fn set_config_path(path: PathBuf) {
    CONFIG_PATH = Some(path);
}

// Service entry point
// Fix service_main signature to match what define_windows_service! expects
fn service_main(_arguments: Vec<OsString>) {
    // Implementation will be provided by define_windows_service! macro
}

define_windows_service!(ffi_service_main, service_main);

/// Install the service
pub fn install(name: &str, display_name: &str, description: &str) -> Result<()> {
    info!("Installing service: {}", name);

    // Get the path to the executable
    let exe_path = std::env::current_exe().context("Failed to get executable path")?;

    // Create the service
    let service_info = windows_service::service::ServiceInfo {
        name: name.to_string().into(),
        display_name: display_name.to_string().into(),
        service_type: ServiceType::OWN_PROCESS,
        start_type: windows_service::service::ServiceStartType::AutoStart,
        error_control: windows_service::service::ServiceErrorControl::Normal,
        executable_path: exe_path,
        launch_arguments: vec!["run".to_string().into()],
        dependencies: vec![],
        account_name: Some("NT AUTHORITY\\SYSTEM".to_string().into()),
        account_password: None,
    };

    // Create the service manager
    let service_manager = windows_service::service_manager::ServiceManager::local_computer(
        None::<&str>,
        windows_service::service_manager::ServiceManagerAccess::CREATE_SERVICE,
    )
    .context("Failed to create service manager")?;

    // Create the service
    let service = service_manager
        .create_service(&service_info, windows_service::service::ServiceAccess::CHANGE_CONFIG)
        .context("Failed to create service")?;

    // Set the service description
    service
        .set_description(description)
        .context("Failed to set service description")?;

    info!("Service installed successfully");
    Ok(())
}

/// Uninstall the service
pub fn uninstall() -> Result<()> {
    info!("Uninstalling service");

    // Create the service manager
    let service_manager = windows_service::service_manager::ServiceManager::local_computer(
        None::<&str>,
        windows_service::service_manager::ServiceManagerAccess::CONNECT,
    )
    .context("Failed to create service manager")?;

    // Open the service
    let service = service_manager
        .open_service(
            SERVICE_NAME,
            windows_service::service::ServiceAccess::DELETE,
        )
        .context("Failed to open service")?;

    // Delete the service
    service.delete().context("Failed to delete service")?;

    info!("Service uninstalled successfully");
    Ok(())
}

/// Run the service
pub fn run(_config: Config, _db_pool: DbPool) -> Result<()> {
    info!("Running service");

    // Set global state
    unsafe {
        SERVICE_RUNNING = true;
    }

    // Start the service dispatcher
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)
        .context("Failed to start service dispatcher")?;

    Ok(())
}

/// Service main function
// This function is replaced by the define_windows_service! macro
// The actual implementation is below, but it's not used directly
#[allow(dead_code)]
fn service_main_impl(_arguments: Vec<OsString>) {
    info!("Service main function started");

    // Register the service control handler
    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Stop => {
                info!("Service stop requested");
                unsafe {
                    SERVICE_RUNNING = false;
                }
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    let status_handle = match service_control_handler::register(SERVICE_NAME, event_handler) {
        Ok(handle) => handle,
        Err(e) => {
            error!("Failed to register service control handler: {}", e);
            return;
        }
    };

    // Tell the service manager we are starting
    if let Err(e) = status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::StartPending,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: std::time::Duration::from_secs(10),
        process_id: None,
    }) {
        error!("Failed to set service status: {}", e);
        return;
    }

    // Run the service
    match run_service() {
        Ok(_) => {
            info!("Service completed successfully");
        }
        Err(e) => {
            error!("Service failed: {}", e);
        }
    }

    // Tell the service manager we are stopped
    if let Err(e) = status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: std::time::Duration::default(),
        process_id: None,
    }) {
        error!("Failed to set service status: {}", e);
    }
}

/// Ensure that all necessary directories exist
fn ensure_directories_exist(config: &Config) -> Result<()> {
    debug!("Ensuring necessary directories exist");

    // Create directory for database
    let db_path = Path::new(&config.database.path);
    if let Some(parent) = db_path.parent() {
        if !parent.exists() {
            debug!("Creating database directory: {:?}", parent);
            std::fs::create_dir_all(parent).context("Failed to create database directory")?;
        }
    }

    // Create directory for logs
    let log_path = Path::new(&config.logging.path);
    if let Some(parent) = log_path.parent() {
        if !parent.exists() {
            debug!("Creating log directory: {:?}", parent);
            std::fs::create_dir_all(parent).context("Failed to create log directory")?;
        }
    }

    // Create directory for icon if it doesn't exist
    let icon_path = Path::new(&config.notification.branding.icon_path);
    if let Some(parent) = icon_path.parent() {
        if !parent.exists() {
            debug!("Creating icon directory: {:?}", parent);
            std::fs::create_dir_all(parent).context("Failed to create icon directory")?;
        }
    }

    Ok(())
}

/// Run the service
fn run_service() -> Result<()> {
    info!("Starting service");

    // Load configuration
    #[allow(static_mut_refs)]
    let config_path = unsafe { CONFIG_PATH.clone() }.unwrap_or_else(|| {
        let mut path = std::env::current_exe()
            .expect("Failed to get executable path")
            .parent()
            .expect("Failed to get executable directory")
            .to_path_buf();
        path.push("config.json");
        path
    });

    info!("Using configuration file: {:?}", config_path);

    // Validate the configuration path
    let path_str = config_path.to_string_lossy();
    if path_str.contains(':') && !config_path.is_absolute() {
        // Check for unsupported URL schemes
        if path_str.starts_with("file://") {
            error!("File URL scheme is not supported. Use a local path instead.");
            return Err(anyhow::anyhow!("Unsupported URL scheme: file://"));
        } else if !path_str.starts_with("http://") && !path_str.starts_with("https://") {
            error!("Unsupported URL scheme in configuration path: {}", path_str);
            return Err(anyhow::anyhow!("Unsupported URL scheme: {}", path_str));
        }
    }

    let config = config::load(&config_path).context("Failed to load configuration")?;
    info!("Configuration loaded from {:?}", config_path);

    // Create necessary directories
    ensure_directories_exist(&config).context("Failed to create necessary directories")?;

    // Initialize database
    let db_pool = database::init(&config.database).context("Failed to initialize database")?;
    info!("Database initialized");

    // Create impersonator
    let impersonator = Arc::new(Impersonator::new());

    // Create notification manager
    let mut notification_manager = NotificationManager::new(
        &config,
        db_pool.clone(),
        impersonator.clone(),
    );
    notification_manager
        .initialize()
        .context("Failed to initialize notification manager")?;
    let notification_manager = Arc::new(Mutex::new(notification_manager));

    // Create reboot detector
    let detector = RebootDetector::new(&config.reboot);

    // Create reboot history manager
    let history_manager = RebootHistoryManager::new(config.reboot.clone(), db_pool.clone());

    // Create and start watchdog if enabled
    if config.watchdog.enabled {
        info!("Initializing watchdog service");
        let mut watchdog_config = crate::watchdog::WatchdogConfig {
            enabled: config.watchdog.enabled,
            check_interval_seconds: config.watchdog.check_interval_seconds,
            max_restart_attempts: config.watchdog.max_restart_attempts,
            restart_delay_seconds: config.watchdog.restart_delay_seconds,
            service_path: PathBuf::from(config.watchdog.service_path.clone()),
            service_name: config.watchdog.service_name.clone(),
        };

        // If service path is not specified, use the current executable path
        if watchdog_config.service_path.as_os_str().is_empty() {
            watchdog_config.service_path = std::env::current_exe()
                .expect("Failed to get executable path");
        }

        let watchdog = crate::watchdog::Watchdog::new(watchdog_config);
        if let Err(e) = watchdog.start() {
            warn!("Failed to start watchdog service: {}", e);
        } else {
            info!("Watchdog service started");
        }
    } else {
        debug!("Watchdog service is disabled");
    }

    // Scan event log for reboot history
    if let Err(e) = history_manager.get_reboot_history_from_event_log(10) {
        warn!("Failed to scan event log for reboot history: {}", e);
    }

    // Get system info
    match detector.get_system_info() {
        Ok(info) => {
            info!("System info: {:?}", info);
        }
        Err(e) => {
            warn!("Failed to get system info: {}", e);
        }
    }

    // Create shared configuration
    let shared_config = Arc::new(RwLock::new(config.clone()));

    // Create thread for configuration refresh
    let config_refresh_thread = {
        let shared_config = shared_config.clone();
        let config_path = config_path.clone();
        let config_refresh_minutes = config.service.config_refresh_minutes;

        thread::spawn(move || {
            let mut last_refresh = Utc::now();

            loop {
                // Check if service is still running
                if unsafe { !SERVICE_RUNNING } {
                    break;
                }

                // Check if it's time to refresh the configuration
                let now = Utc::now();
                if now - last_refresh >= Duration::minutes(config_refresh_minutes as i64) {
                    debug!("Refreshing configuration");

                    // Load configuration
                    match config::load(&config_path) {
                        Ok(new_config) => {
                            // Update shared configuration
                            if let Ok(mut config) = shared_config.write() {
                                *config = new_config;
                                info!("Configuration refreshed successfully");
                            } else {
                                error!("Failed to acquire write lock for configuration");
                            }

                            last_refresh = now;
                        }
                        Err(e) => {
                            error!("Failed to refresh configuration: {}", e);
                        }
                    }
                }

                // Sleep for a minute
                thread::sleep(time::Duration::from_secs(60));
            }
        })
    };

    // Create thread for checking if a reboot is required
    let reboot_check_thread = {
        let shared_config = shared_config.clone();
        let db_pool = db_pool.clone();
        let notification_manager = notification_manager.clone();

        thread::spawn(move || {
            let mut last_check = Utc::now();

            loop {
                // Check if service is still running
                if unsafe { !SERVICE_RUNNING } {
                    break;
                }

                // Get configuration
                let config = match shared_config.read() {
                    Ok(config) => config.clone(),
                    Err(e) => {
                        error!("Failed to acquire read lock for configuration: {}", e);
                        thread::sleep(time::Duration::from_secs(60));
                        continue;
                    }
                };

                // Check if it's time to check if a reboot is required
                let now = Utc::now();
                if now - last_check >= Duration::minutes(config.reboot.timeframes[0].min_hours as i64 * 60) {
                    debug!("Checking if a reboot is required");

                    // Create detector with current configuration
                    let detector = RebootDetector::new(&config.reboot);

                    // Check if a reboot is required
                    match detector.check_reboot_required() {
                        Ok((required, sources)) => {
                            // Get current reboot state
                            let state = match database::get_reboot_state(&db_pool) {
                                Ok(Some(state)) => state,
                                Ok(None) => {
                                    // Create new state
                                    RebootState::new(required, false)
                                }
                                Err(e) => {
                                    error!("Failed to get reboot state: {}", e);
                                    continue;
                                }
                            };

                            // Update reboot state
                            let mut new_state = state.clone();

                            // If reboot status changed, update accordingly
                            if !new_state.reboot_required && required {
                                // Reboot is now required but wasn't before
                                info!("Reboot requirement detected for the first time");
                                new_state.reboot_required_since = Some(now);
                            } else if new_state.reboot_required && !required {
                                // Reboot is no longer required (likely after a reboot)
                                info!("Reboot is no longer required - system was likely rebooted");
                                new_state.reboot_required_since = None;
                            }

                            new_state.reboot_required = required;
                            new_state.last_check_time = now;
                            new_state.updated_at = now;

                            // Update sources
                            new_state.sources = sources;

                            // Log how long reboot has been required if applicable
                            if required {
                                if let Some(required_since) = new_state.reboot_required_since {
                                    let duration = now.signed_duration_since(required_since);
                                    let hours = duration.num_hours();
                                    let minutes = duration.num_minutes() % 60;
                                    info!("Reboot has been required for {} hours and {} minutes (since {})",
                                          hours, minutes, required_since);
                                }
                            }

                            // If reboot is required, show notification
                            if required && now >= state.next_reminder_time.unwrap_or(now) {
                                // Get appropriate timeframe
                                if let Some(timeframe) = reboot::get_timeframe(&config.reboot, &new_state) {
                                    // Calculate next reminder time
                                    let next_reminder_time = if let Some(hours) = timeframe.reminder_interval_hours {
                                        now + Duration::hours(hours as i64)
                                    } else if let Some(minutes) = timeframe.reminder_interval_minutes {
                                        now + Duration::minutes(minutes as i64)
                                    } else {
                                        now + Duration::hours(1)
                                    };

                                    new_state.next_reminder_time = Some(next_reminder_time);

                                    // Show notification
                                    if let Ok(manager) = notification_manager.lock() {
                                        let message = config.notification.messages.reboot_required.clone();

                                        // Create reboot action if system reboots are enabled
                                        let action = if config.reboot.system_reboot.enabled {
                                            Some("reboot:now".to_string())
                                        } else {
                                            Some(config.notification.messages.action_required.clone())
                                        };

                                        if let Err(e) = manager.show_notification("reboot_required", &message, action.as_deref()) {
                                            error!("Failed to show notification: {}", e);
                                        }

                                        // Update tray status
                                        if let Err(e) = manager.update_tray_status("Reboot Required") {
                                            error!("Failed to update tray status: {}", e);
                                        }

                                        // Enable reboot and postpone options
                                        if let Err(e) = manager.enable_reboot_option(true) {
                                            error!("Failed to enable reboot option: {}", e);
                                        }

                                        if let Err(e) = manager.enable_postpone_option(true) {
                                            error!("Failed to enable postpone option: {}", e);
                                        }

                                        // Set deferral options
                                        if let Err(e) = manager.set_deferral_options(&timeframe.deferrals) {
                                            error!("Failed to set deferral options: {}", e);
                                        }
                                    }
                                }
                            } else if !required {
                                // Reset next reminder time
                                new_state.next_reminder_time = None;

                                // Update tray status
                                if let Ok(manager) = notification_manager.lock() {
                                    if let Err(e) = manager.update_tray_status("No Reboot Required") {
                                        error!("Failed to update tray status: {}", e);
                                    }

                                    // Disable reboot and postpone options
                                    if let Err(e) = manager.enable_reboot_option(false) {
                                        error!("Failed to disable reboot option: {}", e);
                                    }

                                    if let Err(e) = manager.enable_postpone_option(false) {
                                        error!("Failed to disable postpone option: {}", e);
                                    }
                                }
                            }

                            // Save reboot state
                            if let Err(e) = database::save_reboot_state(&db_pool, &new_state) {
                                error!("Failed to save reboot state: {}", e);
                            }

                            last_check = now;
                        }
                        Err(e) => {
                            error!("Failed to check if reboot is required: {}", e);
                        }
                    }
                }

                // Sleep for a minute
                thread::sleep(time::Duration::from_secs(60));
            }
        })
    };

    // Tell the service manager we are running
    let status_handle = service_control_handler::register(SERVICE_NAME, |_| {
        ServiceControlHandlerResult::NoError
    })
    .context("Failed to register service control handler")?;

    status_handle
        .set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::Running,
            controls_accepted: ServiceControlAccept::STOP,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: std::time::Duration::default(),
            process_id: None,
        })
        .context("Failed to set service status")?;

    // Wait for service to stop
    while unsafe { SERVICE_RUNNING } {
        thread::sleep(time::Duration::from_secs(1));
    }

    // Wait for threads to finish
    config_refresh_thread
        .join()
        .map_err(|_| anyhow::anyhow!("Failed to join configuration refresh thread"))?;

    reboot_check_thread
        .join()
        .map_err(|_| anyhow::anyhow!("Failed to join reboot check thread"))?;

    info!("Service stopped");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, DatabaseConfig, LoggingConfig, NotificationConfig, BrandingConfig, ServiceConfig, RebootConfig, DetectionMethodsConfig, NotificationType, QuietHoursConfig, MessagesConfig, WatchdogConfig};
    use tempfile::tempdir;

    #[test]
    fn test_ensure_directories_exist() {
        // Create a temporary directory for the test
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();

        // Create test paths
        let db_dir = temp_path.join("db");
        let log_dir = temp_path.join("logs");
        let icon_dir = temp_path.join("icons");

        let db_path = db_dir.join("test.db").to_string_lossy().to_string();
        let log_path = log_dir.join("test.log").to_string_lossy().to_string();
        let icon_path = icon_dir.join("test.ico").to_string_lossy().to_string();

        // Create a test configuration
        let config = Config {
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
                    icon_path: icon_path,
                    company: "Test Company".to_string(),
                },
                messages: MessagesConfig::default(),
                quiet_hours: QuietHoursConfig::default(),
            },
            reboot: RebootConfig {
                timeframes: vec![],
                detection_methods: DetectionMethodsConfig::default(),
                system_reboot: config::models::default_system_reboot_config(),
            },
            database: DatabaseConfig {
                path: db_path,
            },
            logging: LoggingConfig {
                path: log_path,
                level: "info".to_string(),
                max_files: 5,
                max_size: 10,
            },
            watchdog: WatchdogConfig {
                enabled: true,
                check_interval_seconds: 60,
                max_restart_attempts: 3,
                restart_delay_seconds: 10,
                service_path: "".to_string(),
                service_name: "TestService".to_string(),
            },
        };

        // Ensure directories exist
        let result = ensure_directories_exist(&config);
        assert!(result.is_ok());

        // Check that directories were created
        assert!(db_dir.exists());
        assert!(log_dir.exists());
        assert!(icon_dir.exists());
    }
}
