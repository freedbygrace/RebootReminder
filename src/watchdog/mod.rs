use anyhow::Result;
use log::{debug, error, info, warn};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use windows::Win32::System::Services::{
    CloseServiceHandle, ControlService, OpenSCManagerW, OpenServiceW, QueryServiceStatus,
    SC_MANAGER_CONNECT, SERVICE_CONTROL_STOP, SERVICE_QUERY_STATUS, SERVICE_START,
    SERVICE_STATUS, StartServiceW,
};
use windows::core::PCWSTR;

/// Watchdog service configuration
#[derive(Debug, Clone)]
pub struct WatchdogConfig {
    /// Whether the watchdog is enabled
    pub enabled: bool,

    /// Interval in seconds between health checks
    pub check_interval_seconds: u64,

    /// Maximum number of restart attempts
    pub max_restart_attempts: u32,

    /// Delay in seconds between restart attempts
    pub restart_delay_seconds: u64,

    /// Path to the main service executable
    pub service_path: PathBuf,

    /// Name of the main service
    pub service_name: String,
}

impl Default for WatchdogConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            check_interval_seconds: 60,
            max_restart_attempts: 3,
            restart_delay_seconds: 10,
            service_path: PathBuf::new(),
            service_name: "RebootReminder".to_string(),
        }
    }
}

/// Watchdog service
pub struct Watchdog {
    config: WatchdogConfig,
    running: Arc<AtomicBool>,
}

impl Watchdog {
    /// Create a new watchdog
    pub fn new(config: WatchdogConfig) -> Self {
        Self {
            config,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start the watchdog
    pub fn start(&self) -> Result<()> {
        if !self.config.enabled {
            info!("Watchdog is disabled, not starting");
            return Ok(());
        }

        info!("Starting watchdog service");

        // Set running flag
        self.running.store(true, Ordering::SeqCst);

        // Clone values for the thread
        let config = self.config.clone();
        let running = self.running.clone();

        // Start watchdog thread
        thread::spawn(move || {
            let mut restart_attempts = 0;

            while running.load(Ordering::SeqCst) {
                // Check if the main service is running
                match is_service_running(&config.service_name) {
                    Ok(true) => {
                        debug!("Main service is running");
                        // Reset restart attempts if service is running
                        restart_attempts = 0;
                    }
                    Ok(false) => {
                        warn!("Main service is not running");

                        // Check if we've exceeded the maximum restart attempts
                        if restart_attempts >= config.max_restart_attempts {
                            error!("Maximum restart attempts ({}) reached, giving up", config.max_restart_attempts);
                            break;
                        }

                        // Attempt to restart the service
                        info!("Attempting to restart main service (attempt {}/{})",
                             restart_attempts + 1, config.max_restart_attempts);

                        match restart_service(&config.service_name) {
                            Ok(()) => {
                                info!("Successfully restarted main service");
                                restart_attempts += 1;
                            }
                            Err(e) => {
                                error!("Failed to restart main service: {}", e);
                                restart_attempts += 1;
                            }
                        }

                        // Wait before checking again
                        thread::sleep(Duration::from_secs(config.restart_delay_seconds));
                    }
                    Err(e) => {
                        error!("Failed to check if main service is running: {}", e);
                    }
                }

                // Wait for the next check
                thread::sleep(Duration::from_secs(config.check_interval_seconds));
            }

            info!("Watchdog thread exiting");
        });

        info!("Watchdog service started");
        Ok(())
    }

    /// Stop the watchdog
    pub fn stop(&self) -> Result<()> {
        info!("Stopping watchdog service");

        // Clear running flag
        self.running.store(false, Ordering::SeqCst);

        info!("Watchdog service stopped");
        Ok(())
    }
}

/// Check if a service is running
fn is_service_running(service_name: &str) -> Result<bool> {
    unsafe {
        // Open the service control manager
        let sc_manager = OpenSCManagerW(
            PCWSTR::null(),
            PCWSTR::null(),
            SC_MANAGER_CONNECT,
        )?;

        // Convert service name to wide string
        let service_name_wide: Vec<u16> = service_name.encode_utf16().chain(std::iter::once(0)).collect();

        // Open the service
        let service = OpenServiceW(
            sc_manager,
            PCWSTR::from_raw(service_name_wide.as_ptr()),
            SERVICE_QUERY_STATUS,
        )?;

        // Close the service control manager when we're done with it
        let _ = CloseServiceHandle(sc_manager);

        // Query the service status
        let mut status = SERVICE_STATUS::default();
        let result = QueryServiceStatus(service, &mut status);

        // Close the service handle when we're done with it
        let _ = CloseServiceHandle(service);

        // Check the result
        match result {
            Ok(_) => {
                // Check if the service is running
                Ok(status.dwCurrentState == windows::Win32::System::Services::SERVICE_RUNNING)
            },
            Err(e) => {
                Err(anyhow::anyhow!("Failed to query service status: {}", e))
            }
        }
    }
}

/// Restart a service
fn restart_service(service_name: &str) -> Result<()> {
    unsafe {
        // Open the service control manager
        let sc_manager = OpenSCManagerW(
            PCWSTR::null(),
            PCWSTR::null(),
            SC_MANAGER_CONNECT,
        )?;

        // Convert service name to wide string
        let service_name_wide: Vec<u16> = service_name.encode_utf16().chain(std::iter::once(0)).collect();

        // Open the service with stop and start permissions
        let service = OpenServiceW(
            sc_manager,
            PCWSTR::from_raw(service_name_wide.as_ptr()),
            SERVICE_QUERY_STATUS | windows::Win32::System::Services::SERVICE_STOP | SERVICE_START,
        )?;

        // Close the service control manager when we're done with it
        let _ = CloseServiceHandle(sc_manager);

        // Query the service status
        let mut status = SERVICE_STATUS::default();
        let result = QueryServiceStatus(service, &mut status);

        if let Err(e) = result {
            let _ = CloseServiceHandle(service);
            return Err(anyhow::anyhow!("Failed to query service status: {}", e));
        }

        // Stop the service if it's running
        if status.dwCurrentState == windows::Win32::System::Services::SERVICE_RUNNING {
            info!("Stopping service {}", service_name);

            let result = ControlService(service, SERVICE_CONTROL_STOP, &mut status);

            if let Err(e) = result {
                let _ = CloseServiceHandle(service);
                return Err(anyhow::anyhow!("Failed to stop service: {}", e));
            }

            // Wait for the service to stop
            let mut attempts = 0;
            while status.dwCurrentState != windows::Win32::System::Services::SERVICE_STOPPED {
                if attempts >= 30 {
                    let _ = CloseServiceHandle(service);
                    return Err(anyhow::anyhow!("Timeout waiting for service to stop"));
                }

                thread::sleep(Duration::from_secs(1));

                let result = QueryServiceStatus(service, &mut status);

                if let Err(e) = result {
                    let _ = CloseServiceHandle(service);
                    return Err(anyhow::anyhow!("Failed to query service status: {}", e));
                }

                attempts += 1;
            }

            info!("Service {} stopped", service_name);
        }

        // Start the service
        info!("Starting service {}", service_name);

        let result = StartServiceW(service, None);

        if let Err(e) = result {
            let _ = CloseServiceHandle(service);
            return Err(anyhow::anyhow!("Failed to start service: {}", e));
        }

        // Wait for the service to start
        let mut attempts = 0;
        while status.dwCurrentState != windows::Win32::System::Services::SERVICE_RUNNING {
            if attempts >= 30 {
                let _ = CloseServiceHandle(service);
                return Err(anyhow::anyhow!("Timeout waiting for service to start"));
            }

            thread::sleep(Duration::from_secs(1));

            let result = QueryServiceStatus(service, &mut status);

            if let Err(e) = result {
                let _ = CloseServiceHandle(service);
                return Err(anyhow::anyhow!("Failed to query service status: {}", e));
            }

            attempts += 1;
        }

        // Close the service handle when we're done with it
        let _ = CloseServiceHandle(service);

        info!("Service {} started", service_name);
        Ok(())
    }
}
