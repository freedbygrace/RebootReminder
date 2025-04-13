pub mod config;
pub mod database;
pub mod impersonation;
pub mod logging;
pub mod notification;
pub mod reboot;
pub mod service;
pub mod utils;
pub mod watchdog;

use anyhow::Result;
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
    if let Err(e) = logging::init(args.debug) {
        // Can't use log macros yet since logging isn't initialized
        eprintln!("Failed to initialize logging: {}", e);
        return Err(anyhow::anyhow!("Failed to initialize logging: {}", e));
    }
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

    info!("Using configuration file: {:?}", config_path);

    // Set the config path for the service
    if let Some(Commands::Run) = &args.command {
        unsafe {
            service::set_config_path(config_path.clone());
        }
    }

    let config = match config::load(&config_path) {
        Ok(cfg) => {
            info!("Configuration loaded successfully from {:?}", config_path);
            cfg
        },
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            return Err(anyhow::anyhow!("Failed to load configuration: {}", e));
        }
    };

    // Initialize database
    let db = match database::init(&config.database) {
        Ok(pool) => {
            info!("Database initialized successfully at {}", config.database.path);
            pool
        },
        Err(e) => {
            error!("Failed to initialize database: {}", e);
            return Err(anyhow::anyhow!("Failed to initialize database: {}", e));
        }
    };

    // Process command
    match args.command {
        Some(Commands::Install {
            name,
            display_name,
            description,
        }) => {
            info!("Installing service: {}", name);
            match service::install(&name, &display_name, &description) {
                Ok(_) => info!("Service installed successfully"),
                Err(e) => {
                    error!("Failed to install service: {}", e);
                    return Err(anyhow::anyhow!("Failed to install service: {}", e));
                }
            }
        }
        Some(Commands::Uninstall) => {
            info!("Uninstalling service");
            match service::uninstall() {
                Ok(_) => info!("Service uninstalled successfully"),
                Err(e) => {
                    error!("Failed to uninstall service: {}", e);
                    return Err(anyhow::anyhow!("Failed to uninstall service: {}", e));
                }
            }
        }
        Some(Commands::Run) => {
            info!("Running service");
            match service::run(config, db) {
                Ok(_) => info!("Service completed successfully"),
                Err(e) => {
                    error!("Failed to run service: {}", e);
                    return Err(anyhow::anyhow!("Failed to run service: {}", e));
                }
            }
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
                    return Err(anyhow::anyhow!("Failed to check if reboot is required: {}", e));
                }
            }
        }
        None => {
            // Default to running the service
            info!("No command specified, running service");
            match service::run(config, db) {
                Ok(_) => info!("Service completed successfully"),
                Err(e) => {
                    error!("Failed to run service: {}", e);
                    return Err(anyhow::anyhow!("Failed to run service: {}", e));
                }
            }
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
