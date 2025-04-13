use anyhow::{Context, Result};
use log::{debug, LevelFilter};
use log4rs::{
    append::{
        console::ConsoleAppender,
        rolling_file::{
            policy::compound::{
                roll::fixed_window::FixedWindowRoller, trigger::size::SizeTrigger, CompoundPolicy,
            },
            RollingFileAppender,
        },
    },
    config::{Appender, Config, Root},
    encode::pattern::PatternEncoder,
    filter::threshold::ThresholdFilter,
};
use std::path::Path;

/// Initialize logging
pub fn init(debug: bool) -> Result<()> {
    // Create a console appender
    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%Y-%m-%d %H:%M:%S)} [{l}] {m}{n}",
        )))
        .build();

    // Create a default file appender
    let log_path = "logs/rebootreminder.log";
    let file_appender = create_rolling_file_appender(log_path, 10, 7)?;

    // Set log level based on debug flag
    let level = if debug {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };

    // Build the logging configuration
    let config = Config::builder()
        .appender(
            Appender::builder()
                .filter(Box::new(ThresholdFilter::new(level)))
                .build("stdout", Box::new(stdout)),
        )
        .appender(
            Appender::builder()
                .filter(Box::new(ThresholdFilter::new(level)))
                .build("file", Box::new(file_appender)),
        )
        .build(
            Root::builder()
                .appender("stdout")
                .appender("file")
                .build(level),
        )
        .context("Failed to build logging configuration")?;

    // Initialize the logger
    log4rs::init_config(config).context("Failed to initialize logger")?;

    debug!("Logging initialized with level: {:?}", level);
    Ok(())
}

/// Initialize logging with configuration
pub fn init_with_config(config_path: &Path, debug: bool) -> Result<()> {
    // Load configuration
    let config = crate::config::load(config_path).context("Failed to load configuration")?;

    // Create a console appender
    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%Y-%m-%d %H:%M:%S)} [{l}] {m}{n}",
        )))
        .build();

    // Create a file appender from configuration
    let file_appender = create_rolling_file_appender(
        &config.logging.path,
        config.logging.max_size,
        config.logging.max_files,
    )?;

    // Set log level based on configuration and debug flag
    let level = if debug {
        LevelFilter::Debug
    } else {
        match config.logging.level.to_lowercase().as_str() {
            "trace" => LevelFilter::Trace,
            "debug" => LevelFilter::Debug,
            "info" => LevelFilter::Info,
            "warn" => LevelFilter::Warn,
            "error" => LevelFilter::Error,
            _ => LevelFilter::Info,
        }
    };

    // Build the logging configuration
    let config = Config::builder()
        .appender(
            Appender::builder()
                .filter(Box::new(ThresholdFilter::new(level)))
                .build("stdout", Box::new(stdout)),
        )
        .appender(
            Appender::builder()
                .filter(Box::new(ThresholdFilter::new(level)))
                .build("file", Box::new(file_appender)),
        )
        .build(
            Root::builder()
                .appender("stdout")
                .appender("file")
                .build(level),
        )
        .context("Failed to build logging configuration")?;

    // Initialize the logger
    log4rs::init_config(config).context("Failed to initialize logger")?;

    debug!("Logging initialized with level: {:?}", level);
    Ok(())
}

/// Create a rolling file appender
fn create_rolling_file_appender(
    path: &str,
    max_size_mb: u32,
    max_files: u32,
) -> Result<RollingFileAppender> {
    // Create log directory if it doesn't exist
    if let Some(parent) = Path::new(path).parent() {
        std::fs::create_dir_all(parent).context("Failed to create log directory")?;
    }

    // Create a fixed window roller
    let roller = FixedWindowRoller::builder()
        .build(
            &format!("{}.{{}}", path),
            max_files as u32,
        )
        .context("Failed to build log roller")?;

    // Create a size trigger
    let trigger = SizeTrigger::new((max_size_mb * 1024 * 1024) as u64);

    // Create a compound policy
    let policy = CompoundPolicy::new(Box::new(trigger), Box::new(roller));

    // Create a rolling file appender
    let appender = RollingFileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%Y-%m-%d %H:%M:%S)} [{l}] {t} - {m}{n}",
        )))
        .build(path, Box::new(policy))
        .context("Failed to build file appender")?;

    Ok(appender)
}
