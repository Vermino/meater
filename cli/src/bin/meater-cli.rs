//! Meater CLI for session management
//!
//! Command-line tool for managing cooking sessions, viewing history,
//! and exporting data from the Meater probe system.

use anyhow::Result;

// Import commands module from the lib
use meater::commands;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Run CLI
    commands::run_cli().await?;

    Ok(())
}
