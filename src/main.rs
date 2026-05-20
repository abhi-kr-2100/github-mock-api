use std::net::IpAddr;

use axum::Router;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long, default_value = "127.0.0.1")]
    host: IpAddr,
    #[arg(short, long, default_value_t = 3000)]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt().try_init()?;

    let args = Args::parse();

    let app = Router::new();

    let listener = tokio::net::TcpListener::bind((args.host, args.port))
        .await?;

    tracing::info!("Server running on http://{}", listener.local_addr()?);

    axum::serve(listener, app)
        .await?;

    Ok(())
}
