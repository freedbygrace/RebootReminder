# Installation Guide

This guide provides instructions for installing the RebootReminder service on Windows systems.

## Prerequisites

- Windows operating system (Windows 10 or later recommended)
- Administrative privileges
- Rust toolchain (if building from source)
- WiX Toolset (if building MSI installer from source)

## Installation Methods

### Method 1: Using the MSI Installer (Recommended)

1. Download the latest MSI installer from the [Releases](https://github.com/freedbygrace/RebootReminder/releases) page.
2. Run the installer with administrative privileges.
3. Follow the installation wizard.
4. The service will be installed and started automatically.

### Method 2: Using Command Line

1. Download the latest release binary from the [Releases](https://github.com/freedbygrace/RebootReminder/releases) page.
2. Extract the ZIP file to a directory of your choice (e.g., `C:\Program Files\RebootReminder`).
3. Open a Command Prompt or PowerShell window with administrative privileges.
4. Navigate to the directory where you extracted the files.
5. Run the following command to install the service:

```powershell
.\reboot_reminder.exe install --name "RebootReminder" --display-name "Reboot Reminder Service" --description "Provides notifications when system reboots are necessary"
```

6. Start the service:

```powershell
Start-Service -Name "RebootReminder"
```

### Method 3: Building from Source

1. Clone the repository:

```bash
git clone https://github.com/freedbygrace/RebootReminder.git
cd RebootReminder
```

2. Build the project:

```bash
cargo build --release
```

3. Install the service:

```powershell
.\target\release\reboot_reminder.exe install --name "RebootReminder" --display-name "Reboot Reminder Service" --description "Provides notifications when system reboots are necessary"
```

4. Start the service:

```powershell
Start-Service -Name "RebootReminder"
```

## Configuration

After installation, you can configure the service by editing the configuration file:

- If installed using the MSI installer: `C:\Program Files\RebootReminder\config.json`
- If installed manually: In the directory where you extracted the files

See the [Configuration Guide](CONFIGURATION.md) for details on customizing the application.

## Verification

To verify that the service is installed and running:

1. Open PowerShell or Command Prompt with administrative privileges.
2. Run the following command:

```powershell
Get-Service -Name "RebootReminder"
```

The service status should be "Running".

## Uninstallation

### Method 1: Using the MSI Installer

1. Open "Add or Remove Programs" in Windows Settings.
2. Find "Reboot Reminder" in the list of installed programs.
3. Click "Uninstall" and follow the prompts.

### Method 2: Using Command Line

1. Open a Command Prompt or PowerShell window with administrative privileges.
2. Stop the service:

```powershell
Stop-Service -Name "RebootReminder"
```

3. Uninstall the service:

```powershell
.\reboot_reminder.exe uninstall
```

4. Delete the installation directory:

```powershell
Remove-Item -Path "C:\Program Files\RebootReminder" -Recurse -Force
```

## Troubleshooting

If you encounter issues during installation:

1. Check the Windows Event Log for service-related errors:
   - Open Event Viewer
   - Navigate to Windows Logs > Application
   - Look for events with source "RebootReminder"

2. Check the service log file:
   - Open `C:\Program Files\RebootReminder\logs\rebootreminder.log`

3. Verify that the service account has the necessary permissions:
   - The service runs as SYSTEM by default
   - Ensure that the SYSTEM account has read/write access to the installation directory

4. If the service fails to start, try running the executable directly with debug logging:

```powershell
.\reboot_reminder.exe --debug
```

## Silent Installation

To perform a silent installation using the MSI installer:

```powershell
msiexec /i RebootReminder.msi /quiet
```

You can also specify custom configuration parameters:

```powershell
msiexec /i RebootReminder.msi /quiet CONFIG_URL="https://example.com/config.json"
```
