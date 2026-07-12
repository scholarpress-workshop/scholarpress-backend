#[tokio::main]
async fn main() {
    if let Err(e) = publish_service::run().await {
        eprintln!("Fatal error: {}", e);
        std::process::exit(1);
    }
}
