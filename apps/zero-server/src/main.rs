use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;

use tokio::net::TcpListener;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

fn find_interface_dir() -> Option<PathBuf> {
    ["interface/dist", "../../interface/dist"]
        .iter()
        .map(PathBuf::from)
        .find(|p| p.join("index.html").exists())
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("zero_server=debug,tower_http=debug,info")),
        )
        .init();

    let interface_dir = find_interface_dir();
    match interface_dir {
        Some(ref dir) => info!(path = %dir.display(), "serving interface"),
        None => warn!(
            "no interface dist found; API-only mode (run `cd interface && npm run dev` for UI)"
        ),
    }

    let app = zero_server::create_router(interface_dir);

    let port: u16 = std::env::var("ZERO_SERVER_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3100);
    let host: IpAddr = std::env::var("ZERO_SERVER_HOST")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(IpAddr::from([127, 0, 0, 1]));
    let addr = SocketAddr::from((host, port));

    info!(%addr, "zero server listening");
    let listener = TcpListener::bind(addr).await.expect("failed to bind");
    axum::serve(listener, app).await.expect("server error");
}
