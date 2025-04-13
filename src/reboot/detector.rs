use crate::config::RebootConfig;
use crate::database::RebootSource;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use log::{debug, info, warn};

use windows::Win32::System::Registry::HKEY_LOCAL_MACHINE;
// use std::time::SystemTime;
// use uuid::Uuid;
use wmi::{self, COMLibrary};
use serde_derive::Deserialize;

/// Reboot detector
pub struct RebootDetector {
    config: RebootConfig,
}

impl RebootDetector {
    /// Create a new reboot detector
    pub fn new(config: &RebootConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Check if a reboot is required
    pub fn check_reboot_required(&self) -> Result<(bool, Vec<RebootSource>)> {
        info!("Checking if reboot is required");
        let mut sources = Vec::new();
        let mut is_required = false;

        // Check Windows Update
        if self.config.detection_methods.windows_update {
            info!("Checking Windows Update to determine if a reboot is required");
            match self.check_windows_update() {
                Ok((required, source)) => {
                    if required {
                        info!("Windows Update requires a reboot: {}", source.description.as_deref().unwrap_or("No details"));
                        is_required = true;
                        sources.push(source);
                    } else {
                        info!("Windows Update does not require a reboot");
                    }
                }
                Err(e) => {
                    warn!("Failed to check Windows Update: {}", e);
                }
            }
        } else {
            debug!("Windows Update check is disabled");
        }

        // Check SCCM
        if self.config.detection_methods.sccm {
            info!("Checking SCCM to determine if a reboot is required");
            match self.check_sccm() {
                Ok((required, source)) => {
                    if required {
                        info!("SCCM requires a reboot: {}", source.description.as_deref().unwrap_or("No details"));
                        is_required = true;
                        sources.push(source);
                    } else {
                        info!("SCCM does not require a reboot");
                    }
                }
                Err(e) => {
                    warn!("Failed to check SCCM: {}", e);
                }
            }
        } else {
            debug!("SCCM check is disabled");
        }

        // Check registry
        if self.config.detection_methods.registry {
            info!("Checking registry to determine if a reboot is required");
            match self.check_registry() {
                Ok((required, source)) => {
                    if required {
                        info!("Registry indicates a reboot is required: {}", source.description.as_deref().unwrap_or("No details"));
                        is_required = true;
                        sources.push(source);
                    } else {
                        info!("Registry does not indicate a reboot is required");
                    }
                }
                Err(e) => {
                    warn!("Failed to check registry: {}", e);
                }
            }
        } else {
            debug!("Registry check is disabled");
        }

        // Check pending file operations
        if self.config.detection_methods.pending_file_operations {
            info!("Checking for pending file operations that require a reboot");
            match self.check_pending_file_operations() {
                Ok((required, source)) => {
                    if required {
                        info!("Pending file operations require a reboot: {}", source.description.as_deref().unwrap_or("No details"));
                        is_required = true;
                        sources.push(source);
                    } else {
                        info!("No pending file operations requiring a reboot");
                    }
                }
                Err(e) => {
                    warn!("Failed to check pending file operations: {}", e);
                }
            }
        } else {
            debug!("Pending file operations check is disabled");
        }

        debug!("Reboot required: {}, sources: {:?}", is_required, sources);
        // Log the final result
        if is_required {
            info!("Reboot is required. Found {} sources requiring reboot.", sources.len());
            for (i, source) in sources.iter().enumerate() {
                info!("  Source {}: {} - {} (detected at {})",
                      i + 1,
                      source.name,
                      source.description.as_deref().unwrap_or("No details"),
                      source.detected_at);
            }
        } else {
            info!("No reboot is required");
        }

        Ok((is_required, sources))
    }

    /// Check Windows Update to determine if a reboot is required
    fn check_windows_update(&self) -> Result<(bool, RebootSource)> {
        debug!("Checking Windows Update to determine if a reboot is required");

        // Create a source object
        let mut source = RebootSource::new(
            "windows_update",
            Some("Windows Update requires a reboot"),
            "required",
        );

        // Check the registry key that indicates Windows Update requires a reboot
        let required = crate::utils::registry::key_exists(
            HKEY_LOCAL_MACHINE,
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\WindowsUpdate\\Auto Update\\RebootRequired"
        )?;

        if required {
            source.details = Some("Windows Update registry key indicates a reboot is required".to_string());
            debug!("Windows Update requires a reboot");
        } else {
            debug!("Windows Update does not require a reboot");
        }

        Ok((required, source))
    }

    /// Check SCCM to determine if a reboot is required
    fn check_sccm(&self) -> Result<(bool, RebootSource)> {
        debug!("Checking SCCM to determine if a reboot is required");

        // Create a source object
        let mut source = RebootSource::new(
            "sccm",
            Some("SCCM requires a reboot"),
            "required",
        );

        // Check if SCCM client service is installed
        let impersonator = crate::impersonation::Impersonator::new();
        let sccm_installed = match impersonator.is_sccm_client_installed() {
            Ok(installed) => installed,
            Err(e) => {
                warn!("Failed to check if SCCM client is installed: {}", e);
                false
            }
        };

        if !sccm_installed {
            debug!("SCCM client not installed");
            return Ok((false, source));
        }

        // Check registry for SCCM reboot flags
        // This is more efficient than using WMI and PowerShell
        let registry_paths = [
            // Main SCCM reboot flag
            "SOFTWARE\\Microsoft\\CCM\\ClientSDK\\RebootPending",
            // Additional SCCM reboot flags
            "SOFTWARE\\Microsoft\\SMS\\Mobile Client\\Reboot Management\\RebootData",
        ];

        for path in &registry_paths {
            if crate::utils::registry::key_exists(HKEY_LOCAL_MACHINE, path)? {
                source.details = Some(format!("SCCM registry key indicates a reboot is pending: {}", path));
                debug!("SCCM requires a reboot (registry key: {})", path);
                return Ok((true, source));
            }
        }

        // Check for SCCM reboot files
        let ccm_reboot_files = [
            "C:\\Windows\\CCM\\ClientCache\\SCNotify.exe.reboot",
            "C:\\Windows\\CCM\\CIStateStore\\Reboot",
        ];

        for file_path in &ccm_reboot_files {
            if std::path::Path::new(file_path).exists() {
                source.details = Some(format!("SCCM reboot file exists: {}", file_path));
                debug!("SCCM requires a reboot (file exists: {})", file_path);
                return Ok((true, source));
            }
        }

        debug!("SCCM does not require a reboot");
        Ok((false, source))
    }

    /// Check registry to determine if a reboot is required
    fn check_registry(&self) -> Result<(bool, RebootSource)> {
        debug!("Checking registry to determine if a reboot is required");

        // Create a source object
        let mut source = RebootSource::new(
            "registry",
            Some("Registry indicates a reboot is required"),
            "required",
        );

        // Check Component Based Servicing
        if crate::utils::registry::key_exists(
            HKEY_LOCAL_MACHINE,
            "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Component Based Servicing\\RebootPending"
        )? {
            source.details = Some("Component Based Servicing registry key indicates a reboot is pending".to_string());
            debug!("Component Based Servicing requires a reboot");
            return Ok((true, source));
        }

        // Check Session Manager
        if let Some(pending_renames) = crate::utils::registry::get_string_value(
            HKEY_LOCAL_MACHINE,
            "SYSTEM\\CurrentControlSet\\Control\\Session Manager",
            "PendingFileRenameOperations"
        )? {
            if !pending_renames.is_empty() {
                source.details = Some("Session Manager registry key indicates pending file rename operations".to_string());
                debug!("Session Manager requires a reboot");
                return Ok((true, source));
            }
        }

        // Check for pending computer rename
        let active_name = crate::utils::registry::get_string_value(
            HKEY_LOCAL_MACHINE,
            "SYSTEM\\CurrentControlSet\\Control\\ComputerName\\ActiveComputerName",
            "ComputerName"
        )?;

        let pending_name = crate::utils::registry::get_string_value(
            HKEY_LOCAL_MACHINE,
            "SYSTEM\\CurrentControlSet\\Control\\ComputerName\\ComputerName",
            "ComputerName"
        )?;

        if let (Some(active), Some(pending)) = (active_name, pending_name) {
            if !crate::utils::registry::compare_computer_names(&active, &pending) {
                source.details = Some("Computer name change is pending".to_string());
                debug!("Computer name change requires a reboot");
                return Ok((true, source));
            }
        }

        debug!("Registry does not indicate a reboot is required");
        Ok((false, source))
    }

    /// Check for pending file operations that require a reboot
    fn check_pending_file_operations(&self) -> Result<(bool, RebootSource)> {
        debug!("Checking for pending file operations that require a reboot");

        // Create a source object
        let mut source = RebootSource::new(
            "pending_file_operations",
            Some("Pending file operations require a reboot"),
            "required",
        );

        // Check for pending file rename operations in the registry
        if let Some(pending_renames) = crate::utils::registry::get_string_value(
            HKEY_LOCAL_MACHINE,
            "SYSTEM\\CurrentControlSet\\Control\\Session Manager",
            "PendingFileRenameOperations"
        )? {
            if !pending_renames.is_empty() {
                source.details = Some("Pending file rename operations detected".to_string());
                debug!("Pending file rename operations require a reboot");
                return Ok((true, source));
            }
        }

        // Check for Windows.~BT or Windows.~WS directories
        let win_dir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());
        let bt_dir = std::path::Path::new(&win_dir).join("Windows.~BT");
        let ws_dir = std::path::Path::new(&win_dir).join("Windows.~WS");

        if bt_dir.exists() {
            source.details = Some("Windows.~BT directory exists, indicating a pending Windows upgrade".to_string());
            debug!("Windows.~BT directory exists, indicating a pending Windows upgrade");
            return Ok((true, source));
        }

        if ws_dir.exists() {
            source.details = Some("Windows.~WS directory exists, indicating a pending Windows upgrade".to_string());
            debug!("Windows.~WS directory exists, indicating a pending Windows upgrade");
            return Ok((true, source));
        }

        debug!("No pending file operations detected");
        Ok((false, source))
    }

    /// Get the last boot time using WMI
    pub fn get_last_boot_time(&self) -> Result<DateTime<Utc>> {
        debug!("Getting last boot time using WMI");

        // Use WMI to get the last boot time
        let wmi_con = wmi::WMIConnection::new(wmi::COMLibrary::new()?.into())
            .context("Failed to connect to WMI")?;

        // Define a struct to hold the WMI query results
        #[derive(Debug, Deserialize)]
        struct OSInfo {
            #[serde(rename = "LastBootUpTime")]
            last_boot_up_time: String,
        }

        // Query WMI for the last boot time
        let results: Vec<OSInfo> = wmi_con.query()
            .context("Failed to query WMI for last boot time")?;

        if results.is_empty() {
            return Err(anyhow::anyhow!("No OS information found in WMI"));
        }

        let last_boot_time = &results[0].last_boot_up_time;

        // WMI returns the time in a format like: 20230101000000.000000+000
        // We need to parse this into a DateTime<Utc>

        // Extract the date and time parts
        if last_boot_time.len() < 14 {
            return Err(anyhow::anyhow!("Invalid last boot time format: {}", last_boot_time));
        }

        let year = &last_boot_time[0..4];
        let month = &last_boot_time[4..6];
        let day = &last_boot_time[6..8];
        let hour = &last_boot_time[8..10];
        let minute = &last_boot_time[10..12];
        let second = &last_boot_time[12..14];

        // Parse into DateTime<Utc>
        let datetime_str = format!("{}-{}-{}T{}:{}:{}Z", year, month, day, hour, minute, second);
        let datetime = DateTime::parse_from_rfc3339(&datetime_str)
            .context("Failed to parse last boot time")?
            .with_timezone(&Utc);

        debug!("Last boot time: {}", datetime);
        Ok(datetime)
    }

    /// Get system information using WMI with optimized queries
    pub fn get_system_info(&self) -> Result<SystemInfo> {
        debug!("Getting system information using WMI");

        // Create a single WMI connection to reuse
        let com_lib = wmi::COMLibrary::new().context("Failed to initialize COM library")?;
        let wmi_con = wmi::WMIConnection::new(com_lib.into())
            .context("Failed to connect to WMI")?;

        // Define a combined struct to hold the WMI query results
        // This allows us to get multiple properties in a single query
        #[derive(Debug, Deserialize)]
        struct SystemInfoWMI {
            // OS properties
            #[serde(rename = "Caption")]
            caption: Option<String>,
            #[serde(rename = "CSName")]
            computer_name: Option<String>,
            #[serde(rename = "LastBootUpTime")]
            last_boot_up_time: Option<String>,

            // ComputerSystem properties
            #[serde(rename = "Domain")]
            domain: Option<String>,
            #[serde(rename = "Model")]
            model: Option<String>,
        }

        // Define network adapter info struct
        #[derive(Debug, Deserialize)]
        struct NetworkAdapterInfo {
            #[serde(rename = "IPAddress")]
            ip_address: Option<Vec<String>>,
        }

        // Query WMI for OS and ComputerSystem information in a single query
        // This is more efficient than multiple queries
        let query = "SELECT OS.Caption, OS.CSName, OS.LastBootUpTime, CS.Domain, CS.Model \
                    FROM Win32_OperatingSystem AS OS, Win32_ComputerSystem AS CS";

        let results: Vec<SystemInfoWMI> = wmi_con.raw_query(query)
            .context("Failed to query WMI for system information")?;

        if results.is_empty() {
            return Err(anyhow::anyhow!("No system information found in WMI"));
        }

        let system_info = &results[0];

        // Extract values with defaults for missing data
        let computer_name = system_info.computer_name.clone().unwrap_or_else(|| "Unknown".to_string());
        let os_version = system_info.caption.clone().unwrap_or_else(|| "Unknown".to_string());
        let domain = system_info.domain.clone().unwrap_or_else(|| "Unknown".to_string());
        let model = system_info.model.clone().unwrap_or_default().to_lowercase();

        // Parse last boot time if available
        let last_boot_time = if let Some(boot_time_str) = &system_info.last_boot_up_time {
            if boot_time_str.len() >= 14 {
                let year = &boot_time_str[0..4];
                let month = &boot_time_str[4..6];
                let day = &boot_time_str[6..8];
                let hour = &boot_time_str[8..10];
                let minute = &boot_time_str[10..12];
                let second = &boot_time_str[12..14];

                let datetime_str = format!("{}-{}-{}T{}:{}:{}Z", year, month, day, hour, minute, second);
                match DateTime::parse_from_rfc3339(&datetime_str) {
                    Ok(dt) => dt.with_timezone(&Utc),
                    Err(e) => {
                        warn!("Failed to parse last boot time from WMI: {}", e);
                        // Fall back to the dedicated method
                        self.get_last_boot_time()?
                    }
                }
            } else {
                // Fall back to the dedicated method
                self.get_last_boot_time()?
            }
        } else {
            // Fall back to the dedicated method
            self.get_last_boot_time()?
        };

        // Query WMI for network adapter information with a more efficient query
        let query = "SELECT IPAddress FROM Win32_NetworkAdapterConfiguration WHERE IPEnabled = True";
        let na_results: Vec<NetworkAdapterInfo> = wmi_con.raw_query(query)
            .context("Failed to query WMI for network adapter information")?;

        let ip_address = na_results.iter()
            .filter_map(|na| na.ip_address.as_ref())
            .flat_map(|ips| ips.iter())
            .find(|ip| ip.contains('.')) // Filter for IPv4 addresses
            .map(|ip| ip.to_string());

        // Calculate uptime
        let now = Utc::now();
        let uptime = now.signed_duration_since(last_boot_time).num_seconds();

        let is_virtual_machine = model.contains("virtual") || model.contains("vmware") || model.contains("hyper-v");

        // Check if SCCM client is installed using the service check
        let impersonator = crate::impersonation::Impersonator::new();
        let sccm_client_installed = match impersonator.is_sccm_client_installed() {
            Ok(installed) => installed,
            Err(e) => {
                warn!("Failed to check if SCCM client is installed: {}", e);
                false
            }
        };

        // Get SCCM client version using WMI
        let sccm_client_version = if sccm_client_installed {
            // Use WMI to get the SCCM client version
            match wmi::WMIConnection::new(wmi::COMLibrary::new()?.into()) {
                Ok(_wmi_con) => {
                    // Define a struct to hold the WMI query results
                    #[derive(Debug, Deserialize)]
                    struct CCMClientVersion {
                        #[serde(rename = "ClientVersion")]
                        client_version: Option<String>,
                    }

                    // Query WMI for the client version
                    let query = "SELECT ClientVersion FROM CCM_InstalledComponent WHERE Name='SMS Client'";
                    let wmi_con = wmi::WMIConnection::with_namespace_path("root\\ccm", COMLibrary::new()?.into())?;

                    match wmi_con.raw_query::<CCMClientVersion>(query) {
                        Ok(results) => {
                            if let Some(result) = results.first() {
                                result.client_version.clone()
                            } else {
                                None
                            }
                        },
                        Err(e) => {
                            warn!("Failed to query WMI for SCCM client version: {}", e);
                            None
                        }
                    }
                },
                Err(e) => {
                    warn!("Failed to connect to WMI: {}", e);
                    None
                }
            }
        } else {
            None
        };

        let info = SystemInfo {
            computer_name,
            os_version,
            last_boot_time,
            uptime,
            ip_address,
            domain,
            is_virtual_machine,
            sccm_client_installed,
            sccm_client_version,
        };

        debug!("System information: {:?}", info);
        Ok(info)
    }
}

/// System information
#[derive(Debug, Clone)]
pub struct SystemInfo {
    /// Computer name
    pub computer_name: String,

    /// OS version
    pub os_version: String,

    /// Last boot time
    pub last_boot_time: DateTime<Utc>,

    /// Uptime in seconds
    pub uptime: i64,

    /// IP address
    pub ip_address: Option<String>,

    /// Domain
    pub domain: String,

    /// Whether the system is a virtual machine
    pub is_virtual_machine: bool,

    /// Whether the SCCM client is installed
    pub sccm_client_installed: bool,

    /// SCCM client version
    pub sccm_client_version: Option<String>,
}
