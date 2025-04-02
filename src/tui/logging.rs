use std::env;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

pub fn setup_logging() -> anyhow::Result<()> {
    // Create .hal directory in the working directory if it doesn't exist
    let work_dir = env::current_dir()?;
    let log_dir = work_dir.join(".hal");
    std::fs::create_dir_all(&log_dir)?;

    // Set up file appender
    let file_appender = RollingFileAppender::new(Rotation::NEVER, log_dir, "tui.log");

    // Create a logging layer that writes to the file
    let file_layer = fmt::layer()
        .with_writer(file_appender)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true);

    // Create a registry with the file layer
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(file_layer)
        .init();

    Ok(())
}
