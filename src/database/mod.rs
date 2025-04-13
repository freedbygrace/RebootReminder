mod models;

use anyhow::{Context, Result};
use log::{debug, info};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection, OptionalExtension, types::{FromSql, FromSqlResult, ToSql, ToSqlOutput, ValueRef}};
use chrono::{DateTime, Utc, TimeZone};
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

pub use models::*;

/// Database connection pool
pub type DbPool = Arc<Pool<SqliteConnectionManager>>;

// Define a wrapper type for DateTime<Utc> to implement FromSql and ToSql
#[derive(Debug, Clone)]
pub struct DateTimeUtc(pub DateTime<Utc>);

impl FromSql for DateTimeUtc {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value {
            ValueRef::Text(s) => {
                let text = std::str::from_utf8(s).map_err(|_| rusqlite::types::FromSqlError::InvalidType)?;
                DateTime::parse_from_rfc3339(text)
                    .map(|dt| DateTimeUtc(dt.with_timezone(&Utc)))
                    .map_err(|_| rusqlite::types::FromSqlError::InvalidType)
            },
            ValueRef::Integer(i) => {
                Ok(DateTimeUtc(Utc.timestamp_opt(i, 0).unwrap()))
            },
            _ => Err(rusqlite::types::FromSqlError::InvalidType),
        }
    }
}

impl ToSql for DateTimeUtc {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        let s = self.0.to_rfc3339();
        Ok(ToSqlOutput::from(s))
    }
}

// Define a wrapper type for Uuid to implement FromSql and ToSql
#[derive(Debug, Clone)]
pub struct UuidWrapper(pub Uuid);

impl FromSql for UuidWrapper {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value {
            ValueRef::Text(s) => {
                let text = std::str::from_utf8(s).map_err(|_| rusqlite::types::FromSqlError::InvalidType)?;
                Uuid::parse_str(text)
                    .map(UuidWrapper)
                    .map_err(|_| rusqlite::types::FromSqlError::InvalidType)
            },
            _ => Err(rusqlite::types::FromSqlError::InvalidType),
        }
    }
}

impl ToSql for UuidWrapper {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        let s = self.0.to_string();
        Ok(ToSqlOutput::from(s))
    }
}

// Implement conversions between UuidWrapper and Uuid
impl From<Uuid> for UuidWrapper {
    fn from(uuid: Uuid) -> Self {
        UuidWrapper(uuid)
    }
}

impl From<UuidWrapper> for Uuid {
    fn from(wrapper: UuidWrapper) -> Self {
        wrapper.0
    }
}

// Implement conversions between DateTimeUtc and DateTime<Utc>
impl From<DateTime<Utc>> for DateTimeUtc {
    fn from(dt: DateTime<Utc>) -> Self {
        DateTimeUtc(dt)
    }
}

impl From<DateTimeUtc> for DateTime<Utc> {
    fn from(dt: DateTimeUtc) -> Self {
        dt.0
    }
}

/// Check if a table exists in the database
fn table_exists(conn: &Connection, table_name: &str) -> Result<bool> {
    let query = "SELECT name FROM sqlite_master WHERE type='table' AND name=?";
    let exists: Option<String> = conn.query_row(
        query,
        params![table_name],
        |row| row.get(0)
    ).optional()?;

    Ok(exists.is_some())
}

/// Initialize the database
pub fn init(config: &crate::config::DatabaseConfig) -> Result<DbPool> {
    let db_path = &config.path;
    info!("Initializing database at {}", db_path);

    // Create parent directory if it doesn't exist
    if let Some(parent) = Path::new(db_path).parent() {
        if !parent.exists() {
            info!("Creating database directory: {:?}", parent);
            std::fs::create_dir_all(parent).context("Failed to create database directory")?;
        } else {
            info!("Database directory already exists: {:?}", parent);
        }
    }

    // Create connection manager
    info!("Creating SQLite connection manager for {}", db_path);
    let manager = SqliteConnectionManager::file(db_path);

    // Create connection pool
    info!("Creating database connection pool");
    let pool = Pool::new(manager).context("Failed to create database connection pool")?;

    // Initialize database schema
    info!("Getting database connection from pool");
    let conn = pool.get().context("Failed to get database connection")?;

    info!("Initializing database schema");
    init_schema(&conn).context("Failed to initialize database schema")?;

    info!("Database initialized successfully");
    Ok(Arc::new(pool))
}

/// Initialize database schema
fn init_schema(conn: &Connection) -> Result<()> {
    info!("Initializing database schema");

    // Enable foreign keys
    debug!("Enabling SQLite foreign keys");
    conn.execute("PRAGMA foreign_keys = ON", [])?;

    // Create reboot_history table
    let query = "CREATE TABLE IF NOT EXISTS reboot_history (
        id TEXT PRIMARY KEY,
        reboot_time TEXT NOT NULL,
        reason TEXT,
        source TEXT,
        user_name TEXT,
        computer_name TEXT,
        success INTEGER NOT NULL,
        duration INTEGER
    )";

    // Check if table exists before creating
    let exists = table_exists(conn, "reboot_history")?;
    if !exists {
        info!("Creating reboot_history table with query: {}", query);
        conn.execute(query, [])?;
    } else {
        debug!("reboot_history table already exists");
    }

    // Create reboot_state table
    let query = "CREATE TABLE IF NOT EXISTS reboot_state (
        id TEXT PRIMARY KEY,
        reboot_required INTEGER NOT NULL,
        reboot_recommended INTEGER NOT NULL,
        last_check_time TEXT NOT NULL,
        reboot_required_since TEXT,
        last_reboot_time TEXT,
        postpone_count INTEGER NOT NULL,
        next_reminder_time TEXT,
        scheduled_reboot_time TEXT,
        reboot_reason TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    )";

    // Check if table exists before creating
    let exists = table_exists(conn, "reboot_state")?;
    if !exists {
        info!("Creating reboot_state table with query: {}", query);
        conn.execute(query, [])?;
    } else {
        debug!("reboot_state table already exists");
    }

    // Create reboot_sources table
    let query = "CREATE TABLE IF NOT EXISTS reboot_sources (
        id TEXT PRIMARY KEY,
        reboot_state_id TEXT NOT NULL,
        name TEXT NOT NULL,
        description TEXT,
        severity TEXT NOT NULL,
        detected_at TEXT NOT NULL,
        expires_at TEXT,
        details TEXT,
        FOREIGN KEY (reboot_state_id) REFERENCES reboot_state (id) ON DELETE CASCADE
    )";

    // Check if table exists before creating
    let exists = table_exists(conn, "reboot_sources")?;
    if !exists {
        info!("Creating reboot_sources table with query: {}", query);
        conn.execute(query, [])?;
    } else {
        debug!("reboot_sources table already exists");
    }

    // Create notifications table
    let query = "CREATE TABLE IF NOT EXISTS notifications (
        id TEXT PRIMARY KEY,
        timestamp TEXT NOT NULL,
        type TEXT NOT NULL,
        message TEXT NOT NULL,
        user_name TEXT,
        dismissed INTEGER NOT NULL,
        action TEXT,
        created_at TEXT NOT NULL
    )";

    // Check if table exists before creating
    let exists = table_exists(conn, "notifications")?;
    if !exists {
        info!("Creating notifications table with query: {}", query);
        conn.execute(query, [])?;
    } else {
        debug!("notifications table already exists");
    }

    // Create notification_interactions table
    let query = "CREATE TABLE IF NOT EXISTS notification_interactions (
        id TEXT PRIMARY KEY,
        notification_id TEXT NOT NULL,
        timestamp TEXT NOT NULL,
        action TEXT NOT NULL,
        user_name TEXT,
        session_id TEXT,
        details TEXT,
        FOREIGN KEY (notification_id) REFERENCES notifications (id) ON DELETE CASCADE
    )";

    // Check if table exists before creating
    let exists = table_exists(conn, "notification_interactions")?;
    if !exists {
        info!("Creating notification_interactions table with query: {}", query);
        conn.execute(query, [])?;
    } else {
        debug!("notification_interactions table already exists");
    }

    // Create user_sessions table
    let query = "CREATE TABLE IF NOT EXISTS user_sessions (
        id TEXT PRIMARY KEY,
        user_name TEXT NOT NULL,
        session_id TEXT NOT NULL,
        logon_time TEXT NOT NULL,
        is_active INTEGER NOT NULL,
        is_rdp INTEGER NOT NULL,
        is_console INTEGER NOT NULL,
        client_name TEXT,
        client_ip TEXT,
        display_name TEXT,
        last_activity TEXT NOT NULL,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    )";

    // Check if table exists before creating
    let exists = table_exists(conn, "user_sessions")?;
    if !exists {
        info!("Creating user_sessions table with query: {}", query);
        conn.execute(query, [])?;
    } else {
        debug!("user_sessions table already exists");
    }

    info!("Database schema initialized successfully");
    Ok(())
}

/// Get the current reboot state
pub fn get_reboot_state(pool: &DbPool) -> Result<Option<RebootState>> {
    info!("Getting current reboot state from database");
    let conn = pool.get().context("Failed to get database connection")?;

    let query = "SELECT id, reboot_required, reboot_recommended, last_check_time, reboot_required_since, last_reboot_time,
         postpone_count, next_reminder_time, scheduled_reboot_time, reboot_reason,
         created_at, updated_at FROM reboot_state ORDER BY created_at DESC LIMIT 1";

    info!("Executing query: {}", query);
    let state = conn.query_row(
        query,
        [],
        |row| {
            Ok(RebootState {
                id: row.get::<_, UuidWrapper>(0)?.into(),
                reboot_required: row.get(1)?,
                reboot_recommended: row.get(2)?,
                last_check_time: row.get::<_, DateTimeUtc>(3)?.into(),
                reboot_required_since: row.get::<_, Option<DateTimeUtc>>(4)?.map(Into::into),
                last_reboot_time: row.get::<_, Option<DateTimeUtc>>(5)?.map(Into::into),
                postpone_count: row.get(6)?,
                next_reminder_time: row.get::<_, Option<DateTimeUtc>>(7)?.map(Into::into),
                scheduled_reboot_time: row.get::<_, Option<DateTimeUtc>>(8)?.map(Into::into),
                reboot_reason: row.get(9)?,
                sources: Vec::new(), // Will be populated separately
                created_at: row.get::<_, DateTimeUtc>(10)?.into(),
                updated_at: row.get::<_, DateTimeUtc>(11)?.into(),
            })
        },
    ).optional().context(format!("Failed to execute query: {}", query))?;

    // Log the result
    match &state {
        Some(_) => info!("Found existing reboot state in database"),
        None => info!("No reboot state found in database"),
    }

    // If we found a state, get its sources
    if let Some(mut state) = state {
        let sources_query = "SELECT id, name, description, severity, detected_at, expires_at, details
             FROM reboot_sources WHERE reboot_state_id = ?";

        info!("Executing query: {} with params: [{}]", sources_query, state.id);
        let mut stmt = conn.prepare(sources_query)
            .context(format!("Failed to prepare query: {}", sources_query))?;

        let sources = stmt.query_map([&UuidWrapper::from(state.id)], |row| {
            Ok(RebootSource {
                id: row.get::<_, UuidWrapper>(0)?.into(),
                name: row.get(1)?,
                description: row.get(2)?,
                severity: row.get(3)?,
                detected_at: row.get::<_, DateTimeUtc>(4)?.into(),
                expires_at: row.get::<_, Option<DateTimeUtc>>(5)?.map(Into::into),
                details: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        state.sources = sources;
        Ok(Some(state))
    } else {
        Ok(None)
    }
}

/// Save a reboot state
pub fn save_reboot_state(pool: &DbPool, state: &RebootState) -> Result<()> {
    info!("Saving reboot state to database: id={}, required={}", state.id, state.reboot_required);
    let mut conn = pool.get().context("Failed to get database connection")?;

    // Begin transaction
    info!("Beginning database transaction");
    let tx = conn.transaction()?;

    // Insert or update reboot state
    let state_query = "INSERT OR REPLACE INTO reboot_state (
            id, reboot_required, reboot_recommended, last_check_time, reboot_required_since, last_reboot_time,
            postpone_count, next_reminder_time, scheduled_reboot_time, reboot_reason,
            created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";

    info!("Executing query to save reboot state: {}", state_query);
    tx.execute(
        state_query,
        params![
            UuidWrapper::from(state.id),
            state.reboot_required,
            state.reboot_recommended,
            DateTimeUtc::from(state.last_check_time),
            state.reboot_required_since.map(DateTimeUtc::from),
            state.last_reboot_time.map(DateTimeUtc::from),
            state.postpone_count,
            state.next_reminder_time.map(DateTimeUtc::from),
            state.scheduled_reboot_time.map(DateTimeUtc::from),
            state.reboot_reason,
            DateTimeUtc::from(state.created_at),
            DateTimeUtc::from(state.updated_at),
        ],
    )?;

    info!("Reboot state saved successfully");

    // Delete existing sources
    let delete_query = "DELETE FROM reboot_sources WHERE reboot_state_id = ?";
    info!("Executing query to delete existing reboot sources: {}", delete_query);
    let deleted_rows = tx.execute(
        delete_query,
        [&UuidWrapper::from(state.id)],
    )?;
    info!("Deleted {} existing reboot sources", deleted_rows);

    // Insert new sources
    info!("Inserting {} new reboot sources", state.sources.len());
    let insert_query = "INSERT INTO reboot_sources (
                id, reboot_state_id, name, description, severity, detected_at, expires_at, details
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)";

    for source in &state.sources {
        info!("Inserting reboot source: id={}, name={}", source.id, source.name);
        tx.execute(
            insert_query,
            params![
                UuidWrapper::from(source.id),
                UuidWrapper::from(state.id),
                source.name,
                source.description,
                source.severity,
                DateTimeUtc::from(source.detected_at),
                source.expires_at.map(DateTimeUtc::from),
                source.details,
            ],
        )?;
    }

    // Commit transaction
    info!("Committing database transaction");
    tx.commit()?;

    info!("Reboot state and sources saved successfully");
    Ok(())
}

/// Add a reboot history entry
pub fn add_reboot_history(pool: &DbPool, history: &RebootHistory) -> Result<()> {
    info!("Adding reboot history entry to database: id={}, time={}", history.id, history.reboot_time);
    let conn = pool.get().context("Failed to get database connection")?;

    let query = "INSERT INTO reboot_history (
            id, reboot_time, reason, source, user_name, computer_name, success, duration
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)";

    info!("Executing query: {} with params: [id={}, time={}]", query, history.id, history.reboot_time);
    conn.execute(
        query,
        params![
            UuidWrapper::from(history.id),
            DateTimeUtc::from(history.reboot_time),
            history.reason,
            history.source,
            history.user_name,
            history.computer_name,
            history.success,
            history.duration,
        ],
    ).context(format!("Failed to execute query: {}", query))?;

    info!("Reboot history entry added successfully");
    Ok(())
}

/// Get reboot history
pub fn get_reboot_history(pool: &DbPool, limit: Option<u32>) -> Result<Vec<RebootHistory>> {
    info!("Getting reboot history from database");
    let conn = pool.get().context("Failed to get database connection")?;

    let limit_clause = limit.map_or(String::from(""), |l| format!("LIMIT {}", l));

    let query = format!(
        "SELECT id, reboot_time, reason, source, user_name, computer_name, success, duration
         FROM reboot_history ORDER BY reboot_time DESC {}",
        limit_clause
    );

    info!("Executing query: {}", query);
    let mut stmt = conn.prepare(&query)
        .context(format!("Failed to prepare query: {}", query))?;

    let history = stmt.query_map([], |row| {
        Ok(RebootHistory {
            id: row.get::<_, UuidWrapper>(0)?.into(),
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

/// Add a notification
pub fn add_notification(pool: &DbPool, notification: &Notification) -> Result<()> {
    info!("Adding notification to database: id={}, type={}, user={}",
          notification.id,
          notification.notification_type,
          notification.user_name.as_deref().unwrap_or("<unknown>"));

    let conn = pool.get().context("Failed to get database connection")?;

    let query = "INSERT INTO notifications (
            id, timestamp, type, message, user_name, dismissed, action, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)";

    info!("Executing query: {}", query);
    conn.execute(
        query,
        params![
            UuidWrapper::from(notification.id),
            DateTimeUtc::from(notification.timestamp),
            notification.notification_type,
            notification.message,
            notification.user_name,
            notification.dismissed,
            notification.action,
            DateTimeUtc::from(notification.created_at),
        ],
    )?;

    info!("Notification added successfully: {}", notification.message);
    Ok(())
}

/// Get notifications
pub fn get_notifications(pool: &DbPool, limit: Option<u32>) -> Result<Vec<Notification>> {
    let conn = pool.get().context("Failed to get database connection")?;

    let limit_clause = limit.map_or(String::from(""), |l| format!("LIMIT {}", l));

    let mut stmt = conn.prepare(&format!(
        "SELECT id, timestamp, type, message, user_name, dismissed, action, created_at
         FROM notifications ORDER BY timestamp DESC {}",
        limit_clause
    ))?;

    let notifications = stmt.query_map([], |row| {
        Ok(Notification {
            id: row.get::<_, UuidWrapper>(0)?.into(),
            timestamp: row.get::<_, DateTimeUtc>(1)?.into(),
            notification_type: row.get(2)?,
            message: row.get(3)?,
            user_name: row.get(4)?,
            dismissed: row.get(5)?,
            action: row.get(6)?,
            created_at: row.get::<_, DateTimeUtc>(7)?.into(),
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;

    Ok(notifications)
}

/// Add a notification interaction
pub fn add_notification_interaction(pool: &DbPool, interaction: &NotificationInteraction) -> Result<()> {
    info!("Adding notification interaction to database: id={}, notification_id={}, action={}",
          interaction.id,
          interaction.notification_id,
          interaction.action);

    info!("Interaction details: user={}, session={}, time={}",
          interaction.user_name.as_deref().unwrap_or("<unknown>"),
          interaction.session_id.as_deref().unwrap_or("<unknown>"),
          interaction.timestamp);

    if let Some(details) = &interaction.details {
        info!("Interaction additional details: {}", details);
    }

    let conn = pool.get().context("Failed to get database connection")?;

    let query = "INSERT INTO notification_interactions (
            id, notification_id, timestamp, action, user_name, session_id, details
        ) VALUES (?, ?, ?, ?, ?, ?, ?)";

    info!("Executing query: {}", query);
    conn.execute(
        query,
        params![
            UuidWrapper::from(interaction.id),
            UuidWrapper::from(interaction.notification_id),
            DateTimeUtc::from(interaction.timestamp),
            interaction.action,
            interaction.user_name,
            interaction.session_id,
            interaction.details,
        ],
    )?;

    info!("Notification interaction added successfully: {} by {}",
          interaction.action,
          interaction.user_name.as_deref().unwrap_or("<unknown>"));
    Ok(())
}

/// Save a user session
pub fn save_user_session(pool: &DbPool, session: &UserSession) -> Result<()> {
    info!("Saving user session to database: id={}, user={}, session_id={}",
          session.id, session.user_name, session.session_id);
    let conn = pool.get().context("Failed to get database connection")?;

    let query = "INSERT OR REPLACE INTO user_sessions (
            id, user_name, session_id, logon_time, is_active, is_rdp, is_console,
            client_name, client_ip, display_name, last_activity, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)";

    info!("Executing query: {} with params: [id={}, user={}, session_id={}]",
          query, session.id, session.user_name, session.session_id);
    conn.execute(
        query,
        params![
            UuidWrapper::from(session.id),
            session.user_name,
            session.session_id,
            DateTimeUtc::from(session.logon_time),
            session.is_active,
            session.is_rdp,
            session.is_console,
            session.client_name,
            session.client_ip,
            session.display_name,
            match &session.last_activity {
                Some(dt) => Some(DateTimeUtc::from(*dt)),
                None => None,
            },
            DateTimeUtc::from(session.created_at),
            DateTimeUtc::from(session.updated_at),
        ],
    ).context(format!("Failed to execute query: {}", query))?;

    info!("User session saved successfully");
    Ok(())
}

/// Get active user sessions
pub fn get_active_user_sessions(pool: &DbPool) -> Result<Vec<UserSession>> {
    info!("Getting active user sessions from database");
    let conn = pool.get().context("Failed to get database connection")?;

    let query = "SELECT id, user_name, session_id, logon_time, is_active, is_rdp, is_console,
         client_name, client_ip, display_name, last_activity, created_at, updated_at
         FROM user_sessions WHERE is_active = 1 ORDER BY logon_time DESC";

    info!("Executing query: {}", query);
    let mut stmt = conn.prepare(query)
        .context(format!("Failed to prepare query: {}", query))?;

    let sessions = stmt.query_map([], |row| {
        Ok(UserSession {
            id: row.get::<_, UuidWrapper>(0)?.into(),
            user_name: row.get(1)?,
            session_id: row.get(2)?,
            logon_time: row.get::<_, DateTimeUtc>(3)?.into(),
            is_active: row.get(4)?,
            is_rdp: row.get(5)?,
            is_console: row.get(6)?,
            client_name: row.get(7)?,
            client_ip: row.get(8)?,
            display_name: row.get(9)?,
            last_activity: match row.get::<_, Option<DateTimeUtc>>(10)? {
                Some(dt) => Some(dt.into()),
                None => None,
            },
            created_at: row.get::<_, DateTimeUtc>(11)?.into(),
            updated_at: row.get::<_, DateTimeUtc>(12)?.into(),
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;

    Ok(sessions)
}
