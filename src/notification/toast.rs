use anyhow::Result;
use std::path::Path;

/// Toast notification
#[derive(Debug, Clone)]
pub struct ToastNotification {
    /// Title of the notification
    pub title: String,

    /// Message body
    pub message: String,

    /// Path to the icon
    pub icon_path: String,

    /// Action URI
    pub action_uri: Option<String>,

    /// Unique identifier
    pub id: String,
}

impl ToastNotification {
    /// Create a new toast notification
    pub fn new(title: &str, message: &str) -> Self {
        Self {
            title: title.to_string(),
            message: message.to_string(),
            icon_path: String::new(),
            action_uri: None,
            id: String::new(),
        }
    }

    /// Create a new toast notification with icon and ID
    pub fn new_with_icon(title: &str, message: &str, icon_path: &Path, id: uuid::Uuid) -> Self {
        Self {
            title: title.to_string(),
            message: message.to_string(),
            icon_path: icon_path.to_string_lossy().to_string(),
            action_uri: None,
            id: id.to_string(),
        }
    }

    /// Show the notification
    pub fn show(&self) -> Result<()> {
        use winrt_notification::{Toast, Duration, Sound};

        // Create a new toast notification
        let mut toast = Toast::new(Toast::POWERSHELL_APP_ID);
        toast = toast.title(&self.title);
        toast = toast.text1(&self.message);

        // Set longer duration for important notifications
        toast = toast.duration(Duration::Long);

        // Add a sound for better visibility
        toast = toast.sound(Some(Sound::Default));

        // Add icon if it exists
        let icon_path = Path::new(&self.icon_path);
        if icon_path.exists() {
            // Skip icon for now due to API compatibility issues
            // toast = toast.icon(icon_path, IconCrop::Circle, "App Icon");
        }

        // Add action if provided
        if let Some(action_uri) = &self.action_uri {
            // For reboot action, add a clear button
            if action_uri.starts_with("reboot:") {
                toast = toast.text2("Click to restart your computer now");
                // In a real implementation, we would use the button API
                // toast = toast.button("Restart Now", action_uri);
            } else {
                // For other actions, just show the action text
                let action_text = format!("Action: {}", action_uri);
                toast = toast.text2(&action_text);
            }
        }

        // Show the notification
        toast.show()?;

        Ok(())
    }
}
