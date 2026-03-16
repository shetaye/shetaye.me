mod data;
mod markdown;
mod layout;
mod pages;

use axum::Router;
use std::net::SocketAddr;

use data::StaticFiles;
use pages::home::Site;
use pages::weblog::Weblog;
use pages::design_language::DesignLanguage;
use pages::faucet::Faucet;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .merge(Site::routes())
        .merge(StaticFiles::routes())
        .merge(Weblog::routes())
        .merge(Faucet::routes())
        .merge(DesignLanguage::routes());

    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3030".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid port number");

    let addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .expect("Invalid HOST or PORT");

    println!("Server starting on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
