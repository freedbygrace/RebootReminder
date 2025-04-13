# Build MSI Installer Script

# Check if running as administrator
$currentPrincipal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
$isAdmin = $currentPrincipal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if (-not $isAdmin) {
    Write-Host "This script must be run as Administrator. Please restart PowerShell as Administrator and try again." -ForegroundColor Red
    exit 1
}

# Set variables
$projectDir = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$releaseDir = Join-Path $projectDir "target\release"
$wixDir = Join-Path $projectDir "wix"
$outputDir = Join-Path $projectDir "installer"

# Create output directory if it doesn't exist
if (-not (Test-Path $outputDir)) {
    New-Item -Path $outputDir -ItemType Directory -Force | Out-Null
}

# Build the project in release mode
Write-Host "Building project in release mode..." -ForegroundColor Yellow
Push-Location $projectDir
try {
    cargo build --release
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Failed to build project. Please check the output for errors." -ForegroundColor Red
        exit 1
    }
} finally {
    Pop-Location
}

# Check if cargo-wix is installed
$cargoWixInstalled = $null -ne (Get-Command cargo-wix -ErrorAction SilentlyContinue)
if (-not $cargoWixInstalled) {
    Write-Host "cargo-wix is not installed. Installing..." -ForegroundColor Yellow
    cargo install cargo-wix
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Failed to install cargo-wix. Please check the output for errors." -ForegroundColor Red
        exit 1
    }
}

# Check if WiX Toolset is installed
$wixInstalled = $null -ne (Get-Command candle.exe -ErrorAction SilentlyContinue)
if (-not $wixInstalled) {
    Write-Host "WiX Toolset is not installed. Please install it from https://wixtoolset.org/releases/" -ForegroundColor Red
    exit 1
}

# Build the MSI installer
Write-Host "Building MSI installer..." -ForegroundColor Yellow
Push-Location $projectDir
try {
    cargo wix --no-build --nocapture
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Failed to build MSI installer. Please check the output for errors." -ForegroundColor Red
        exit 1
    }
} finally {
    Pop-Location
}

# Copy the MSI installer to the output directory
$msiFile = Get-ChildItem -Path $projectDir -Filter "*.msi" | Select-Object -First 1
if ($null -eq $msiFile) {
    Write-Host "Failed to find MSI installer. Please check the output for errors." -ForegroundColor Red
    exit 1
}

$outputFile = Join-Path $outputDir "RebootReminder.msi"
Copy-Item -Path $msiFile.FullName -Destination $outputFile -Force

# Clean up
Remove-Item -Path $msiFile.FullName -Force

Write-Host "MSI installer built successfully!" -ForegroundColor Green
Write-Host "Installer: $outputFile" -ForegroundColor Green
