# RebootReminder Installation Script

# Check if running as administrator
$currentPrincipal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
$isAdmin = $currentPrincipal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if (-not $isAdmin) {
    Write-Host "This script must be run as Administrator. Please restart PowerShell as Administrator and try again." -ForegroundColor Red
    exit 1
}

# Set variables
$serviceName = "RebootReminder"
$serviceDisplayName = "Reboot Reminder Service"
$serviceDescription = "Provides notifications when system reboots are necessary"
$installDir = "C:\Program Files\RebootReminder"
$executablePath = Join-Path $installDir "reboot_reminder.exe"
$configPath = Join-Path $installDir "config.json"

# Create installation directory
if (-not (Test-Path $installDir)) {
    Write-Host "Creating installation directory: $installDir" -ForegroundColor Yellow
    New-Item -Path $installDir -ItemType Directory -Force | Out-Null
}

# Copy files to installation directory
Write-Host "Copying files to installation directory..." -ForegroundColor Yellow
$sourceDir = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$releaseDir = Join-Path $sourceDir "target\release"

if (-not (Test-Path (Join-Path $releaseDir "reboot_reminder.exe"))) {
    Write-Host "Executable not found. Building project..." -ForegroundColor Yellow
    Push-Location $sourceDir
    try {
        cargo build --release
        if ($LASTEXITCODE -ne 0) {
            Write-Host "Failed to build project. Please check the output for errors." -ForegroundColor Red
            exit 1
        }
    } finally {
        Pop-Location
    }
}

Copy-Item -Path (Join-Path $releaseDir "reboot_reminder.exe") -Destination $executablePath -Force
Copy-Item -Path (Join-Path $sourceDir "config\config.json") -Destination $configPath -Force
Copy-Item -Path (Join-Path $sourceDir "resources\icons\app_icon.ico") -Destination (Join-Path $installDir "icon.ico") -Force

# Create logs directory
$logsDir = Join-Path $installDir "logs"
if (-not (Test-Path $logsDir)) {
    New-Item -Path $logsDir -ItemType Directory -Force | Out-Null
}

# Install the service
Write-Host "Installing service..." -ForegroundColor Yellow
& $executablePath install --name $serviceName --display-name $serviceDisplayName --description $serviceDescription
if ($LASTEXITCODE -ne 0) {
    Write-Host "Failed to install service. Please check the output for errors." -ForegroundColor Red
    exit 1
}

# Check if service was installed successfully
$service = Get-Service -Name $serviceName -ErrorAction SilentlyContinue
if ($null -eq $service) {
    Write-Host "Failed to install service. Please check the logs for more information." -ForegroundColor Red
    exit 1
}

# Start the service
Write-Host "Starting service..." -ForegroundColor Yellow
Start-Service -Name $serviceName

# Check if service is running
$service = Get-Service -Name $serviceName
if ($service.Status -ne "Running") {
    Write-Host "Failed to start service. Please check the logs for more information." -ForegroundColor Red
    exit 1
}

Write-Host "Installation completed successfully!" -ForegroundColor Green
Write-Host "Service Name: $serviceName" -ForegroundColor Green
Write-Host "Service Status: $($service.Status)" -ForegroundColor Green
Write-Host "Installation Directory: $installDir" -ForegroundColor Green
Write-Host "Configuration File: $configPath" -ForegroundColor Green
