# Cleanup script to keep only the latest release zip file
# This script deletes all but the most recent zip file in the release-packages directory

# Ensure the script runs in the correct directory
$scriptPath = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Split-Path -Parent $scriptPath
$releaseDir = Join-Path $repoRoot "release-packages"

# Check if the release-packages directory exists
if (-not (Test-Path $releaseDir)) {
    Write-Host "Release packages directory not found: $releaseDir" -ForegroundColor Yellow
    exit
}

# Get all zip files in the release-packages directory
$zipFiles = Get-ChildItem -Path $releaseDir -Filter "*.zip" | Sort-Object LastWriteTime -Descending

# If there are more than one zip files, delete all but the most recent
if ($zipFiles.Count -gt 1) {
    Write-Host "Found $($zipFiles.Count) zip files. Keeping only the most recent one." -ForegroundColor Yellow
    
    # Keep track of the most recent file
    $latestFile = $zipFiles[0]
    Write-Host "Keeping latest file: $($latestFile.Name)" -ForegroundColor Green
    
    # Delete all other files
    for ($i = 1; $i -lt $zipFiles.Count; $i++) {
        Write-Host "Deleting old file: $($zipFiles[$i].Name)" -ForegroundColor Red
        Remove-Item $zipFiles[$i].FullName -Force
    }
    
    Write-Host "Cleanup complete. Only the latest zip file remains." -ForegroundColor Green
} else {
    Write-Host "Only one or no zip files found. No cleanup needed." -ForegroundColor Green
    if ($zipFiles.Count -eq 1) {
        Write-Host "Current zip file: $($zipFiles[0].Name)" -ForegroundColor Green
    }
}
