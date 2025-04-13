pub mod config;
pub mod database;
pub mod impersonation;
pub mod logging;
pub mod notification;
pub mod reboot;
pub mod service;
pub mod utils;
pub mod watchdog;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use log::{error, info};
use std::path::PathBuf;

/// Reboot Reminder - A cross-platform reboot reminder system
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to configuration file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,

    /// Subcommands
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Install the service
    Install {
        /// Service name
        #[arg(short, long, default_value = "RebootReminder")]
        name: String,

        /// Service display name
        #[arg(short, long, default_value = "Reboot Reminder Service")]
        display_name: String,

        /// Service description
        #[arg(short, long, default_value = "Provides notifications when system reboots are necessary")]
        description: String,
    },
    /// Uninstall the service
    Uninstall,
    /// Run the service
    Run,
    /// Check if the system requires a reboot
    Check,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    logging::init(args.debug).context("Failed to initialize logging")?;
    info!("Starting Reboot Reminder");

    // Check if running with administrative privileges for commands that require it
    let admin_required = matches!(&args.command,
        Some(Commands::Install {..}) | Some(Commands::Uninstall) | Some(Commands::Run)
    );

    if admin_required && !is_running_as_admin() {
        error!("This command requires administrative privileges. Please run as administrator.");
        return Err(anyhow::anyhow!("Administrative privileges required"));
    }

    // Load configuration
    let config_path = args.config.unwrap_or_else(|| {
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

    // Set the config path for the service
    if let Some(Commands::Run) = &args.command {
        unsafe {
            service::set_config_path(config_path.clone());
        }
    }

    let config = config::load(&config_path).context("Failed to load configuration")?;
    info!("Configuration loaded from {:?}", config_path);

    // Initialize database
    let db = database::init(&config.database).context("Failed to initialize database")?;
    info!("Database initialized");

    // Process command
    match args.command {
        Some(Commands::Install {
            name,
            display_name,
            description,
        }) => {
            info!("Installing service: {}", name);
            service::install(&name, &display_name, &description)
                .context("Failed to install service")?;
            info!("Service installed successfully");
        }
        Some(Commands::Uninstall) => {
            info!("Uninstalling service");
            service::uninstall().context("Failed to uninstall service")?;
            info!("Service uninstalled successfully");
        }
        Some(Commands::Run) => {
            info!("Running service");
            service::run(config, db).context("Failed to run service")?;
        }
        Some(Commands::Check) => {
            info!("Checking if the system requires a reboot");
            let detector = reboot::detector::RebootDetector::new(&config.reboot);
            match detector.check_reboot_required() {
                Ok((required, sources)) => {
                    if required {
                        info!("Reboot is required. Sources: {:?}", sources);
                    } else {
                        info!("No reboot is required");
                    }
                }
                Err(e) => {
                    error!("Failed to check if reboot is required: {}", e);
                }
            }
        }
        None => {
            // Default to running the service
            info!("No command specified, running service");
            service::run(config, db).context("Failed to run service")?;
        }
    }

    info!("Reboot Reminder exiting");
    Ok(())
}

/// Check if the application is running with administrative privileges
fn is_running_as_admin() -> bool {
    use windows::Win32::UI::Shell::IsUserAnAdmin;

    unsafe {
        // Call the Windows API to check if the current user is an administrator
        IsUserAnAdmin() == true
    }
}
