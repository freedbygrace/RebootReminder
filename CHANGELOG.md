# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [v2025.4.12-2300] - 2025-04-12

### Fixed
- Fixed service installation to use LocalSystem account without password
- Added automatic restart recovery options for the service
- Updated documentation to reflect service installation changes

## [v2025.04.12-2130] - 2025-04-12

### Added
- Enhanced notification interaction logging with detailed who, what, when, where, why information
- Improved deferral tracking and logging
- Added build support for both debug and release versions
- Added system reboot functionality with confirmation dialog and customizable countdown
- Added ability for users to initiate system restart directly from notifications
- Added configuration options for system reboot behavior

### Changed
- Replaced all registry operations with native Windows API calls
- Removed dependency on external command-line tools for registry operations
- Improved performance and reliability of registry checks

## [2025.4.12-9000] - 2025-04-12

### Added
- Initial project structure
- Windows service implementation
- User impersonation for interactive sessions
- Configuration system with JSON and XML support
- Database integration for state persistence
- Reboot detection with multiple methods
- Notification system with tray and toast support
- Logging system with rotation
- Customizable reboot reminder timeframes
- Deferral options based on reboot timespan
- Support for timespan format in reminder intervals and deferrals (e.g., "30m", "2h")
- Enhanced logging for database operations
- Detailed tracking of how long a reboot has been required
- Support for command-line configuration file path parameter
- Improved logging for configuration loading and reboot detection
- Quiet hours support
- MSI installer generation
- Support for Windows environment variables in configuration paths
- Optional watchdog service for improved reliability
- Native Windows API usage for event log access
- Updated Windows crate to v0.61.1

## [0.1.0] - 2023-10-12

### Added
- Initial release
- Basic project structure
- Windows service implementation
- Configuration system
- Reboot detection
- Notification system
- Logging system
