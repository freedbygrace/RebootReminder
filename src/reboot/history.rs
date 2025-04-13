use crate::config::RebootConfig;
use crate::database::{DbPool, RebootHistory, DateTimeUtc};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use log::{debug, warn};
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use uuid::Uuid;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, SYSTEMTIME};
use windows::Win32::System::EventLog::{
    EvtClose, EvtCreateRenderContext, EvtNext, EvtQuery, EvtRender,
    EvtSystemComputer, EvtSystemTimeCreated, EvtSystemUserID,
    EVT_HANDLE,
};
use windows::Win32::System::Time::FileTimeToSystemTime;
use windows::Win32::Foundation::FILETIME;

/// Reboot history manager
pub struct RebootHistoryManager {
    _config: RebootConfig,
    db_pool: DbPool,
}

impl RebootHistoryManager {
    /// Create a new reboot history manager
    pub fn new(config: RebootConfig, db_pool: DbPool) -> Self {
        Self { _config: config, db_pool }
    }

    /// Get reboot history from the Windows Event Log
    pub fn get_reboot_history_from_event_log(&self, limit: usize) -> Result<Vec<RebootHistory>> {
        let mut history = Vec::new();

        // Get reboot events from the System event log
        let reboot_events = self.get_reboot_events(limit)?;
        for event in reboot_events {
            history.push(event);
        }

        // Sort by reboot time descending
        history.sort_by(|a, b| b.reboot_time.cmp(&a.reboot_time));

        // Limit to the requested number of events
        if history.len() > limit {
            history.truncate(limit);
        }

        Ok(history)
    }

    /// Get reboot events from the System event log
    fn get_reboot_events(&self, limit: usize) -> Result<Vec<RebootHistory>> {
        let mut events = Vec::new();

        unsafe {
            // Open the System event log
            let query = "Event/System[EventID=1074]";
            let path = "System";
            let path_wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
            let query_wide: Vec<u16> = query.encode_utf16().chain(std::iter::once(0)).collect();

            // Create a query for shutdown events
            let query_handle = EvtQuery(
                None,
                PCWSTR::from_raw(path_wide.as_ptr()),
                PCWSTR::from_raw(query_wide.as_ptr()),
                0, // Use default flags
            )?;

            // Create a render context for system properties
            let context = EvtCreateRenderContext(None, 0)?;

            // Get events
            let mut event_handles = [EVT_HANDLE::default()];
            let mut returned = 0;
            let mut event_handles_raw = [0isize; 1];
            let result = EvtNext(
                query_handle,
                &mut event_handles_raw,
                1,
                0,
                &mut returned,
            );
            if let Ok(_) = result {
                event_handles[0] = std::mem::transmute(event_handles_raw[0]);
            }

            if let Err(e) = result {
                let error_code = windows::Win32::Foundation::GetLastError();
                debug!("EvtNext failed: {}, error code: {}", e, error_code.0);
                if returned == 0 {
                    // No more events
                    let _ = EvtClose(query_handle);
                    return Ok(events);
                }
            }

            // Process each event
            while returned > 0 && events.len() < limit {
                let event_handle = event_handles[0];

                // Get the event XML
                let mut buffer_used = 0;
                let mut property_count = 0;
                let mut buffer_size = 0;

                // First call to get buffer size
                let result = EvtRender(
                    Some(context),
                    event_handle,
                    1, // EVT_RENDER_FLAG_SYSTEM_PROPERTIES
                    buffer_size,
                    Some(std::ptr::null_mut()),
                    &mut buffer_used,
                    &mut property_count,
                );

                if let Err(e) = result {
                    let error_code = windows::Win32::Foundation::GetLastError();
                    if error_code.0 != ERROR_INSUFFICIENT_BUFFER.0 {
                        warn!("EvtRender failed: {}, error code: {}", e, error_code.0);
                        let _ = EvtClose(event_handle);
                        continue;
                    }
                }

                // Allocate buffer
                buffer_size = buffer_used;
                let mut buffer = vec![0u8; buffer_size as usize];

                // Second call to get properties
                let result = EvtRender(
                    Some(context),
                    event_handle,
                    1, // EVT_RENDER_FLAG_SYSTEM_PROPERTIES
                    buffer_size,
                    Some(buffer.as_mut_ptr() as *mut _),
                    &mut buffer_used,
                    &mut property_count,
                );

                if let Err(e) = result {
                    let error_code = windows::Win32::Foundation::GetLastError();
                    warn!("EvtRender failed: {}, error code: {}", e, error_code.0);
                    let _ = EvtClose(event_handle);
                    continue;
                }

                // Get the event properties
                let props = buffer.as_ptr() as *const *const u8;

                // Get the time created
                let time_created = *(props.add(EvtSystemTimeCreated.0 as usize) as *const FILETIME);
                let mut system_time = SYSTEMTIME::default();
                let result = FileTimeToSystemTime(&time_created, &mut system_time);

                if let Err(e) = result {
                    let error_code = windows::Win32::Foundation::GetLastError();
                    warn!("FileTimeToSystemTime failed: {}, error code: {}", e, error_code.0);
                    let _ = EvtClose(event_handle);
                    continue;
                }

                // Convert to DateTime<Utc>
                let reboot_time = DateTime::<Utc>::from_naive_utc_and_offset(
                    chrono::NaiveDateTime::new(
                        chrono::NaiveDate::from_ymd_opt(
                            system_time.wYear as i32,
                            system_time.wMonth as u32,
                            system_time.wDay as u32,
                        )
                        .unwrap_or_default(),
                        chrono::NaiveTime::from_hms_opt(
                            system_time.wHour as u32,
                            system_time.wMinute as u32,
                            system_time.wSecond as u32,
                        )
                        .unwrap_or_default(),
                    ),
                    Utc,
                );

                // Get the computer name
                let computer_ptr = *(props.add(EvtSystemComputer.0 as usize) as *const *const u16);
                let computer_name = if !computer_ptr.is_null() {
                    let len = (0..).take_while(|&i| *computer_ptr.add(i) != 0).count();
                    let computer_slice = std::slice::from_raw_parts(computer_ptr, len);
                    OsString::from_wide(computer_slice)
                        .into_string()
                        .unwrap_or_else(|_| String::from("Unknown"))
                } else {
                    String::from("Unknown")
                };

                // Get the user SID
                let user_sid_ptr = *(props.add(EvtSystemUserID.0 as usize) as *const *const u8);
                let user_name = if !user_sid_ptr.is_null() {
                    // For simplicity, we'll just use "System" for now
                    // In a real implementation, you would use LookupAccountSidW to get the user name
                    String::from("System")
                } else {
                    String::from("Unknown")
                };

                // Create a reboot history entry
                let history = RebootHistory {
                    id: Uuid::new_v4().to_string(),
                    reboot_time,
                    reason: Some(String::from("System shutdown")),
                    source: Some(String::from("Event Log")),
                    user_name: Some(user_name),
                    computer_name: Some(computer_name),
                    success: true,
                    duration: Some(0),
                };

                events.push(history);

                // Close the event handle
                let _ = EvtClose(event_handle);

                // Get the next event
                let mut returned = 0;
                let mut event_handles_raw = [0isize; 1];
                let result = EvtNext(
                    query_handle,
                    &mut event_handles_raw,
                    1,
                    0,
                    &mut returned,
                );
                if let Ok(_) = result {
                    event_handles[0] = std::mem::transmute(event_handles_raw[0]);
                }

                if let Err(e) = result {
                    let error_code = windows::Win32::Foundation::GetLastError();
                    debug!("EvtNext failed: {}, error code: {}", e, error_code.0);
                    if returned == 0 {
                        // No more events
                        break;
                    }
                }
            }

            // Close the query handle
            let _ = EvtClose(query_handle);
        }

        Ok(events)
    }

    /// Get reboot history from the database
    pub fn get_reboot_history_from_db(&self, limit: usize) -> Result<Vec<RebootHistory>> {
        // Get reboot history from the database
        let conn = self.db_pool.get().context("Failed to get database connection")?;
        let mut stmt = conn.prepare(
            "SELECT id, reboot_time, reason, source, user_name, computer_name, success, duration
             FROM reboot_history
             ORDER BY reboot_time DESC
             LIMIT ?",
        )?;

        let history = stmt
            .query_map([limit as i64], |row| {
                Ok(RebootHistory {
                    id: row.get(0)?,
                    reboot_time: row.get::<_, DateTimeUtc>(1)?.into(),
                    reason: row.get(2)?,
                    source: row.get(3)?,
                    user_name: row.get(4)?,
                    computer_name: row.get(5)?,
                    success: row.get(6)?,
                    duration: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(history)
    }

    /// Save reboot history to the database
    pub fn save_reboot_history(&self, history: &RebootHistory) -> Result<()> {
        let conn = self.db_pool.get().context("Failed to get database connection")?;
        conn.execute(
            "INSERT INTO reboot_history (id, reboot_time, reason, source, user_name, computer_name, success, duration)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            (
                &history.id,
                &DateTimeUtc::from(history.reboot_time),
                &history.reason,
                &history.source,
                &history.user_name,
                &history.computer_name,
                &history.success,
                &history.duration,
            ),
        )?;

        Ok(())
    }

    /// Get reboot history
    pub fn get_reboot_history(&self, limit: usize) -> Result<Vec<RebootHistory>> {
        // First try to get history from the database
        let mut history = self.get_reboot_history_from_db(limit)?;

        // If we don't have enough history, try to get more from the event log
        if history.len() < limit {
            let event_log_history = self.get_reboot_history_from_event_log(limit - history.len())?;
            for event in event_log_history {
                // Check if this event is already in the database
                if !history.iter().any(|h| h.reboot_time == event.reboot_time) {
                    // Save to database
                    let _ = self.save_reboot_history(&event);
                    history.push(event);
                }
            }
        }

        // Sort by reboot time descending
        history.sort_by(|a, b| b.reboot_time.cmp(&a.reboot_time));

        // Limit to the requested number of events
        if history.len() > limit {
            history.truncate(limit);
        }

        Ok(history)
    }
}
