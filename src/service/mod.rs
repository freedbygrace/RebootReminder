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

use std::process::Command;
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
static mut RUNNING_AS_SERVICE: bool = false;

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
        account_name: None, // Use LocalSystem account by default
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

    // Configure service recovery options using SC.exe
    // This sets the service to restart on the first, second, and subsequent failures
    info!("Configuring service recovery options");
    let output = Command::new("sc")
        .args([
            "failure",
            &name,
            "reset=0",
            "actions=restart/60000/restart/60000/restart/60000"
        ])
        .output()
        .context("Failed to execute SC command for recovery options")?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        warn!("Failed to set service recovery options: {}", error);
        // Continue even if this fails, as it's not critical
    } else {
        info!("Service recovery options configured successfully");
    }

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

/// Check if running as a service
pub fn is_running_as_service() -> bool {
    unsafe { RUNNING_AS_SERVICE }
}

/// Run the service directly without going through the service control manager
fn run_service_directly(_config: Config, _db_pool: DbPool) -> Result<()> {
    info!("Running service directly (not as a Windows service)");

    // Set the config path for the run_service function
    unsafe {
        CONFIG_PATH = Some(PathBuf::from("config.json"));
    }

    // The database is already initialized in the main function
    // We just need to make sure the configuration is set correctly

    // Call the run_service function directly
    run_service()
}

/// Run the service
pub fn run(config: Config, db_pool: DbPool) -> Result<()> {
    info!("Starting service initialization");

    // Set global state
    unsafe {
        SERVICE_RUNNING = true;
        RUNNING_AS_SERVICE = true;
    }

    info!("Global state initialized");

    // Try to start the service dispatcher
    info!("Starting service dispatcher");
    match service_dispatcher::start(SERVICE_NAME, ffi_service_main) {
        Ok(_) => {
            info!("Service dispatcher started successfully");
            Ok(())
        },
        Err(e) => {
            // Check if the error is ERROR_FAILED_SERVICE_CONTROLLER_CONNECT (1063)
            // This error occurs when the program is not started as a service
            let error_code = match &e {
                windows_service::Error::Winapi(io_error) => io_error.raw_os_error(),
                _ => None,
            };

            if let Some(code) = error_code {
                if code == 1063 {
                    warn!("Not running as a Windows service (error 1063), falling back to direct execution");
                    // Set the global flag to indicate we're not running as a service
                    unsafe {
                        RUNNING_AS_SERVICE = false;
                    }
                    // Fall back to direct execution
                    run_service_directly(config, db_pool)
                } else {
                    error!("Failed to start service dispatcher: {} (os error {})", e, code);
                    Err(anyhow::anyhow!("Failed to start service dispatcher: {} (os error {})", e, code))
                }
            } else {
                error!("Failed to start service dispatcher: {}", e);
                Err(anyhow::anyhow!("Failed to start service dispatcher: {}", e))
            }
        }
    }
}

/// Helper function to update service status with checkpoint
fn update_service_status(
    status_handle: &windows_service::service_control_handler::ServiceStatusHandle,
    current_state: ServiceState,
    checkpoint: u32,
    wait_hint_secs: u32,
    controls_accepted: ServiceControlAccept,
) -> Result<()> {
    info!("Updating service status to {:?} (checkpoint: {}, wait_hint: {}s)", current_state, checkpoint, wait_hint_secs);
    status_handle
        .set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state,
            controls_accepted,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint,
            wait_hint: std::time::Duration::from_secs(wait_hint_secs as u64),
            process_id: None,
        })
        .context("Failed to set service status")
}

/// Service main function
// This function is replaced by the define_windows_service! macro
// The actual implementation is below, but it's not used directly
#[allow(dead_code)]
fn service_main_impl(arguments: Vec<OsString>) {
    info!("Service main function started with {} arguments", arguments.len());

    // Log the arguments
    for (i, arg) in arguments.iter().enumerate() {
        info!("Service argument {}: {:?}", i, arg);
    }

    // Register the service control handler
    info!("Registering service control handler");
    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Stop => {
                info!("Service stop requested");
                unsafe {
                    SERVICE_RUNNING = false;
                }
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => {
                info!("Service interrogate requested");
                ServiceControlHandlerResult::NoError
            },
            _ => {
                info!("Unhandled service control event: {:?}", control_event);
                ServiceControlHandlerResult::NotImplemented
            },
        }
    };

    info!("Calling service_control_handler::register");
    let status_handle = match service_control_handler::register(SERVICE_NAME, event_handler) {
        Ok(handle) => {
            info!("Service control handler registered successfully");
            handle
        },
        Err(e) => {
            error!("Failed to register service control handler: {}", e);
            return;
        }
    };

    // Tell the service manager we are starting
    if let Err(e) = update_service_status(&status_handle, ServiceState::StartPending, 0, 60, ServiceControlAccept::empty()) {
        error!("Failed to set service status to StartPending: {}", e);
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
    if let Err(e) = update_service_status(&status_handle, ServiceState::Stopped, 0, 0, ServiceControlAccept::empty()) {
        error!("Failed to set service status to Stopped: {}", e);
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
    info!("Starting service initialization in run_service");

    // Create a status handle for updating service status
    let status_handle = match service_control_handler::register(SERVICE_NAME, |control_event| {
        match control_event {
            ServiceControl::Stop => {
                info!("Service stop requested");
                unsafe {
                    SERVICE_RUNNING = false;
                }
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => {
                debug!("Service interrogate requested");
                ServiceControlHandlerResult::NoError
            },
            ServiceControl::PowerEvent(event_type) => {
                debug!("Power event received: {:?}", event_type);
                // Handle power events like sleep/resume
                ServiceControlHandlerResult::NoError
            },
            ServiceControl::SessionChange(session_change) => {
                debug!("Session change event received: {:?}", session_change);
                // Handle session changes like user logon/logoff
                ServiceControlHandlerResult::NoError
            },
            _ => {
                debug!("Unhandled service control event: {:?}", control_event);
                ServiceControlHandlerResult::NotImplemented
            },
        }
    }) {
        Ok(handle) => {
            info!("Service control handler registered successfully");
            handle
        },
        Err(e) => {
            error!("Failed to register service control handler in run_service: {}", e);
            // If we're not running as a service, we can continue without the service control handler
            if !unsafe { RUNNING_AS_SERVICE } {
                info!("Not running as a service, continuing without service control handler");
                return Ok(());
            }
            return Err(anyhow::anyhow!("Failed to register service control handler: {}", e));
        }
    };

    // Set initial status to StartPending
    if let Err(e) = update_service_status(&status_handle, ServiceState::StartPending, 1, 120, ServiceControlAccept::empty()) {
        error!("Failed to set initial service status: {}", e);
        // Continue anyway, as this might not be fatal
    }

    // Load configuration
    info!("Determining configuration path");
    // Update status to indicate progress
    let _ = update_service_status(&status_handle, ServiceState::StartPending, 2, 120, ServiceControlAccept::empty());
    #[allow(static_mut_refs)]
    let config_path = unsafe { CONFIG_PATH.clone() }.unwrap_or_else(|| {
        info!("No configuration path set, using default");
        let mut path = match std::env::current_exe() {
            Ok(exe_path) => {
                info!("Executable path: {:?}", exe_path);
                match exe_path.parent() {
                    Some(parent) => {
                        info!("Executable directory: {:?}", parent);
                        parent.to_path_buf()
                    },
                    None => {
                        error!("Failed to get executable directory, using current directory");
                        PathBuf::from(".")
                    }
                }
            },
            Err(e) => {
                error!("Failed to get executable path: {}, using current directory", e);
                PathBuf::from(".")
            }
        };
        path.push("config.json");
        info!("Default configuration path: {:?}", path);
        path
    });

    info!("Loading configuration from {:?}", config_path);
    let config = match config::load(&config_path) {
        Ok(cfg) => {
            info!("Configuration loaded successfully");
            cfg
        },
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            return Err(e.into());
        }
    };
    info!("Configuration loaded from {:?}", config_path);
    // Update status to indicate progress
    let _ = update_service_status(&status_handle, ServiceState::StartPending, 3, 120, ServiceControlAccept::empty());

    // Create necessary directories
    info!("Creating necessary directories");
    match ensure_directories_exist(&config) {
        Ok(_) => info!("Directories created successfully"),
        Err(e) => {
            error!("Failed to create necessary directories: {}", e);
            return Err(e.into());
        }
    }

    // Initialize database
    info!("Initializing database at {}", config.database.path);
    // Update status to indicate progress
    let _ = update_service_status(&status_handle, ServiceState::StartPending, 4, 120, ServiceControlAccept::empty());
    let db_pool = match database::init(&config.database) {
        Ok(pool) => {
            info!("Database initialized successfully");
            pool
        },
        Err(e) => {
            error!("Failed to initialize database: {}", e);
            return Err(e.into());
        }
    };

    // Create impersonator
    let impersonator = Arc::new(Impersonator::new());
    // Update status to indicate progress
    let _ = update_service_status(&status_handle, ServiceState::StartPending, 5, 120, ServiceControlAccept::empty());

    // Create notification manager
    let mut notification_manager = NotificationManager::new(
        &config,
        db_pool.clone(),
        impersonator.clone(),
    );
    // Update status to indicate progress
    let _ = update_service_status(&status_handle, ServiceState::StartPending, 6, 120, ServiceControlAccept::empty());
    notification_manager
        .initialize()
        .context("Failed to initialize notification manager")?;
    let notification_manager = Arc::new(Mutex::new(notification_manager));

    // Create reboot detector
    let detector = RebootDetector::new(&config.reboot);
    // Update status to indicate progress
    let _ = update_service_status(&status_handle, ServiceState::StartPending, 7, 120, ServiceControlAccept::empty());

    // Create reboot history manager
    let history_manager = RebootHistoryManager::new(config.reboot.clone(), db_pool.clone());
    // Update status to indicate progress
    let _ = update_service_status(&status_handle, ServiceState::StartPending, 8, 120, ServiceControlAccept::empty());

    // Create and start watchdog if enabled
    // Update status to indicate progress
    let _ = update_service_status(&status_handle, ServiceState::StartPending, 9, 120, ServiceControlAccept::empty());
    if config.watchdog.enabled {
        info!("Initializing watchdog service");
        // Get check interval from either timespan or legacy field
        let check_interval_seconds = if let Some(check_interval) = &config.watchdog.check_interval {
            // Parse the timespan string
            match crate::utils::timespan::parse_timespan(check_interval) {
                Ok(duration) => duration.as_secs(),
                Err(e) => {
                    warn!("Failed to parse check interval timespan: {}", e);
                    // Fall back to the legacy value or default
                    config.watchdog.check_interval_seconds.unwrap_or(60)
                }
            }
        } else {
            // Use the legacy value or default
            config.watchdog.check_interval_seconds.unwrap_or(60)
        };

        // Get restart delay from either timespan or legacy field
        let restart_delay_seconds = if let Some(restart_delay) = &config.watchdog.restart_delay {
            // Parse the timespan string
            match crate::utils::timespan::parse_timespan(restart_delay) {
                Ok(duration) => duration.as_secs(),
                Err(e) => {
                    warn!("Failed to parse restart delay timespan: {}", e);
                    // Fall back to the legacy value or default
                    config.watchdog.restart_delay_seconds.unwrap_or(10)
                }
            }
        } else {
            // Use the legacy value or default
            config.watchdog.restart_delay_seconds.unwrap_or(10)
        };

        let mut watchdog_config = crate::watchdog::WatchdogConfig {
            enabled: config.watchdog.enabled,
            check_interval_seconds: check_interval_seconds,
            check_interval: config.watchdog.check_interval.clone(),
            max_restart_attempts: config.watchdog.max_restart_attempts,
            restart_delay_seconds: restart_delay_seconds,
            restart_delay: config.watchdog.restart_delay.clone(),
            service_path: PathBuf::from(config.watchdog.service_path.clone()),
            service_name: config.watchdog.service_name.clone(),
            power_checker: None,
        };

        // If service path is not specified, use the current executable path
        if watchdog_config.service_path.as_os_str().is_empty() {
            watchdog_config.service_path = std::env::current_exe()
                .expect("Failed to get executable path");
        }

        let mut watchdog = crate::watchdog::Watchdog::new(watchdog_config);
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

    // Update status to indicate progress - final checkpoint before Running
    let _ = update_service_status(&status_handle, ServiceState::StartPending, 10, 120, ServiceControlAccept::empty());

    // Set service status to Running
    if let Err(e) = update_service_status(&status_handle, ServiceState::Running, 0, 0, ServiceControlAccept::STOP) {
        error!("Failed to set service status to Running: {}", e);
        // Continue anyway, as this might not be fatal
    } else {
        info!("Service status set to Running successfully");
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
                // Get min hours from the first timeframe
                let min_hours = if let Some(min_timespan) = &config.reboot.timeframes[0].min_timespan {
                    match crate::utils::timespan::parse_timespan(min_timespan) {
                        Ok(duration) => (duration.as_secs() / 3600) as i64,
                        Err(_) => config.reboot.timeframes[0].min_hours.unwrap_or(24) as i64
                    }
                } else {
                    config.reboot.timeframes[0].min_hours.unwrap_or(24) as i64
                };

                if now - last_check >= Duration::minutes(min_hours * 60) {
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

    // Tell the service manager we are running - this is a second registration
    // that will be used for the main service loop, not the initialization
    let status_handle = match service_control_handler::register(SERVICE_NAME, |control_event| {
        match control_event {
            ServiceControl::Stop => {
                info!("Service stop requested");
                unsafe {
                    SERVICE_RUNNING = false;
                }
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => {
                debug!("Service interrogate requested");
                ServiceControlHandlerResult::NoError
            },
            ServiceControl::PowerEvent(event_type) => {
                debug!("Power event received: {:?}", event_type);
                // Handle power events like sleep/resume
                ServiceControlHandlerResult::NoError
            },
            ServiceControl::SessionChange(session_change) => {
                debug!("Session change event received: {:?}", session_change);
                // Handle session changes like user logon/logoff
                ServiceControlHandlerResult::NoError
            },
            _ => {
                debug!("Unhandled service control event: {:?}", control_event);
                ServiceControlHandlerResult::NotImplemented
            },
        }
    }) {
        Ok(handle) => {
            info!("Service control handler registered successfully");
            handle
        },
        Err(e) => {
            error!("Failed to register service control handler: {}", e);
            // If we're not running as a service, we can continue without the service control handler
            if !unsafe { RUNNING_AS_SERVICE } {
                info!("Not running as a service, continuing without service control handler");
                return Ok(());
            }
            return Err(anyhow::anyhow!("Failed to register service control handler: {}", e));
        }
    };

    // Set the service status to Running using our helper function
    update_service_status(&status_handle, ServiceState::Running, 0, 0, ServiceControlAccept::STOP)
        .context("Failed to set service status to Running")?;

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
                notification_type: Some(NotificationType::Both),
                show_toast: true,
                show_tray: true,
                show_balloon: false,
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
                check_interval_seconds: Some(60),
                check_interval: Some("1m".to_string()),
                max_restart_attempts: 3,
                restart_delay_seconds: Some(10),
                restart_delay: Some("10s".to_string()),
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
