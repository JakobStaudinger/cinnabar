use std::io;

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    backend::main().await
}
