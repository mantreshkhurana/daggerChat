use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod app;
mod blockchain;
mod chat;
mod config;
mod crypto;
mod errors;
mod storage;
mod tui;

#[derive(Parser, Debug)]
#[command(name = "dagger-chat")]
#[command(version, about = "Blockchain-based secure chat application")]
struct Args {
    /// RPC endpoint URL (overrides .env)
    #[arg(short, long)]
    rpc_url: Option<String>,

    /// Private key hex string or path to keystore
    #[arg(short, long)]
    wallet: Option<String>,

    /// Contract address
    #[arg(short, long)]
    contract: Option<String>,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Load .env file
    dotenvy::dotenv().ok();

    // Initialize logging
    let log_level = if args.debug { "debug" } else { "info" };
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(log_level)),
        )
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();

    tracing::info!("Starting daggerChat...");

    // Load configuration
    let config = config::Config::load(args.rpc_url, args.wallet, args.contract)?;

    // Initialize and run application
    let mut app = app::App::new(config).await?;
    tui::run(&mut app).await.map_err(|e| anyhow::anyhow!("{}", e))
}
