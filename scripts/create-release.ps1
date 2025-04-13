# Script to build and package a release
# This script builds the project, creates a release package, and cleans up old packages

param(
    [string]$version = ""
)

# Ensure the script runs in the correct directory
$scriptPath = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Split-Path -Parent $scriptPath
Set-Location $repoRoot

# If no version is provided, try to extract it from README.md
if ([string]::IsNullOrEmpty($version)) {
    $readmeContent = Get-Content -Path (Join-Path $repoRoot "README.md") -Raw
    $versionMatch = [regex]::Match($readmeContent, '# Reboot Reminder (v[\d\.\-]+)')
    
    if ($versionMatch.Success) {
        $version = $versionMatch.Groups[1].Value
    } else {
        Write-Host "Could not extract version from README.md. Please provide a version parameter." -ForegroundColor Red
        exit 1
    }
}

# Ensure version starts with 'v'
if (-not $version.StartsWith("v")) {
    $version = "v" + $version
}

Write-Host "Creating release package for version: $version" -ForegroundColor Green

# Build the project
Write-Host "Building project..." -ForegroundColor Yellow
& (Join-Path $repoRoot "build.bat")

if ($LASTEXITCODE -ne 0) {
    Write-Host "Build failed. Aborting release creation." -ForegroundColor Red
    exit 1
}

# Create release-packages directory if it doesn't exist
$releasePackagesDir = Join-Path $repoRoot "release-packages"
if (-not (Test-Path $releasePackagesDir)) {
    New-Item -ItemType Directory -Path $releasePackagesDir -Force | Out-Null
    Write-Host "Created release-packages directory" -ForegroundColor Yellow
}

# Create the release package
$zipFileName = "RebootReminder-$version.zip"
$zipFilePath = Join-Path $releasePackagesDir $zipFileName
$releaseDir = Join-Path $repoRoot "release"

Write-Host "Creating release package: $zipFileName" -ForegroundColor Yellow
Compress-Archive -Path "$releaseDir\*" -DestinationPath $zipFilePath -Force

if (-not (Test-Path $zipFilePath)) {
    Write-Host "Failed to create release package." -ForegroundColor Red
    exit 1
}

Write-Host "Release package created successfully: $zipFilePath" -ForegroundColor Green

# Clean up old release packages
Write-Host "Cleaning up old release packages..." -ForegroundColor Yellow
& (Join-Path $scriptPath "cleanup-releases.ps1")

Write-Host "Release creation complete!" -ForegroundColor Green
Write-Host "You can now upload the release package to GitHub: $zipFilePath" -ForegroundColor Green
