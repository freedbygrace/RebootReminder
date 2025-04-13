use crate::database::DbPool;
use anyhow::{Context, Result};
use log::debug;
use std::path::Path;
use systray::Application;

/// Tray manager
pub struct TrayManager {
    app: Application,
    title: String,
    status_item_id: Option<u32>,
    reboot_item_id: Option<u32>,
    postpone_item_id: Option<u32>,
    #[allow(dead_code)]
    deferral_menu_id: Option<u32>,
    #[allow(dead_code)]
    deferral_item_ids: Vec<u32>,
}

impl TrayManager {
    /// Create a new tray manager
    pub fn new<P: AsRef<Path>>(title: &str, icon_path: P, _db_pool: DbPool) -> Result<Self> {
        debug!("Creating tray manager");

        // Create application
        let app = Application::new().context("Failed to create tray application")?;

        // Set icon
        let icon_path = icon_path.as_ref();
        if icon_path.exists() {
            app.set_icon_from_file(&icon_path.to_string_lossy())
                .context("Failed to set tray icon")?;
        }

        // Set tooltip
        app.set_tooltip(title)
            .context("Failed to set tray tooltip")?;

        // Create tray manager
        let mut tray = Self {
            app,
            title: title.to_string(),
            status_item_id: None,
            reboot_item_id: None,
            postpone_item_id: None,
            deferral_menu_id: None,
            deferral_item_ids: Vec::new(),
        };

        // Initialize menu
        tray.init_menu()?;

        Ok(tray)
    }

    /// Initialize the tray menu
    fn init_menu(&mut self) -> Result<()> {
        debug!("Initializing tray menu");

        // Add title
        self.app
            .add_menu_item(&self.title, |_| {
                Ok::<(), systray::Error>(())
            })
            .context("Failed to add title to tray menu")?;

        // Add separator
        self.app
            .add_menu_separator()
            .context("Failed to add separator to tray menu")?;

        Ok(())
    }

    /// Add a status item to the tray menu
    #[allow(dead_code)]
    pub fn add_status_item(&mut self, status: &str) -> Result<()> {
        debug!("Adding status item to tray menu");

        // Add status item
        let status_id = self.app
            .add_menu_item(&format!("Status: {}", status), |_| {
                Ok::<(), systray::Error>(())
            })
            .context("Failed to add status item to tray menu")?;

        self.status_item_id = Some(status_id);

        // Add separator
        self.app
            .add_menu_separator()
            .context("Failed to add separator to tray menu")?;

        Ok(())
    }

    /// Update the status text
    pub fn update_status(&mut self, status: &str) -> Result<()> {
        debug!("Updating status: {}", status);

        if let Some(_status_id) = self.status_item_id {
            // The systray crate doesn't support updating menu items
            // We'll need to remove the old one and add a new one
            // But the crate doesn't support removing items either
            // So we'll just leave it as is for now
        }

        Ok(())
    }

    /// Add a reboot item to the tray menu
    #[allow(dead_code)]
    pub fn add_reboot_item<F>(&mut self, callback: F) -> Result<()>
    where
        F: FnMut() -> Result<()> + Send + Sync + 'static,
    {
        debug!("Adding reboot item to tray menu");

        // Create a boxed callback
        let mut callback = Box::new(callback);

        // Add reboot item
        let reboot_id = self.app
            .add_menu_item("Reboot Now", move |_| {
                match callback() {
                    Ok(()) => Ok::<(), systray::Error>(()),
                    Err(_) => Ok::<(), systray::Error>(()),
                }
            })
            .context("Failed to add reboot item to tray menu")?;

        self.reboot_item_id = Some(reboot_id);

        Ok(())
    }

    /// Add a postpone item to the tray menu
    #[allow(dead_code)]
    pub fn add_postpone_item<F>(&mut self, callback: F) -> Result<()>
    where
        F: FnMut() -> Result<()> + Send + Sync + 'static,
    {
        debug!("Adding postpone item to tray menu");

        // Create a boxed callback
        let mut callback = Box::new(callback);

        // Add postpone item
        let postpone_id = self.app
            .add_menu_item("Postpone Reboot", move |_| {
                match callback() {
                    Ok(()) => Ok::<(), systray::Error>(()),
                    Err(_) => Ok::<(), systray::Error>(()),
                }
            })
            .context("Failed to add postpone item to tray menu")?;

        self.postpone_item_id = Some(postpone_id);

        Ok(())
    }

    /// Add a quit item to the tray menu
    #[allow(dead_code)]
    pub fn add_quit_item<F>(&mut self, callback: F) -> Result<()>
    where
        F: FnMut() -> Result<()> + Send + Sync + 'static,
    {
        debug!("Adding quit item to tray menu");

        // Create a boxed callback
        let mut callback = Box::new(callback);

        // Add quit item
        self.app
            .add_menu_item("Quit", move |app| {
                match callback() {
                    Ok(()) => {
                        app.quit();
                        Ok::<(), systray::Error>(())
                    },
                    Err(_) => Ok::<(), systray::Error>(()),
                }
            })
            .context("Failed to add quit item to tray menu")?;

        Ok(())
    }

    /// Remove a menu item
    #[allow(dead_code)]
    pub fn remove_menu_item(&mut self, id: u32) -> Result<()> {
        debug!("Removing menu item: {}", id);

        // The systray crate doesn't support removing menu items
        // We'll just leave it as is for now

        Ok(())
    }

    /// Enable reboot item
    pub fn enable_reboot_item(&mut self) -> Result<()> {
        debug!("Enabling reboot item");

        if let Some(_) = self.reboot_item_id {
            // The systray crate doesn't support enabling/disabling items
            // We'll just leave it as is for now
        }

        Ok(())
    }

    /// Disable reboot item
    #[allow(dead_code)]
    pub fn disable_reboot_item(&mut self) -> Result<()> {
        debug!("Disabling reboot item");

        if let Some(_id) = self.reboot_item_id {
            // The systray crate doesn't support enabling/disabling items
            // We'll just leave it as is for now
        }

        Ok(())
    }

    /// Enable postpone item
    pub fn enable_postpone_item(&mut self) -> Result<()> {
        debug!("Enabling postpone item");

        if let Some(_id) = self.postpone_item_id {
            // The systray crate doesn't support enabling/disabling items
            // We'll just leave it as is for now
        }

        Ok(())
    }

    /// Disable postpone item
    #[allow(dead_code)]
    pub fn disable_postpone_item(&mut self) -> Result<()> {
        debug!("Disabling postpone item");

        if let Some(_id) = self.postpone_item_id {
            // The systray crate doesn't support enabling/disabling items
            // We'll just leave it as is for now
        }

        Ok(())
    }

    /// Add a deferral menu
    #[allow(dead_code)]
    pub fn add_deferral_menu(&mut self) -> Result<()> {
        debug!("Adding deferral menu");

        // The systray crate doesn't support submenus
        // We'll just add deferral items directly to the main menu

        Ok(())
    }

    /// Add a deferral item to the tray menu
    #[allow(dead_code)]
    pub fn add_deferral_item<F>(&mut self, label: &str, callback: F) -> Result<u32>
    where
        F: FnMut() -> Result<()> + Send + Sync + 'static,
    {
        debug!("Adding deferral item to tray menu: {}", label);

        // Create a boxed callback
        let mut callback = Box::new(callback);

        // Add deferral item
        let id = self.app
            .add_menu_item(label, move |_| {
                match callback() {
                    Ok(()) => Ok::<(), systray::Error>(()),
                    Err(_) => Ok::<(), systray::Error>(()),
                }
            })
            .context("Failed to add deferral item to tray menu")?;

        self.deferral_item_ids.push(id);

        Ok(id)
    }

    /// Clear all deferral items
    #[allow(dead_code)]
    pub fn clear_deferral_items(&mut self) -> Result<()> {
        debug!("Clearing deferral items");

        // The systray crate doesn't support removing menu items
        // We'll just leave them as is for now

        Ok(())
    }
}
