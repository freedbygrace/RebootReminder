# Configuration Guide

RebootReminder can be configured using either JSON or XML configuration files. This guide explains the available configuration options and how to customize the application.

## Configuration File Location

By default, the application looks for a configuration file named `config.json` in the same directory as the executable. You can specify a different configuration file using the `--config` command-line parameter:

```
reboot_reminder.exe --config "C:\path\to\config.json"
```

### Supported Configuration Paths

The application supports the following types of configuration paths:

1. **Local file paths**:
   ```
   reboot_reminder.exe --config "C:\path\to\config.json"
   ```

2. **UNC network paths**:
   ```
   reboot_reminder.exe --config "\\server\share\config.json"
   ```

3. **HTTP/HTTPS URLs**:
   ```
   reboot_reminder.exe --config "https://example.com/config.json"
   ```

4. **File URLs**:
   ```
   reboot_reminder.exe --config "file:///C:/path/to/config.json"
   ```

> **Note**: Only HTTP, HTTPS, and file URL schemes are supported. Other URL schemes (such as FTP) will result in an error.

## Configuration Format

The configuration file can be in either JSON or XML format. The application automatically detects the format based on the file extension.

### JSON Example

```json
{
  "service": {
    "name": "RebootReminder",
    "displayName": "Reboot Reminder Service",
    "description": "Provides notifications when system reboots are necessary",
    "configRefreshMinutes": 60
  },
  "notification": {
    "showToast": true,
    "showTray": true,
    "showBalloon": false,
    "branding": {
      "title": "Reboot Reminder",
      "iconPath": "%PROGRAMDATA%\\RebootReminder\\icon.ico",
      "company": "IT Department"
    },
    "messages": {
      "rebootRequired": "Your computer requires a reboot to complete recent updates.",
      "rebootRecommended": "It is recommended to reboot your computer to apply recent updates.",
      "rebootScheduled": "Your computer is scheduled to reboot at %s.",
      "rebootInProgress": "Your computer will reboot in %s.",
      "rebootCancelled": "The scheduled reboot has been cancelled.",
      "rebootPostponed": "The reboot has been postponed for %s.",
      "rebootCompleted": "Your computer has been successfully rebooted.",
      "actionRequired": "Reboot is required. Click to schedule.",
      "actionRecommended": "Reboot is recommended. Click for options.",
      "actionNotRequired": "No reboot is required at this time.",
      "actionNotAvailable": "Reboot options are not available at this time."
    },
    "quietHours": {
      "enabled": true,
      "startTime": "22:00",
      "endTime": "08:00",
      "daysOfWeek": [0, 1, 2, 3, 4, 5, 6]
    }
  },
  "reboot": {
    "systemReboot": {
      "enabled": true,
      "countdown": "30s",
      "showConfirmation": true,
      "confirmationMessage": "The system needs to restart. Do you want to restart now?",
      "confirmationTitle": "System Restart Required"
    },
    "timeframes": [
      {
        "min": "24h",
        "max": "48h",
        "reminderInterval": "4h",
        "deferrals": ["1h", "4h", "8h", "24h"]
      },
      {
        "min": "49h",
        "max": "72h",
        "reminderInterval": "2h",
        "deferrals": ["1h", "2h", "4h"]
      },
      {
        "min": "73h",
        "max": null,
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
    "path": "%PROGRAMDATA%\\RebootReminder\\reboot_reminder.db"
  },
  "logging": {
    "path": "%PROGRAMDATA%\\RebootReminder\\logs\\reboot_reminder.log",
    "level": "info",
    "maxFiles": 7,
    "maxSize": 10
  },
  "watchdog": {
    "enabled": true,
    "checkInterval": "5m",
    "maxRestartAttempts": 3,
    "restartDelay": "30s",
    "serviceName": "RebootReminder"
  }
}
```

### XML Example

```xml
<?xml version="1.0" encoding="UTF-8"?>
<config>
  <service>
    <n>RebootReminder</n>
    <displayName>Reboot Reminder Service</displayName>
    <description>Provides notifications when system reboots are necessary</description>
    <configRefreshMinutes>60</configRefreshMinutes>
  </service>
  <notification>
    <showToast>true</showToast>
    <showTray>true</showTray>
    <showBalloon>false</showBalloon>
    <branding>
      <title>Reboot Reminder</title>
      <iconPath>%PROGRAMDATA%\RebootReminder\icon.ico</iconPath>
      <company>IT Department</company>
    </branding>
    <messages>
      <rebootRequired>Your computer requires a reboot to complete recent updates.</rebootRequired>
      <rebootRecommended>It is recommended to reboot your computer to apply recent updates.</rebootRecommended>
      <rebootScheduled>Your computer is scheduled to reboot at %s.</rebootScheduled>
      <rebootInProgress>Your computer will reboot in %s.</rebootInProgress>
      <rebootCancelled>The scheduled reboot has been cancelled.</rebootCancelled>
      <rebootPostponed>The reboot has been postponed for %s.</rebootPostponed>
      <rebootCompleted>Your computer has been successfully rebooted.</rebootCompleted>
      <actionRequired>Reboot is required. Click to schedule.</actionRequired>
      <actionRecommended>Reboot is recommended. Click for options.</actionRecommended>
      <actionNotRequired>No reboot is required at this time.</actionNotRequired>
      <actionNotAvailable>Reboot options are not available at this time.</actionNotAvailable>
    </messages>
    <quietHours>
      <enabled>true</enabled>
      <startTime>22:00</startTime>
      <endTime>08:00</endTime>
      <daysOfWeek>0</daysOfWeek>
      <daysOfWeek>1</daysOfWeek>
      <daysOfWeek>2</daysOfWeek>
      <daysOfWeek>3</daysOfWeek>
      <daysOfWeek>4</daysOfWeek>
      <daysOfWeek>5</daysOfWeek>
      <daysOfWeek>6</daysOfWeek>
    </quietHours>
  </notification>
  <reboot>
    <systemReboot>
      <enabled>true</enabled>
      <countdown>30s</countdown>
      <showConfirmation>true</showConfirmation>
      <confirmationMessage>The system needs to restart. Do you want to restart now?</confirmationMessage>
      <confirmationTitle>System Restart Required</confirmationTitle>
    </systemReboot>
    <timeframes>
      <timeframes>
        <min>24h</min>
        <max>48h</max>
        <reminderInterval>4h</reminderInterval>
        <deferrals>1h</deferrals>
        <deferrals>4h</deferrals>
        <deferrals>8h</deferrals>
        <deferrals>24h</deferrals>
      </timeframes>
      <timeframes>
        <min>49h</min>
        <max>72h</max>
        <reminderInterval>2h</reminderInterval>
        <deferrals>1h</deferrals>
        <deferrals>2h</deferrals>
        <deferrals>4h</deferrals>
      </timeframes>
      <timeframes>
        <min>73h</min>
        <max></max>
        <reminderInterval>30m</reminderInterval>
        <deferrals>30m</deferrals>
        <deferrals>1h</deferrals>
      </timeframes>
    </timeframes>
    <detectionMethods>
      <windowsUpdate>true</windowsUpdate>
      <sccm>true</sccm>
      <registry>true</registry>
      <pendingFileOperations>true</pendingFileOperations>
    </detectionMethods>
  </reboot>
  <database>
    <path>%PROGRAMDATA%\RebootReminder\reboot_reminder.db</path>
  </database>
  <logging>
    <path>%PROGRAMDATA%\RebootReminder\logs\reboot_reminder.log</path>
    <level>info</level>
    <maxFiles>7</maxFiles>
    <maxSize>10</maxSize>
  </logging>
  <watchdog>
    <enabled>true</enabled>
    <checkInterval>5m</checkInterval>
    <maxRestartAttempts>3</maxRestartAttempts>
    <restartDelay>30s</restartDelay>
    <serviceName>RebootReminder</serviceName>
  </watchdog>
</config>
```

## Configuration Sections

### Service Configuration

The `service` section configures the Windows service:

| Option | Description | Default |
|--------|-------------|---------|
| `name` | The name of the service | `"RebootReminder"` |
| `displayName` | The display name of the service | `"Reboot Reminder Service"` |
| `description` | The description of the service | `"Provides notifications when system reboots are necessary"` |
| `configRefreshMinutes` | How often to refresh the configuration (in minutes) | `60` |

### Notification Configuration

The `notification` section configures the notification system:

| Option | Description | Default |
|--------|-------------|---------|
| `showToast` | Whether to show toast notifications | `true` |
| `showTray` | Whether to show tray notifications | `true` |
| `showBalloon` | Whether to show balloon notifications | `false` |
| `type` | (Legacy) The type of notifications to show (`"tray"`, `"toast"`, or `"both"`) | `"both"` |

**Note:** The individual boolean flags (`showToast`, `showTray`, `showBalloon`) take precedence over the legacy `type` field. It is recommended to use these flags instead of the `type` field for more granular control over notification types.

#### Branding

The `branding` subsection configures the notification branding:

| Option | Description | Default |
|--------|-------------|---------|
| `title` | The title of the notifications | `"Reboot Reminder"` |
| `iconPath` | The path to the application icon file (icon.ico) | `"icon.ico"` |
| `company` | The company name | `"IT Department"` |

#### Messages

The `messages` subsection configures the notification messages:

| Option | Description |
|--------|-------------|
| `rebootRequired` | Message shown when a reboot is required |
| `rebootRecommended` | Message shown when a reboot is recommended |
| `rebootScheduled` | Message shown when a reboot is scheduled |
| `rebootInProgress` | Message shown when a reboot is in progress |
| `rebootCancelled` | Message shown when a reboot is cancelled |
| `rebootPostponed` | Message shown when a reboot is postponed |
| `rebootCompleted` | Message shown when a reboot is completed |
| `actionRequired` | Action message for required reboots |
| `actionRecommended` | Action message for recommended reboots |
| `actionNotRequired` | Action message when no reboot is required |
| `actionNotAvailable` | Action message when reboot options are not available |

#### Quiet Hours

The `quietHours` subsection configures quiet hours when notifications are suppressed:

| Option | Description | Default |
|--------|-------------|---------|
| `enabled` | Whether quiet hours are enabled | `true` |
| `startTime` | The start time of quiet hours (24-hour format) | `"22:00"` |
| `endTime` | The end time of quiet hours (24-hour format) | `"08:00"` |
| `daysOfWeek` | The days of the week when quiet hours are active (0 = Sunday, 6 = Saturday) | `[0, 1, 2, 3, 4, 5, 6]` |

### Reboot Configuration

The `reboot` section configures the reboot detection and behavior:

#### Timeframes

The `timeframes` array configures how often to show notifications based on how long a reboot has been required:

| Option | Description | Default |
|--------|-------------|---------|
| `min` | Minimum time since reboot required as a timespan string (e.g., "24h") | - |
| `max` | Maximum time since reboot required as a timespan string (e.g., "48h") | - |
| `minHours` | (Legacy) The minimum hours since a reboot was required | - |
| `maxHours` | (Legacy) The maximum hours since a reboot was required (optional) | - |
| `reminderInterval` | How often to show reminders as a timespan string (e.g., "4h", "30m") | - |
| `reminderIntervalHours` | (Legacy) How often to show reminders (in hours) | - |
| `reminderIntervalMinutes` | (Legacy) How often to show reminders (in minutes) | - |
| `deferrals` | Available deferral options as timespan strings (e.g., "1h", "30m") | - |

**Note:** The timespan properties (`min`, `max`, `reminderInterval`) take precedence over the legacy hour/minute properties.

#### Timespan Format

The application supports a flexible timespan format for reminder intervals and deferrals. The format is a string that consists of a number followed by a unit. The supported units are:

- `h`: hours
- `m`: minutes
- `s`: seconds

Examples:
- `"30s"`: 30 seconds
- `"30m"`: 30 minutes
- `"2h"`: 2 hours
- `"1h30m"`: 1 hour and 30 minutes
- `"1h30m15s"`: 1 hour, 30 minutes, and 15 seconds

The new `reminderInterval` property uses this format and is the recommended way to specify reminder intervals. The legacy `reminderIntervalHours` and `reminderIntervalMinutes` properties are still supported for backward compatibility.

The default configuration includes three timeframes:
1. 24-48 hours: Show reminders every 4 hours with deferrals of 1h, 4h, 8h, and 24h
2. 49-72 hours: Show reminders every 2 hours with deferrals of 1h, 2h, and 4h
3. 73+ hours: Show reminders every 30 minutes with deferrals of 30m and 1h

#### Detection Methods

The `detectionMethods` subsection configures which methods are used to detect if a reboot is required:

| Option | Description | Default |
|--------|-------------|---------|
| `windowsUpdate` | Check Windows Update for pending reboots | `true` |
| `sccm` | Check SCCM for pending reboots | `true` |
| `registry` | Check registry for pending reboots | `true` |
| `pendingFileOperations` | Check for pending file operations | `true` |

#### System Reboot Configuration

The `systemReboot` subsection configures the system reboot behavior when users initiate a restart from notifications:

| Option | Description | Default |
|--------|-------------|----------|
| `enabled` | Whether to allow users to initiate system reboots | `true` |
| `countdown` | Countdown duration before reboot (e.g., "30s", "1m") | `"30s"` |
| `countdownSeconds` | (Legacy) Countdown duration in seconds before reboot | `30` |
| `showConfirmation` | Whether to show a confirmation dialog | `true` |
| `confirmationMessage` | The confirmation message | `"The system needs to restart. Do you want to restart now?"` |
| `confirmationTitle` | The confirmation dialog title | `"System Restart Required"` |

**Note:** The `countdown` property uses the timespan format and takes precedence over the legacy `countdownSeconds` property.

### Database Configuration

The `database` section configures the database:

| Option | Description | Default |
|--------|-------------|---------|
| `path` | The path to the database file | `"rebootreminder.db"` |

### Logging Configuration

The `logging` section configures the logging system:

| Option | Description | Default |
|--------|-------------|---------|
| `path` | The path to the log file | `"logs/rebootreminder.log"` |
| `level` | The log level (`"trace"`, `"debug"`, `"info"`, `"warn"`, or `"error"`) | `"info"` |
| `maxFiles` | The maximum number of log files to keep | `7` |
| `maxSize` | The maximum size of each log file (in MB) | `10` |

## Remote Configuration

The application can also load configuration from a URL. To use a remote configuration, specify a URL as the configuration path:

```
reboot_reminder.exe --config "https://example.com/config.json"
```

The application will download the configuration file and use it. This allows for centralized management of configuration across multiple systems.

## Configuration Refresh

The application periodically refreshes its configuration based on the `configRefreshMinutes` setting. This allows you to update the configuration without restarting the service.

## Command Line Options

The application supports the following command line options:

| Option | Description |
|--------|-------------|
| `--config <FILE>` | Path to configuration file |
| `--debug` | Enable debug logging |
| `install` | Install the service |
| `uninstall` | Uninstall the service |
| `run` | Run the service |
| `check` | Check if a reboot is required |

### Installation Options

When installing the service, you can specify the following options:

| Option | Description | Default |
|--------|-------------|---------|
| `--name <NAME>` | Service name | `"RebootReminder"` |
| `--display-name <NAME>` | Service display name | `"Reboot Reminder Service"` |
| `--description <DESC>` | Service description | `"Provides notifications when system reboots are necessary"` |

Example:

```
reboot_reminder.exe install --name "CustomRebootReminder" --display-name "Custom Reboot Reminder" --description "Custom reboot reminder service"
```
