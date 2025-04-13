use anyhow::{Context, Result};
use log::{debug, info, warn, error};
use std::process::Command;
use std::thread;
use std::time::Duration;
use windows::Win32::System::Shutdown::{ExitWindowsEx, EWX_REBOOT, SHUTDOWN_REASON};
use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONQUESTION, MB_YESNO, MB_DEFBUTTON2, IDYES};
use windows::core::PCWSTR;

/// Configuration for system reboot
#[derive(Debug, Clone)]
pub struct RebootConfig {
    /// Countdown duration in seconds
    pub countdown_seconds: u32,
    
    /// Whether to show a confirmation dialog
    pub show_confirmation: bool,
    
    /// Confirmation message
    pub confirmation_message: String,
    
    /// Confirmation title
    pub confirmation_title: String,
}

impl Default for RebootConfig {
    fn default() -> Self {
        Self {
            countdown_seconds: 30,
            show_confirmation: true,
            confirmation_message: "The system needs to restart. Do you want to restart now?".to_string(),
            confirmation_title: "System Restart Required".to_string(),
        }
    }
}

/// Initiate a system reboot with confirmation and countdown
pub fn reboot_system(config: &RebootConfig) -> Result<bool> {
    info!("Initiating system reboot process");
    
    // Show confirmation dialog if configured
    if config.show_confirmation {
        info!("Showing reboot confirmation dialog");
        
        // Convert strings to wide strings for Windows API
        let message_wide: Vec<u16> = config.confirmation_message.encode_utf16().chain(std::iter::once(0)).collect();
        let title_wide: Vec<u16> = config.confirmation_title.encode_utf16().chain(std::iter::once(0)).collect();
        
        // Show message box
        let result = unsafe {
            MessageBoxW(
                None,
                PCWSTR::from_raw(message_wide.as_ptr()),
                PCWSTR::from_raw(title_wide.as_ptr()),
                MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON2,
            )
        };
        
        // Check if user confirmed
        if result != IDYES {
            info!("User declined system reboot");
            return Ok(false);
        }
        
        info!("User confirmed system reboot");
    }
    
    // If countdown is enabled, show countdown dialog
    if config.countdown_seconds > 0 {
        info!("Starting reboot countdown: {} seconds", config.countdown_seconds);
        
        // Create countdown message
        let countdown_message = format!(
            "The system will restart in {} seconds. Please save your work and close applications.",
            config.countdown_seconds
        );
        
        // Show countdown notification
        // This is a simple implementation - in a real app, you might want to show a GUI countdown
        let countdown_wide: Vec<u16> = countdown_message.encode_utf16().chain(std::iter::once(0)).collect();
        let title_wide: Vec<u16> = "System Restarting".encode_utf16().chain(std::iter::once(0)).collect();
        
        unsafe {
            MessageBoxW(
                None,
                PCWSTR::from_raw(countdown_wide.as_ptr()),
                PCWSTR::from_raw(title_wide.as_ptr()),
                MB_ICONQUESTION,
            );
        }
        
        // Wait for the countdown
        for i in (1..=config.countdown_seconds).rev() {
            if i % 5 == 0 || i <= 5 {
                debug!("Reboot countdown: {} seconds remaining", i);
            }
            thread::sleep(Duration::from_secs(1));
        }
    }
    
    // Perform the actual reboot
    info!("Executing system reboot");
    
    // Try using Windows API first
    let result = unsafe {
        ExitWindowsEx(
            EWX_REBOOT,
            SHUTDOWN_REASON(0), // No specific reason code
        )
    };
    
    if let Err(e) = result {
        warn!("Failed to reboot using Windows API: {}", e);
        
        // Fall back to using shutdown.exe command
        info!("Attempting to reboot using shutdown.exe command");
        match Command::new("shutdown")
            .args(&["/r", "/t", "0", "/f"])
            .output() {
                Ok(_) => {
                    info!("System reboot initiated successfully using shutdown.exe");
                    Ok(true)
                },
                Err(e) => {
                    error!("Failed to reboot using shutdown.exe: {}", e);
                    Err(e).context("Failed to initiate system reboot")
                }
            }
    } else {
        info!("System reboot initiated successfully using Windows API");
        Ok(true)
    }
}

/// Cancel a pending system reboot
pub fn cancel_reboot() -> Result<()> {
    info!("Cancelling pending system reboot");
    
    // Use shutdown.exe to abort a pending shutdown
    match Command::new("shutdown")
        .args(&["/a"])
        .output() {
            Ok(_) => {
                info!("Pending system reboot cancelled successfully");
                Ok(())
            },
            Err(e) => {
                error!("Failed to cancel pending reboot: {}", e);
                Err(e).context("Failed to cancel pending system reboot")
            }
        }
}
