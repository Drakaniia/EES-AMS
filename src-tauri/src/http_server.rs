// Simple HTTP Server to run alongside the Tauri application
use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Router,
};

// Simple state to hold a server health check
#[derive(Clone)]
pub struct ServerState {
    pub health: String,
}

pub async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "Server is running")
}

// Main function to build the HTTP server
pub fn create_http_server() -> Router {
    Router::new()
        .route("/health", get(health_check))
}

// Function to start the HTTP server
pub async fn start_http_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = create_http_server();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("HTTP server listening on http://0.0.0.0:3000");
    
    axum::serve(listener, app).await?;
    
    Ok(())
}