use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Reboot state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebootState {
    /// Unique identifier
    pub id: String,

    /// Whether a reboot is required
    pub reboot_required: bool,

    /// Whether a reboot is recommended
    pub reboot_recommended: bool,

    /// Time of last reboot check
    pub last_check_time: DateTime<Utc>,

    /// Time when a reboot was first detected as required
    pub reboot_required_since: Option<DateTime<Utc>>,

    /// Time of last reboot
    pub last_reboot_time: Option<DateTime<Utc>>,

    /// Number of times the reboot has been postponed
    pub postpone_count: u32,

    /// Time of next reminder
    pub next_reminder_time: Option<DateTime<Utc>>,

    /// Time of scheduled reboot
    pub scheduled_reboot_time: Option<DateTime<Utc>>,

    /// Reason for reboot
    pub reboot_reason: Option<String>,

    /// Reboot sources
    pub sources: Vec<RebootSource>,

    /// Creation time
    pub created_at: DateTime<Utc>,

    /// Last update time
    pub updated_at: DateTime<Utc>,
}

impl RebootState {
    /// Create a new reboot state
    pub fn new(reboot_required: bool, reboot_recommended: bool) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            reboot_required,
            reboot_recommended,
            last_check_time: now,
            reboot_required_since: if reboot_required { Some(now) } else { None },
            last_reboot_time: None,
            postpone_count: 0,
            next_reminder_time: None,
            scheduled_reboot_time: None,
            reboot_reason: None,
            sources: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// Reboot source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebootSource {
    /// Unique identifier
    pub id: String,

    /// Source name
    pub name: String,

    /// Source description
    pub description: Option<String>,

    /// Severity (required, recommended, optional)
    pub severity: String,

    /// Time when the source was detected
    pub detected_at: DateTime<Utc>,

    /// Time when the source expires
    pub expires_at: Option<DateTime<Utc>>,

    /// Additional details
    pub details: Option<String>,
}

impl RebootSource {
    /// Create a new reboot source
    pub fn new(name: &str, description: Option<&str>, severity: &str) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
            severity: severity.to_string(),
            detected_at: now,
            expires_at: None,
            details: None,
        }
    }
}

/// Reboot history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebootHistory {
    /// Unique identifier
    pub id: String,

    /// Time of reboot
    pub reboot_time: DateTime<Utc>,

    /// Reason for reboot
    pub reason: Option<String>,

    /// Source of reboot
    pub source: Option<String>,

    /// User who initiated the reboot
    pub user_name: Option<String>,

    /// Computer name
    pub computer_name: Option<String>,

    /// Whether the reboot was successful
    pub success: bool,

    /// Duration of reboot in seconds
    pub duration: Option<i64>,
}

impl RebootHistory {
    /// Create a new reboot history entry
    pub fn new(reboot_time: DateTime<Utc>, success: bool) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            reboot_time,
            reason: None,
            source: None,
            user_name: None,
            computer_name: None,
            success,
            duration: None,
        }
    }
}

/// Notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    /// Unique identifier
    pub id: String,

    /// Time of notification
    pub timestamp: DateTime<Utc>,

    /// Notification type
    pub notification_type: String,

    /// Notification message
    pub message: String,

    /// User who received the notification
    pub user_name: Option<String>,

    /// Whether the notification has been dismissed
    pub dismissed: bool,

    /// Action associated with the notification
    pub action: Option<String>,

    /// Creation time
    pub created_at: DateTime<Utc>,
}

impl Notification {
    /// Create a new notification
    pub fn new(notification_type: &str, message: &str, user_name: Option<&str>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: now,
            notification_type: notification_type.to_string(),
            message: message.to_string(),
            user_name: user_name.map(|s| s.to_string()),
            dismissed: false,
            action: None,
            created_at: now,
        }
    }
}

/// Notification interaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationInteraction {
    /// Unique identifier
    pub id: String,

    /// Notification ID
    pub notification_id: String,

    /// Time of interaction
    pub timestamp: DateTime<Utc>,

    /// Action taken
    pub action: String,

    /// User who interacted with the notification
    pub user_name: Option<String>,

    /// Session ID
    pub session_id: Option<String>,

    /// Additional details
    pub details: Option<String>,
}

impl NotificationInteraction {
    /// Create a new notification interaction
    pub fn new(notification_id: &str, action: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            notification_id: notification_id.to_string(),
            timestamp: Utc::now(),
            action: action.to_string(),
            user_name: None,
            session_id: None,
            details: None,
        }
    }

    /// Create a new notification interaction with detailed information
    pub fn new_detailed(notification_id: &str, action: &str, user_name: Option<&str>,
                        session_id: Option<&str>, details: Option<&str>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            notification_id: notification_id.to_string(),
            timestamp: Utc::now(),
            action: action.to_string(),
            user_name: user_name.map(|s| s.to_string()),
            session_id: session_id.map(|s| s.to_string()),
            details: details.map(|s| s.to_string()),
        }
    }

    /// Add user information to the interaction
    pub fn with_user(mut self, user_name: &str) -> Self {
        self.user_name = Some(user_name.to_string());
        self
    }

    /// Add session information to the interaction
    pub fn with_session(mut self, session_id: &str) -> Self {
        self.session_id = Some(session_id.to_string());
        self
    }

    /// Add details to the interaction
    pub fn with_details(mut self, details: &str) -> Self {
        self.details = Some(details.to_string());
        self
    }
}

/// User session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSession {
    /// Unique identifier
    pub id: String,

    /// User name
    pub user_name: String,

    /// Session ID
    pub session_id: String,

    /// Logon time
    pub logon_time: DateTime<Utc>,

    /// Whether the session is active
    pub is_active: bool,

    /// Whether the session is RDP
    pub is_rdp: bool,

    /// Whether the session is console
    pub is_console: bool,

    /// Client name
    pub client_name: Option<String>,

    /// Client IP address
    pub client_ip: Option<String>,

    /// Display name
    pub display_name: Option<String>,

    /// Time of last activity
    pub last_activity: Option<DateTime<Utc>>,

    /// Creation time
    pub created_at: DateTime<Utc>,

    /// Last update time
    pub updated_at: DateTime<Utc>,
}

impl UserSession {
    /// Create a new user session
    pub fn new(user_name: &str, session_id: &str, is_rdp: bool, is_console: bool) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            user_name: user_name.to_string(),
            session_id: session_id.to_string(),
            logon_time: now,
            is_active: true,
            is_rdp,
            is_console,
            client_name: None,
            client_ip: None,
            display_name: None,
            last_activity: Some(now),
            created_at: now,
            updated_at: now,
        }
    }
}
