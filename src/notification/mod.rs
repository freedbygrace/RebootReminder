pub mod toast;
mod tray;

use crate::config::{Config, NotificationConfig, NotificationType, SystemRebootConfig};
use crate::database::{DbPool, Notification, NotificationInteraction, UserSession};
use crate::impersonation::Impersonator;
use crate::service;
use anyhow::{Context, Result};
use chrono::{Datelike, NaiveTime, Utc, Weekday};
use log::{debug, info, warn, error};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
// use uuid::Uuid;

/// Notification manager
pub struct NotificationManager {
    config: NotificationConfig,
    system_reboot_config: SystemRebootConfig,
    db_pool: DbPool,
    impersonator: Arc<Impersonator>,
    tray_manager: Option<Arc<Mutex<tray::TrayManager>>>,
}

impl NotificationManager {
    /// Create a new notification manager
    pub fn new(
        config: &Config,
        db_pool: DbPool,
        impersonator: Arc<Impersonator>,
    ) -> Self {
        Self {
            config: config.notification.clone(),
            system_reboot_config: config.reboot.system_reboot.clone(),
            db_pool,
            impersonator,
            tray_manager: None,
        }
    }

    /// Initialize the notification manager
    pub fn initialize(&mut self) -> Result<()> {
        debug!("Initializing notification manager");

        // Initialize tray if needed and not running as a service
        if (self.config.notification_type == NotificationType::Tray
            || self.config.notification_type == NotificationType::Both)
            && !service::is_running_as_service()
        {
            debug!("Initializing tray manager");
            let icon_path = self.resolve_icon_path(&self.config.branding.icon_path)?;
            match tray::TrayManager::new(
                &self.config.branding.title,
                &icon_path,
                self.db_pool.clone(),
            ) {
                Ok(tray_manager) => {
                    self.tray_manager = Some(Arc::new(Mutex::new(tray_manager)));
                    info!("Tray manager initialized successfully");
                },
                Err(e) => {
                    warn!("Failed to initialize tray manager, continuing without tray: {}", e);
                }
            }
        } else if service::is_running_as_service() {
            info!("Running as a service, skipping tray initialization");
        }

        info!("Notification manager initialized");
        Ok(())
    }

    /// Show a notification
    pub fn show_notification(
        &self,
        notification_type: &str,
        message: &str,
        action: Option<&str>,
    ) -> Result<()> {
        info!("Preparing to show notification: type={}, action={:?}", notification_type, action);
        info!("Notification message: {}", message);

        // Check if we should show notifications (quiet hours)
        if self.is_quiet_hours() {
            info!("Not showing notification during quiet hours");
            info!("Quiet hours configuration: start={}, end={}, days={:?}",
                  self.config.quiet_hours.start_time,
                  self.config.quiet_hours.end_time,
                  self.config.quiet_hours.days_of_week);
            return Ok(());
        }

        // Check if there are any interactive sessions
        let sessions = self.impersonator.get_active_sessions()?;
        if sessions.is_empty() {
            info!("No interactive sessions found, not showing notification");
            return Ok(());
        }

        info!("Found {} active user sessions", sessions.len());
        for (i, session) in sessions.iter().enumerate() {
            info!("Session {}: user={}, id={}, type={}",
                  i + 1,
                  session.user_name,
                  session.session_id,
                  if session.is_console { "console" } else if session.is_rdp { "rdp" } else { "other" });
        }

        // Create notification record
        let notification = Notification::new(
            notification_type,
            message,
            sessions.first().map(|s| s.user_name.as_str()),
        );

        info!("Created notification: id={}, type={}, user={}",
              notification.id,
              notification.notification_type,
              notification.user_name.as_deref().unwrap_or("<unknown>"));

        // Set action if provided
        let mut notification = notification;
        if let Some(action_str) = action {
            notification.action = Some(action_str.to_string());
            info!("Added action to notification: {}", action_str);
        }

        // Save notification to database
        info!("Saving notification to database");
        match crate::database::add_notification(&self.db_pool, &notification) {
            Ok(_) => info!("Successfully saved notification to database"),
            Err(e) => {
                warn!("Failed to save notification to database: {}", e);
                return Err(e.context("Failed to save notification to database"));
            }
        };

        // Show notification based on type
        match self.config.notification_type {
            NotificationType::Tray => {
                self.show_tray_notification(&notification, &sessions[0])?;
            }
            NotificationType::Toast => {
                self.show_toast_notification(&notification, &sessions[0])?;
            }
            NotificationType::Both => {
                // Show both types
                if let Err(e) = self.show_tray_notification(&notification, &sessions[0]) {
                    warn!("Failed to show tray notification: {}", e);
                }

                if let Err(e) = self.show_toast_notification(&notification, &sessions[0]) {
                    warn!("Failed to show toast notification: {}", e);
                }
            }
        }

        info!("Notification successfully shown to user: {}", sessions[0].user_name);
        info!("Notification content: {}", message);
        Ok(())
    }

    /// Show a tray notification
    fn show_tray_notification(
        &self,
        notification: &Notification,
        _session: &UserSession,
    ) -> Result<()> {
        debug!("Showing tray notification");

        if let Some(tray_manager) = &self.tray_manager {
            let mut tray = tray_manager.lock().unwrap();
            // Tray doesn't support showing notifications directly
            // We'll just update the status instead
            tray.update_status(&notification.message)?;
        } else {
            warn!("Tray manager not initialized");
        }

        Ok(())
    }

    /// Show a toast notification
    fn show_toast_notification(
        &self,
        notification: &Notification,
        session: &UserSession,
    ) -> Result<()> {
        debug!("Showing toast notification");

        // Create toast notification
        let icon_path = self.resolve_icon_path(&self.config.branding.icon_path)?;
        let toast = toast::ToastNotification::new_with_icon(
            &self.config.branding.title,
            &notification.message,
            &icon_path,
            notification.id.clone(),
        );

        // Show notification using impersonation
        self.impersonator.show_toast_notification(session, &toast.message)
    }

    /// Record a notification interaction
    pub fn record_interaction(
        &self,
        notification_id: uuid::Uuid,
        action: &str,
        session: &UserSession,
    ) -> Result<()> {
        info!("Recording notification interaction: {} - {}", notification_id, action);
        info!("User: {}, Session: {}", session.user_name, session.session_id);

        // Create interaction record
        let mut interaction = NotificationInteraction::new(notification_id, action);
        interaction.user_name = Some(session.user_name.clone());
        interaction.session_id = Some(session.session_id.clone());

        // Check if this is a reboot action
        if action.starts_with("reboot:") {
            info!("Reboot action detected: {}", action);

            // Add details about the reboot action
            let details = format!("Reboot initiated by user {} from session {}",
                                 session.user_name, session.session_id);
            interaction.details = Some(details.clone());

            // Save to database before attempting reboot
            crate::database::add_notification_interaction(&self.db_pool, &interaction)
                .context("Failed to save notification interaction to database")?;

            info!("Processing reboot action: {}", action);

            // Handle the reboot action
            self.handle_reboot_action(action, session)
                .context("Failed to handle reboot action")?;

            return Ok(());
        }

        // Save to database
        crate::database::add_notification_interaction(&self.db_pool, &interaction)
            .context("Failed to save notification interaction to database")?;

        info!("Notification interaction recorded: {} - {}", notification_id, action);
        Ok(())
    }

    /// Handle a reboot action
    fn handle_reboot_action(&self, action: &str, session: &UserSession) -> Result<()> {
        info!("Handling reboot action: {}", action);
        info!("Initiated by user: {} (session: {})", session.user_name, session.session_id);

        // Parse the action to get parameters
        let parts: Vec<&str> = action.split(':').collect();
        if parts.len() < 2 {
            return Err(anyhow::anyhow!("Invalid reboot action format: {}", action));
        }

        // Get the reboot type
        let reboot_type = parts[1];
        info!("Reboot type: {}", reboot_type);

        // Create reboot configuration
        let reboot_config = crate::reboot::system::RebootConfig {
            countdown_seconds: self.system_reboot_config.countdown_seconds,
            show_confirmation: self.system_reboot_config.show_confirmation,
            confirmation_message: self.system_reboot_config.confirmation_message.clone(),
            confirmation_title: self.system_reboot_config.confirmation_title.clone(),
        };

        // Check if system reboots are enabled
        if !self.system_reboot_config.enabled {
            warn!("System reboot requested but feature is disabled in configuration");
            return Err(anyhow::anyhow!("System reboot feature is disabled"));
        }

        // Initiate the reboot
        info!("Initiating system reboot with countdown: {} seconds", reboot_config.countdown_seconds);
        match crate::reboot::system::reboot_system(&reboot_config) {
            Ok(confirmed) => {
                if confirmed {
                    info!("System reboot initiated successfully");
                    Ok(())
                } else {
                    info!("System reboot was cancelled by user");
                    Err(anyhow::anyhow!("Reboot cancelled by user"))
                }
            },
            Err(e) => {
                error!("Failed to initiate system reboot: {}", e);
                Err(e.context("Failed to initiate system reboot"))
            }
        }
    }

    /// Check if the current time is within quiet hours
    fn is_quiet_hours(&self) -> bool {
        if !self.config.quiet_hours.enabled {
            return false;
        }

        let now = Utc::now();
        let current_day = match now.weekday() {
            Weekday::Mon => 1,
            Weekday::Tue => 2,
            Weekday::Wed => 3,
            Weekday::Thu => 4,
            Weekday::Fri => 5,
            Weekday::Sat => 6,
            Weekday::Sun => 0,
        };

        // Check if current day is in quiet hours days
        let day_included = self.config.quiet_hours.days_of_week.contains(&(current_day as u8));
        if !day_included {
            return false;
        }

        // Parse quiet hours times
        let start_time = match NaiveTime::parse_from_str(&self.config.quiet_hours.start_time, "%H:%M") {
            Ok(time) => time,
            Err(e) => {
                warn!("Failed to parse quiet hours start time: {}", e);
                return false;
            }
        };

        let end_time = match NaiveTime::parse_from_str(&self.config.quiet_hours.end_time, "%H:%M") {
            Ok(time) => time,
            Err(e) => {
                warn!("Failed to parse quiet hours end time: {}", e);
                return false;
            }
        };

        // Get current time in local timezone
        let local_now = chrono::Local::now();
        let current_time = local_now.time();

        // Handle overnight quiet hours
        if start_time > end_time {
            // Quiet hours span midnight
            current_time >= start_time || current_time < end_time
        } else {
            // Normal quiet hours
            current_time >= start_time && current_time < end_time
        }
    }

    /// Resolve an icon path
    fn resolve_icon_path(&self, icon_path: &str) -> Result<PathBuf> {
        let path = Path::new(icon_path);

        if path.is_absolute() {
            return Ok(path.to_path_buf());
        }

        // Try to find the icon relative to the executable
        if let Ok(exec_path) = std::env::current_exe() {
            if let Some(exec_dir) = exec_path.parent() {
                let full_path = exec_dir.join(path);
                if full_path.exists() {
                    return Ok(full_path);
                }
            }
        }

        // Try to find the icon in the resources directory
        let resources_path = Path::new("resources").join("icons").join(path);
        if resources_path.exists() {
            return Ok(resources_path);
        }

        // If all else fails, return the original path
        Ok(path.to_path_buf())
    }

    /// Update the tray status
    pub fn update_tray_status(&self, status: &str) -> Result<()> {
        debug!("Updating tray status: {}", status);

        if service::is_running_as_service() {
            debug!("Running as a service, skipping tray status update");
            return Ok(());
        }

        if let Some(tray_manager) = &self.tray_manager {
            match tray_manager.lock() {
                Ok(mut tray) => {
                    if let Err(e) = tray.update_status(status) {
                        warn!("Failed to update tray status: {}", e);
                    } else {
                        debug!("Tray status updated successfully");
                    }
                },
                Err(e) => {
                    warn!("Failed to acquire lock on tray manager: {}", e);
                }
            }
        } else {
            debug!("No tray manager available for updating status");
        }

        Ok(())
    }

    /// Enable or disable the reboot option
    pub fn enable_reboot_option(&self, enable: bool) -> Result<()> {
        debug!("Setting reboot option enabled: {}", enable);

        if service::is_running_as_service() {
            debug!("Running as a service, skipping reboot option update");
            return Ok(());
        }

        if let Some(tray_manager) = &self.tray_manager {
            match tray_manager.lock() {
                Ok(mut tray) => {
                    if enable {
                        if let Err(e) = tray.enable_reboot_item() {
                            warn!("Failed to enable reboot option: {}", e);
                        } else {
                            debug!("Reboot option enabled successfully");
                        }
                    } else {
                        // TODO: Implement disable functionality when tray menu supports it
                        debug!("Disabling reboot option not implemented yet");
                    }
                },
                Err(e) => {
                    warn!("Failed to acquire lock on tray manager: {}", e);
                }
            }
        } else {
            debug!("No tray manager available for enabling/disabling reboot option");
        }

        Ok(())
    }

    /// Enable or disable the postpone option
    pub fn enable_postpone_option(&self, _enable: bool) -> Result<()> {
        debug!("Setting postpone option enabled");

        if service::is_running_as_service() {
            debug!("Running as a service, skipping postpone option update");
            return Ok(());
        }

        if let Some(tray_manager) = &self.tray_manager {
            match tray_manager.lock() {
                Ok(mut tray) => {
                    if let Err(e) = tray.enable_postpone_item() {
                        warn!("Failed to enable postpone option: {}", e);
                    } else {
                        debug!("Postpone option enabled successfully");
                    }
                },
                Err(e) => {
                    warn!("Failed to acquire lock on tray manager: {}", e);
                }
            }
        } else {
            debug!("No tray manager available for enabling/disabling postpone option");
        }

        Ok(())
    }

    /// Set the available deferral options
    pub fn set_deferral_options(&self, deferrals: &[String]) -> Result<()> {
        info!("Setting deferral options for notifications");

        if deferrals.is_empty() {
            info!("No deferral options provided");
            return Ok(());
        }

        info!("Available deferral options: {}", deferrals.join(", "));

        // Log each deferral option in detail
        for (i, deferral) in deferrals.iter().enumerate() {
            info!("Deferral option {}: {}", i + 1, deferral);

            // Parse the deferral to get the duration
            if let Ok(duration) = crate::utils::timespan::parse_timespan(deferral) {
                let total_seconds = duration.as_secs();
                let hours = total_seconds / 3600;
                let minutes = (total_seconds % 3600) / 60;
                info!("  Duration: {} hours and {} minutes", hours, minutes);
            } else {
                warn!("  Unable to parse deferral timespan: {}", deferral);
            }
        }

        if service::is_running_as_service() {
            debug!("Running as a service, skipping deferral options update");
            return Ok(());
        }

        if let Some(tray_manager) = &self.tray_manager {
            match tray_manager.lock() {
                Ok(_tray) => {
                    debug!("Tray manager found, but tray doesn't support setting deferral options directly");
                    // Tray doesn't support setting deferral options directly
                    // We'll need to implement this functionality
                },
                Err(e) => {
                    warn!("Failed to acquire lock on tray manager: {}", e);
                }
            }
        } else {
            debug!("No tray manager available for setting deferral options");
        }

        info!("Deferral options set successfully");
        Ok(())
    }


}
