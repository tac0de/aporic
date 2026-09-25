use std::error::Error;

use aporic::{AporicMcp, Hub, default_database_path};
use rmcp::{ServiceExt, transport::stdio};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [command] if command == "doctor" => doctor(),
        [mcp, serve, transport] if mcp == "mcp" && serve == "serve" && transport == "--stdio" => {
            serve_stdio().await
        }
        _ => {
            eprintln!("usage: aporic doctor | aporic mcp serve --stdio");
            std::process::exit(2);
        }
    }
}

fn doctor() -> Result<(), Box<dyn Error>> {
    let database = default_database_path().map_err(std::io::Error::other)?;
    let hub = Hub::open(&database)?;
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({
            "ok": true,
            "kernel_sha256": aporic::kernel::digest(),
            "database": hub.database_path(),
            "event_count": hub.event_count()?
        }))?
    );
    Ok(())
}

async fn serve_stdio() -> Result<(), Box<dyn Error>> {
    let database = default_database_path().map_err(std::io::Error::other)?;
    let hub = Hub::open(database)?;
    let service = AporicMcp::new(hub).serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
