use anyhow::Result;
use clap::{Parser, Subcommand};
use meater_core::ble::{ProbeConnection, Scanner};
use std::time::Duration;

#[derive(Parser)]
#[command(name = "meater-cli")]
#[command(about = "CLI tool for interacting with MEATER smart thermometers", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan for MEATER probes
    Scan {
        /// Scan timeout in seconds
        #[arg(short, long, default_value_t = 30)]
        timeout: u64,
    },
    /// Connect to a MEATER probe and display live temperature data
    Connect {
        /// MAC address of the probe
        mac_address: String,
        /// Update interval in seconds
        #[arg(short, long, default_value_t = 5)]
        interval: u64,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Scan { timeout } => {
            println!("Scanning for MEATER probes (timeout: {}s)...", timeout);
            let scanner = Scanner::new(Duration::from_secs(timeout));
            let devices = scanner.scan().await?;

            if devices.is_empty() {
                println!("No MEATER devices found.");
            } else {
                println!("Found {} MEATER device(s):", devices.len());
                for device in devices {
                    println!("  - {}", device);
                }
            }
        }
        Commands::Connect {
            mac_address,
            interval,
        } => {
            println!("Connecting to MEATER probe at {}...", mac_address);
            let connection = ProbeConnection::new(&mac_address).await?;
            connection.connect().await?;

            if let Ok(firmware) = connection.read_firmware_version().await {
                println!("Firmware version: {}", firmware);
            }

            println!("Connected! Reading temperature data (Ctrl+C to stop)...");
            println!();

            let interval_duration = Duration::from_secs(interval);

            // Set up platform-specific signal handling
            #[cfg(unix)]
            let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;

            loop {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {
                        println!("\nShutting down gracefully...");
                        break;
                    }
                    #[cfg(unix)]
                    _ = sigterm.recv() => {
                        println!("\nReceived SIGTERM, shutting down...");
                        break;
                    }
                    result = connection.read_temperature() => {
                        match result {
                            Ok(reading) => {
                                println!(
                                    "[{}] Tip: {:.1}°C | Ambient: {:.1}°C | Battery: {}%",
                                    reading.timestamp.duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_secs(),
                                    reading.tip_temperature,
                                    reading.ambient_temperature,
                                    reading.battery_percent
                                );
                            }
                            Err(e) => {
                                eprintln!("Error reading temperature: {}", e);
                                break;
                            }
                        }
                        tokio::time::sleep(interval_duration).await;
                    }
                }
            }

            connection.disconnect().await?;
            println!("Disconnected.");
        }
    }

    Ok(())
}
