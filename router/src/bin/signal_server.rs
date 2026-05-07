use anyhow::Result;
use router::signaling::server;

#[tokio::main]
async fn main() -> Result<()> {
    server::run_server("127.0.0.1:9000").await
}