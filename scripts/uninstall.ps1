# RebootReminder Uninstallation Script

# Check if running as administrator
$currentPrincipal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
$isAdmin = $currentPrincipal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if (-not $isAdmin) {
    Write-Host "This script must be run as Administrator. Please restart PowerShell as Administrator and try again." -ForegroundColor Red
    exit 1
}

# Set variables
$serviceName = "RebootReminder"
$installDir = "C:\Program Files\RebootReminder"
$executablePath = Join-Path $installDir "reboot_reminder.exe"

# Check if service exists
$service = Get-Service -Name $serviceName -ErrorAction SilentlyContinue
if ($null -ne $service) {
    # Stop the service
    Write-Host "Stopping service..." -ForegroundColor Yellow
    Stop-Service -Name $serviceName -Force

    # Uninstall the service
    Write-Host "Uninstalling service..." -ForegroundColor Yellow
    if (Test-Path $executablePath) {
        & $executablePath uninstall
    } else {
        # If executable is not found, use SC to delete the service
        sc.exe delete $serviceName
    }

    # Wait for service to be removed
    Start-Sleep -Seconds 2
}

# Check if service was uninstalled successfully
$service = Get-Service -Name $serviceName -ErrorAction SilentlyContinue
if ($null -ne $service) {
    Write-Host "Failed to uninstall service. Please check the logs for more information." -ForegroundColor Red
    exit 1
}

# Remove installation directory
if (Test-Path $installDir) {
    Write-Host "Removing installation directory: $installDir" -ForegroundColor Yellow
    Remove-Item -Path $installDir -Recurse -Force
}

Write-Host "Uninstallation completed successfully!" -ForegroundColor Green
