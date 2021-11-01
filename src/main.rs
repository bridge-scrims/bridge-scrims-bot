#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>{
    dotenv::dotenv()?;
    tracing_subscriber::fmt().init();
    Ok(())
}
