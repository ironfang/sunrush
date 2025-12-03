use anyhow::Result;
use clap::Parser;
use sunrush_host::{config::Config, Host};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser, Debug)]
#[command(name = "sunrush-host")]
#[command(about = "SunRush Host - Ultra-low-latency Solana transaction extraction")]
#[command(version)]
struct Args {
    /// Path to configuration file
    #[arg(short, long, default_value = "config.toml")]
    config: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Load configuration
    let config = Config::from_file(&args.config)?;

    // Initialize logging
    init_logging(&config.logging.level, &config.logging.format)?;

    tracing::info!("SunRush Host starting");
    tracing::info!("Configuration loaded from: {}", args.config);

    // Create and run host
    let host = Host::new(config)?;
    host.run().await?;

    Ok(())
}

fn init_logging(level: &str, format: &str) -> Result<()> {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level));

    match format {
        "json" => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer().json())
                .init();
        }
        "pretty" => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer().pretty())
                .init();
        }
        _ => {
            tracing_subscriber::registry()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer().compact())
                .init();
        }
    }

    Ok(())
}
