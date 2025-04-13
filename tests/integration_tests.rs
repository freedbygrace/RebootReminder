#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::process::Command;
    use tempfile::tempdir;

    #[test]
    fn test_config_loading() {
        // Create a temporary directory
        let temp_dir = tempdir().expect("Failed to create temporary directory");
        let config_path = temp_dir.path().join("config.json");

        // Create a test configuration file
        let config_content = r#"{
            "service": {
                "name": "TestService",
                "displayName": "Test Service",
                "description": "Test service description",
                "configRefreshMinutes": 60
            },
            "notification": {
                "type": "both",
                "branding": {
                    "title": "Test Title",
                    "iconPath": "test.ico",
                    "company": "Test Company"
                },
                "messages": {
                    "rebootRequired": "Test reboot required message",
                    "rebootRecommended": "Test reboot recommended message",
                    "rebootScheduled": "Test reboot scheduled message",
                    "rebootInProgress": "Test reboot in progress message",
                    "rebootCancelled": "Test reboot cancelled message",
                    "rebootPostponed": "Test reboot postponed message",
                    "rebootCompleted": "Test reboot completed message",
                    "actionRequired": "Test action required message",
                    "actionRecommended": "Test action recommended message",
                    "actionNotRequired": "Test action not required message",
                    "actionNotAvailable": "Test action not available message"
                },
                "quietHours": {
                    "enabled": true,
                    "startTime": "22:00",
                    "endTime": "08:00",
                    "daysOfWeek": [0, 1, 2, 3, 4, 5, 6]
                }
            },
            "reboot": {
                "timeframes": [
                    {
                        "minHours": 24,
                        "maxHours": 48,
                        "reminderIntervalHours": 4,
                        "deferrals": ["1h", "4h", "8h", "24h"]
                    }
                ],
                "detectionMethods": {
                    "windowsUpdate": true,
                    "sccm": true,
                    "registry": true,
                    "pendingFileOperations": true
                }
            },
            "database": {
                "path": "test.db"
            },
            "logging": {
                "path": "test.log",
                "level": "debug",
                "maxFiles": 7,
                "maxSize": 10
            }
        }"#;

        std::fs::write(&config_path, config_content).expect("Failed to write test configuration file");

        // Run the check command with the test configuration
        let output = Command::new(get_executable_path())
            .arg("--config")
            .arg(config_path)
            .arg("check")
            .output()
            .expect("Failed to execute command");

        // Check that the command executed successfully
        assert!(output.status.success(), "Command failed: {:?}", output);

        // Check that the output contains expected text
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Configuration loaded"), "Unexpected output: {}", stdout);
    }

    #[test]
    fn test_help_command() {
        // Run the help command
        let output = Command::new(get_executable_path())
            .arg("--help")
            .output()
            .expect("Failed to execute command");

        // Check that the command executed successfully
        assert!(output.status.success(), "Command failed: {:?}", output);

        // Check that the output contains expected text
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Reboot Reminder"), "Unexpected output: {}", stdout);
        assert!(stdout.contains("USAGE:"), "Unexpected output: {}", stdout);
        assert!(stdout.contains("OPTIONS:"), "Unexpected output: {}", stdout);
        assert!(stdout.contains("SUBCOMMANDS:"), "Unexpected output: {}", stdout);
    }

    // Helper function to get the path to the executable
    fn get_executable_path() -> PathBuf {
        let mut path = std::env::current_exe().expect("Failed to get current executable path");
        path.pop(); // Remove the test executable name
        path.pop(); // Remove "deps" directory
        path.push("reboot_reminder");
        #[cfg(windows)]
        path.set_extension("exe");
        path
    }
}
