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

# Ensure the version format is correct (no leading zeros in month)
$versionPattern = 'v(\d+)\.(\d+)\.(\d+)-(\d+)'
$versionMatch = [regex]::Match($version, $versionPattern)
if ($versionMatch.Success) {
    $year = $versionMatch.Groups[1].Value
    $month = $versionMatch.Groups[2].Value
    $day = $versionMatch.Groups[3].Value
    $time = $versionMatch.Groups[4].Value

    # Remove leading zero from month if present
    $month = [int]$month

    # Reconstruct version
    $version = "v${year}.${month}.${day}-${time}"
    $versionWithoutV = "${year}.${month}.${day}-${time}"
    Write-Host "Normalized version: $version" -ForegroundColor Green
}

# Ensure version starts with 'v'
if (-not $version.StartsWith("v")) {
    $version = "v" + $version
}

Write-Host "Creating release package for version: $version" -ForegroundColor Green

# Check if Cargo.toml version matches
$cargoTomlPath = Join-Path $repoRoot "Cargo.toml"
$cargoContent = Get-Content -Path $cargoTomlPath -Raw
$cargoVersionMatch = [regex]::Match($cargoContent, 'version\s*=\s*"([\d\.\-]+)"')

if ($cargoVersionMatch.Success) {
    $cargoVersion = $cargoVersionMatch.Groups[1].Value
    $versionWithoutV = $version.TrimStart('v')

    if ($cargoVersion -ne $versionWithoutV) {
        Write-Host "Warning: Version mismatch detected!" -ForegroundColor Yellow
        Write-Host "  README.md version: $version" -ForegroundColor Yellow
        Write-Host "  Cargo.toml version: $cargoVersion" -ForegroundColor Yellow

        $updateCargo = Read-Host "Do you want to update Cargo.toml version to $versionWithoutV? (y/n)"
        if ($updateCargo -eq 'y') {
            $cargoContent = $cargoContent -replace 'version\s*=\s*"[\d\.\-]+"', "version = `"$versionWithoutV`""
            Set-Content -Path $cargoTomlPath -Value $cargoContent
            Write-Host "Updated Cargo.toml version to $versionWithoutV" -ForegroundColor Green

            # Rebuild the project to apply the new version
            Write-Host "Rebuilding project with updated version..." -ForegroundColor Yellow
            & (Join-Path $repoRoot "build.bat")

            if ($LASTEXITCODE -ne 0) {
                Write-Host "Rebuild failed after version update. Please check the output for errors." -ForegroundColor Red
                exit 1
            }
        } else {
            Write-Host "Continuing with version mismatch. This may cause issues." -ForegroundColor Yellow
        }
    } else {
        Write-Host "Version check passed: README.md and Cargo.toml versions match." -ForegroundColor Green
    }
} else {
    Write-Host "Warning: Could not find version in Cargo.toml" -ForegroundColor Yellow
}

# Build the project
Write-Host "Building project..." -ForegroundColor Yellow
& (Join-Path $repoRoot "build.bat")

if ($LASTEXITCODE -ne 0) {
    Write-Host "Build failed. Aborting release creation." -ForegroundColor Red
    exit 1
}

# Create release directory if it doesn't exist
$releaseDir = Join-Path $repoRoot "release"
if (-not (Test-Path $releaseDir)) {
    New-Item -ItemType Directory -Path $releaseDir -Force | Out-Null
    Write-Host "Created release directory" -ForegroundColor Yellow
}

# Copy files to release directory
Write-Host "Copying files to release directory..." -ForegroundColor Yellow
Copy-Item (Join-Path $repoRoot "target\release\reboot_reminder.exe") $releaseDir -Force
Copy-Item (Join-Path $repoRoot "README.md") $releaseDir -Force
Copy-Item (Join-Path $repoRoot "LICENSE") $releaseDir -Force
Copy-Item (Join-Path $repoRoot "config\*.json") $releaseDir -Force
Copy-Item (Join-Path $repoRoot "config\*.xml") $releaseDir -Force
Copy-Item (Join-Path $repoRoot "docs\CONFIGURATION.md") $releaseDir -Force
Copy-Item (Join-Path $repoRoot "resources\icons\icon.ico") $releaseDir -Force

# Create release-packages directory if it doesn't exist
$releasePackagesDir = Join-Path $repoRoot "release-packages"
if (-not (Test-Path $releasePackagesDir)) {
    New-Item -ItemType Directory -Path $releasePackagesDir -Force | Out-Null
    Write-Host "Created release-packages directory" -ForegroundColor Yellow
}

# Create the release package
$zipFileName = "RebootReminder-$version.zip"
$zipFilePath = Join-Path $releasePackagesDir $zipFileName

Write-Host "Creating release package: $zipFileName" -ForegroundColor Yellow
Compress-Archive -Path "$releaseDir\*" -DestinationPath $zipFilePath -Force

if (-not (Test-Path $zipFilePath)) {
    Write-Host "Failed to create release package." -ForegroundColor Red
    exit 1
}

# Copy individual files to release-packages directory for separate upload
Write-Host "Copying individual files for separate upload..." -ForegroundColor Yellow
Copy-Item (Join-Path $repoRoot "target\release\reboot_reminder.exe") $releasePackagesDir -Force
Copy-Item (Join-Path $repoRoot "config.example.json") (Join-Path $releasePackagesDir "config.sample.json") -Force
Copy-Item (Join-Path $repoRoot "config.example.xml") (Join-Path $releasePackagesDir "config.sample.xml") -Force
Copy-Item (Join-Path $repoRoot "resources\icons\icon.ico") $releasePackagesDir -Force

Write-Host "Release package created successfully: $zipFilePath" -ForegroundColor Green

# Clean up old release packages
Write-Host "Cleaning up old release packages..." -ForegroundColor Yellow
& (Join-Path $scriptPath "cleanup-releases.ps1")

Write-Host "Release creation complete!" -ForegroundColor Green
Write-Host "You can now upload the release package to GitHub: $zipFilePath" -ForegroundColor Green
