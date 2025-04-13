use anyhow::Result;
use log::debug;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use windows::Win32::System::Environment::{ExpandEnvironmentStringsW, GetEnvironmentVariableW};
use windows::core::PCWSTR;

pub mod timespan;
pub mod registry;

/// Expand Windows environment variables in a string
///
/// This function expands environment variables in the format %VARIABLE% to their values.
/// For example, %USERPROFILE% might expand to C:\Users\Username
pub fn expand_env_vars(input: &str) -> Result<String> {
    debug!("Expanding environment variables in: {}", input);

    // Convert input to wide string (UTF-16)
    let input_wide: Vec<u16> = input.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        // First call to get required buffer size
        let result = ExpandEnvironmentStringsW(
            PCWSTR::from_raw(input_wide.as_ptr()),
            None,
        );

        if result == 0 {
            let error = windows::Win32::Foundation::GetLastError();
            return Err(anyhow::anyhow!("Failed to get buffer size for environment variable expansion: error code {}", error.0));
        }

        // Allocate buffer
        let mut buffer = vec![0u16; result as usize];

        // Second call to actually expand the variables
        let result = ExpandEnvironmentStringsW(
            PCWSTR::from_raw(input_wide.as_ptr()),
            Some(&mut buffer),
        );

        if result == 0 {
            let error = windows::Win32::Foundation::GetLastError();
            return Err(anyhow::anyhow!("Failed to expand environment variables: error code {}", error.0));
        }

        // Convert back to Rust string
        let expanded = match OsString::from_wide(&buffer[0..result as usize - 1]).into_string() {
            Ok(s) => s,
            Err(_) => return Err(anyhow::anyhow!("Failed to convert expanded environment variables to string")),
        };

        debug!("Expanded to: {}", expanded);
        Ok(expanded)
    }
}

/// Get the value of a Windows environment variable
pub fn get_env_var(name: &str) -> Result<String> {
    debug!("Getting environment variable: {}", name);

    // Convert name to wide string (UTF-16)
    let name_wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        // First call to get required buffer size
        let result = GetEnvironmentVariableW(
            PCWSTR::from_raw(name_wide.as_ptr()),
            None,
        );

        if result == 0 {
            let error = windows::Win32::Foundation::GetLastError();
            if error.0 == windows::Win32::Foundation::ERROR_ENVVAR_NOT_FOUND.0 {
                return Err(anyhow::anyhow!("Environment variable not found: {}", name));
            }
            return Err(anyhow::anyhow!("Failed to get buffer size for environment variable: error code {}", error.0));
        }

        // Allocate buffer
        let mut buffer = vec![0u16; result as usize];

        // Second call to actually get the variable
        let result = GetEnvironmentVariableW(
            PCWSTR::from_raw(name_wide.as_ptr()),
            Some(&mut buffer),
        );

        if result == 0 {
            let error = windows::Win32::Foundation::GetLastError();
            return Err(anyhow::anyhow!("Failed to get environment variable: error code {}", error.0));
        }

        // Convert back to Rust string
        let value = match OsString::from_wide(&buffer[0..result as usize]).into_string() {
            Ok(s) => s,
            Err(_) => return Err(anyhow::anyhow!("Failed to convert environment variable value to string")),
        };

        debug!("Value: {}", value);
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_env_vars() {
        // This test will only work on Windows
        let result = expand_env_vars("%WINDIR%\\System32").unwrap();
        assert!(result.contains("\\System32"));
        assert!(!result.contains("%WINDIR%"));

        // Test with multiple environment variables
        let result = expand_env_vars("%WINDIR%\\System32\\%USERNAME%").unwrap();
        assert!(result.contains("\\System32\\"));
        assert!(!result.contains("%WINDIR%"));
        assert!(!result.contains("%USERNAME%"));

        // Test with non-existent environment variable
        let result = expand_env_vars("%NON_EXISTENT_VAR%").unwrap();
        assert_eq!(result, "%NON_EXISTENT_VAR%"); // Windows keeps non-existent vars as is

        // Test with no environment variables
        let result = expand_env_vars("C:\\Program Files\\App").unwrap();
        assert_eq!(result, "C:\\Program Files\\App");
    }

    #[test]
    fn test_get_env_var() {
        // This test will only work on Windows
        let result = get_env_var("WINDIR").unwrap();
        assert!(!result.is_empty());

        // Test with non-existent environment variable
        let result = get_env_var("NON_EXISTENT_VAR");
        assert!(result.is_err());
    }
}
