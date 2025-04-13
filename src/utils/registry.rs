use anyhow::Result;
use log::{debug, warn};
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use windows::core::PCWSTR;
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{
    HKEY, HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_READ, REG_DWORD, REG_MULTI_SZ, REG_SZ,
    RegCloseKey, RegOpenKeyExW, RegQueryValueExW,
};

/// Check if a registry key exists
pub fn key_exists(hive: HKEY, key_path: &str) -> Result<bool> {
    debug!("Checking if registry key exists: {}\\{}", hive_to_string(hive), key_path);

    let key_path_wide: Vec<u16> = key_path.encode_utf16().chain(std::iter::once(0)).collect();
    let mut h_key = HKEY::default();

    unsafe {
        let result = RegOpenKeyExW(
            hive,
            PCWSTR::from_raw(key_path_wide.as_ptr()),
            Some(0),
            KEY_READ,
            &mut h_key,
        );

        // Always close the key if it was opened
        if h_key != HKEY::default() {
            let _ = RegCloseKey(h_key);
        }

        Ok(result == ERROR_SUCCESS)
    }
}

/// Check if a registry value exists
pub fn value_exists(hive: HKEY, key_path: &str, value_name: &str) -> Result<bool> {
    debug!("Checking if registry value exists: {}\\{}\\{}", hive_to_string(hive), key_path, value_name);

    let key_path_wide: Vec<u16> = key_path.encode_utf16().chain(std::iter::once(0)).collect();
    let value_name_wide: Vec<u16> = value_name.encode_utf16().chain(std::iter::once(0)).collect();
    let mut h_key = HKEY::default();

    unsafe {
        // Open the key
        let result = RegOpenKeyExW(
            hive,
            PCWSTR::from_raw(key_path_wide.as_ptr()),
            Some(0),
            KEY_QUERY_VALUE,
            &mut h_key,
        );

        if result != ERROR_SUCCESS {
            // Key doesn't exist
            debug!("Registry key does not exist: {}\\{}", hive_to_string(hive), key_path);
            return Ok(false);
        }

        // Query the value
        let mut data_type = 0u32;
        let mut data_size = 0u32;

        let query_result = RegQueryValueExW(
            h_key,
            PCWSTR::from_raw(value_name_wide.as_ptr()),
            None,
            Some(&mut data_type as *mut u32 as *mut _),
            None,
            Some(&mut data_size),
        );

        // Always close the key
        let _ = RegCloseKey(h_key);

        Ok(query_result == ERROR_SUCCESS)
    }
}

/// Get a string value from the registry
pub fn get_string_value(hive: HKEY, key_path: &str, value_name: &str) -> Result<Option<String>> {
    debug!("Getting string value from registry: {}\\{}\\{}", hive_to_string(hive), key_path, value_name);

    let key_path_wide: Vec<u16> = key_path.encode_utf16().chain(std::iter::once(0)).collect();
    let value_name_wide: Vec<u16> = value_name.encode_utf16().chain(std::iter::once(0)).collect();
    let mut h_key = HKEY::default();

    unsafe {
        // Open the key
        let result = RegOpenKeyExW(
            hive,
            PCWSTR::from_raw(key_path_wide.as_ptr()),
            Some(0),
            KEY_QUERY_VALUE,
            &mut h_key,
        );

        if result != ERROR_SUCCESS {
            // Key doesn't exist
            debug!("Registry key does not exist: {}\\{}", hive_to_string(hive), key_path);
            return Ok(None);
        }

        // Query the value size
        let mut data_type = 0u32;
        let mut data_size = 0u32;

        let query_result = RegQueryValueExW(
            h_key,
            PCWSTR::from_raw(value_name_wide.as_ptr()),
            None,
            Some(&mut data_type as *mut u32 as *mut _),
            None,
            Some(&mut data_size),
        );

        if query_result != ERROR_SUCCESS || (data_type != REG_SZ.0 && data_type != REG_MULTI_SZ.0) {
            // Value doesn't exist or is not a string
            let _ = RegCloseKey(h_key);
            debug!("Registry value does not exist or is not a string: {}\\{}\\{}", hive_to_string(hive), key_path, value_name);
            return Ok(None);
        }

        // Allocate buffer for the value
        let mut buffer = vec![0u16; (data_size / 2) as usize];

        // Get the value
        let query_result = RegQueryValueExW(
            h_key,
            PCWSTR::from_raw(value_name_wide.as_ptr()),
            None,
            Some(&mut data_type as *mut u32 as *mut _),
            Some(buffer.as_mut_ptr() as *mut u8),
            Some(&mut data_size),
        );

        // Always close the key
        let _ = RegCloseKey(h_key);

        if query_result == ERROR_SUCCESS {
            // Convert to string
            // Remove trailing null character if present
            if !buffer.is_empty() && buffer[buffer.len() - 1] == 0 {
                buffer.pop();
            }

            let os_string = OsString::from_wide(&buffer);
            match os_string.into_string() {
                Ok(s) => {
                    debug!("Got string value from registry: {}\\{}\\{} = {}", hive_to_string(hive), key_path, value_name, s);
                    Ok(Some(s))
                }
                Err(_) => {
                    warn!("Failed to convert registry value to string: {}\\{}\\{}", hive_to_string(hive), key_path, value_name);
                    Ok(None)
                }
            }
        } else {
            debug!("Failed to query registry value: {}\\{}\\{}", hive_to_string(hive), key_path, value_name);
            Ok(None)
        }
    }
}

/// Get a DWORD value from the registry
pub fn get_dword_value(hive: HKEY, key_path: &str, value_name: &str) -> Result<Option<u32>> {
    debug!("Getting DWORD value from registry: {}\\{}\\{}", hive_to_string(hive), key_path, value_name);

    let key_path_wide: Vec<u16> = key_path.encode_utf16().chain(std::iter::once(0)).collect();
    let value_name_wide: Vec<u16> = value_name.encode_utf16().chain(std::iter::once(0)).collect();
    let mut h_key = HKEY::default();

    unsafe {
        // Open the key
        let result = RegOpenKeyExW(
            hive,
            PCWSTR::from_raw(key_path_wide.as_ptr()),
            Some(0),
            KEY_QUERY_VALUE,
            &mut h_key,
        );

        if result != ERROR_SUCCESS {
            // Key doesn't exist
            debug!("Registry key does not exist: {}\\{}", hive_to_string(hive), key_path);
            return Ok(None);
        }

        // Query the value
        let mut data_type = 0u32;
        let mut data_size = std::mem::size_of::<u32>() as u32;
        let mut buffer: u32 = 0;

        let query_result = RegQueryValueExW(
            h_key,
            PCWSTR::from_raw(value_name_wide.as_ptr()),
            None,
            Some(&mut data_type as *mut u32 as *mut _),
            Some(&mut buffer as *mut u32 as *mut u8),
            Some(&mut data_size),
        );

        // Always close the key
        let _ = RegCloseKey(h_key);

        if query_result == ERROR_SUCCESS && data_type == REG_DWORD.0 {
            debug!("Got DWORD value from registry: {}\\{}\\{} = {}", hive_to_string(hive), key_path, value_name, buffer);
            Ok(Some(buffer))
        } else {
            debug!("Failed to query registry value or value is not a DWORD: {}\\{}\\{}", hive_to_string(hive), key_path, value_name);
            Ok(None)
        }
    }
}

/// Compare two computer names from registry
pub fn compare_computer_names(active_name: &str, pending_name: &str) -> bool {
    active_name.eq_ignore_ascii_case(pending_name)
}

/// Convert a registry hive to a string representation
fn hive_to_string(hive: HKEY) -> &'static str {
    if hive == HKEY_LOCAL_MACHINE {
        "HKLM"
    } else if hive == HKEY_CURRENT_USER {
        "HKCU"
    } else {
        "Unknown"
    }
}
