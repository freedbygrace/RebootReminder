@echo off
echo Building project v2025.04.12-2130...
set PATH=%PATH%;%USERPROFILE%\.cargo\bin

echo.
echo Building debug version...
cargo build
if %ERRORLEVEL% neq 0 (
    echo Debug build failed
    exit /b 1
)
echo Debug build completed successfully

echo.
echo Building release version...
cargo build --release
if %ERRORLEVEL% neq 0 (
    echo Release build failed
    exit /b 1
)
echo Release build completed successfully

echo.
echo All builds completed successfully
echo Debug binary: %~dp0target\debug\reboot_reminder.exe
echo Release binary: %~dp0target\release\reboot_reminder.exe
echo.
