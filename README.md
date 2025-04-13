# Reboot Reminder v2025.04.12-2230

A cross-platform reboot reminder system that runs as a Windows service and provides customizable notifications to users when system reboots are necessary.

## Features

- Runs as a Windows service under NTAUTHORITY/System
- Uses passwordless user impersonation to interact with interactive user sessions
- Displays notifications only when interactive console or RDP user sessions are present
- Customizable notifications via JSON or XML configuration files
- Supports both tray and toast notifications
- Tracks system reboot history using Windows events
- Detects when reboots are necessary using multiple methods with native Windows API calls
- Stores state using embedded database
- Comprehensive logging with rotation and detailed database operations
- Customizable reboot reminder timeframes and deferral options with flexible timespan format (e.g., "30m", "2h")
- Supports quiet hours
- Supports Windows environment variables in configuration paths
- Flexible timespan format for reminder intervals and deferrals
- Optional watchdog service for improved reliability
- Detailed tracking of how long a reboot has been required
- Enhanced logging for configuration loading and reboot detection
- Allows users to initiate system restart directly from notifications with confirmation dialog and countdown

## Requirements

- Windows operating system
- Administrative privileges for installation

## Installation

### Using MSI Installer

1. Download the latest MSI installer from the [Releases](https://github.com/freedbygrace/RebootReminder/releases) page.
2. Run the installer with administrative privileges.
3. Follow the installation wizard.

### Command Line Installation

```powershell
# Install the service
reboot_reminder.exe install --name "RebootReminder" --display-name "Reboot Reminder Service" --description "Provides notifications when system reboots are necessary"

# Start the service
Start-Service -Name "RebootReminder"
```

See the [Installation Guide](docs/INSTALLATION.md) for detailed instructions.

## Usage

Once installed, the service will run automatically at system startup. It will check if a reboot is required at regular intervals and notify users when necessary.

### Command-line Options

The application supports the following command-line options:

```
reboot_reminder.exe [OPTIONS] [COMMAND]
```

#### Options

- `--config <PATH>` - Path to the configuration file. If not specified, the application will look for `config.json` in the same directory as the executable.

#### Commands

- `install` - Install the service
- `uninstall` - Uninstall the service
- `run` - Run the application (as a service if installed, or as a console application otherwise)
- `check` - Check if a reboot is required and exit

Example:

```powershell
# Run with a specific configuration file
reboot_reminder.exe --config "C:\path\to\config.json" run

# Check if a reboot is required
reboot_reminder.exe check
```

## Configuration

The application can be configured using either JSON or XML configuration files. The configuration can be loaded from a local file or a URL.

Example configuration:

```json
{
  "service": {
    "name": "RebootReminder",
    "displayName": "Reboot Reminder Service",
    "description": "Provides notifications when system reboots are necessary",
    "configRefreshMinutes": 60
  },
  "notification": {
    "type": "both",
    "branding": {
      "title": "Reboot Reminder",
      "iconPath": "icon.ico",
      "company": "IT Department"
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
        "reminderInterval": "4h",
        "deferrals": ["1h", "4h", "8h", "24h"]
      },
      {
        "minHours": 49,
        "maxHours": 72,
        "reminderInterval": "2h",
        "deferrals": ["1h", "2h", "4h"]
      },
      {
        "minHours": 73,
        "maxHours": null,
        "reminderInterval": "30m",
        "deferrals": ["30m", "1h"]
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
    "path": "%PROGRAMDATA%\\RebootReminder\\rebootreminder.db"
  },
  "logging": {
    "path": "%PROGRAMDATA%\\RebootReminder\\logs\\rebootreminder.log",
    "level": "info",
    "maxFiles": 7,
    "maxSize": 10
  },
  "watchdog": {
    "enabled": true,
    "checkIntervalSeconds": 60,
    "maxRestartAttempts": 3,
    "restartDelaySeconds": 10,
    "servicePath": "",
    "serviceName": "RebootReminder"
  }
}
```

### Environment Variables

The configuration supports Windows environment variables in paths. For example:

```json
"database": {
  "path": "%PROGRAMDATA%\\RebootReminder\\rebootreminder.db"
}
```

Common environment variables:

- `%PROGRAMDATA%` - Usually `C:\ProgramData`
- `%USERPROFILE%` - User's home directory
- `%APPDATA%` - User's application data directory
- `%TEMP%` - Temporary directory
- `%WINDIR%` - Windows directory

See the [Configuration Guide](docs/CONFIGURATION.md) for details on customizing the application.

## Development

### Prerequisites

- Rust 1.70.0 or later
- Cargo
- Windows SDK
- WiX Toolset (for MSI generation)

### Building from Source

```bash
# Clone the repository
git clone https://github.com/freedbygrace/RebootReminder.git
cd RebootReminder

# Build the project
cargo build --release

# Run tests
cargo test

# Generate MSI installer
cargo wix
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for version history and changes.
