#[cfg(test)]
mod windows_api_tests {
    use std::path::PathBuf;
    use std::process::Command;
    use tempfile::tempdir;

    // Test for session detection
    #[test]
    #[cfg(target_os = "windows")]
    fn test_session_detection() {
        // This test will only run on Windows
        // Create a new impersonator
        let impersonator = crate::impersonation::Impersonator::new();
        
        // Get interactive sessions
        let sessions = impersonator.get_interactive_sessions().expect("Failed to get interactive sessions");
        
        // We should at least have the current session
        assert!(!sessions.is_empty(), "No interactive sessions found");
        
        // Check that session properties are valid
        for session in &sessions {
            assert!(!session.user_name.is_empty(), "Session has empty username");
            assert!(!session.session_id.is_empty(), "Session has empty session ID");
            assert!(session.is_active, "Session is not active");
        }
    }
    
    // Test for SCCM client detection
    #[test]
    #[cfg(target_os = "windows")]
    fn test_sccm_client_detection() {
        // Create a new impersonator
        let impersonator = crate::impersonation::Impersonator::new();
        
        // Check if SCCM client is installed
        let result = impersonator.is_sccm_client_installed();
        
        // The test should not fail, regardless of whether SCCM is installed
        assert!(result.is_ok(), "SCCM client detection failed");
        
        // Log the result
        println!("SCCM client installed: {}", result.unwrap());
    }
    
    // Test for WMI last boot time
    #[test]
    #[cfg(target_os = "windows")]
    fn test_wmi_last_boot_time() {
        // Create a new detector
        let detector = crate::reboot::detector::RebootDetector::new(
            &crate::config::RebootConfig {
                timeframes: vec![],
                detection_methods: crate::config::DetectionMethodsConfig {
                    windows_update: true,
                    sccm: true,
                    registry: true,
                    pending_file_operations: true,
                },
            }
        );
        
        // Get last boot time
        let last_boot_time = detector.get_last_boot_time();
        
        // The test should not fail
        assert!(last_boot_time.is_ok(), "Failed to get last boot time");
        
        // The last boot time should be in the past
        let now = chrono::Utc::now();
        assert!(last_boot_time.unwrap() <= now, "Last boot time is in the future");
    }
    
    // Test for system info
    #[test]
    #[cfg(target_os = "windows")]
    fn test_system_info() {
        // Create a new detector
        let detector = crate::reboot::detector::RebootDetector::new(
            &crate::config::RebootConfig {
                timeframes: vec![],
                detection_methods: crate::config::DetectionMethodsConfig {
                    windows_update: true,
                    sccm: true,
                    registry: true,
                    pending_file_operations: true,
                },
            }
        );
        
        // Get system info
        let system_info = detector.get_system_info();
        
        // The test should not fail
        assert!(system_info.is_ok(), "Failed to get system info");
        
        // Check system info properties
        let info = system_info.unwrap();
        assert!(!info.computer_name.is_empty(), "Computer name is empty");
        assert!(!info.os_version.is_empty(), "OS version is empty");
        assert!(!info.domain.is_empty(), "Domain is empty");
        
        // The last boot time should be in the past
        let now = chrono::Utc::now();
        assert!(info.last_boot_time <= now, "Last boot time is in the future");
        
        // Uptime should be positive
        assert!(info.uptime > 0, "Uptime is not positive");
    }
}

// Add this to make the tests compile
#[cfg(test)]
mod reboot_reminder {
    pub use crate::*;
}
