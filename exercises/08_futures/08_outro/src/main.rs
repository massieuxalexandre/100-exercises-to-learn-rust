#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    println!("Ticket API listening on http://127.0.0.1:3000");
    axum::serve(listener, outro_08::app()).await
}
