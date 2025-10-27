use axum::{routing::{get, post}, Router};
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::env;

mod errors;
mod handlers;
mod models;
mod services;
mod state;

use handlers::*;
use state::AppState;

#[tokio::main]
async fn main() {
    // Load environment variables
    dotenv::dotenv().ok();
    
    // Setup logging with env variable
    let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| log_level.into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state = AppState::new();
    
    // Load cache dari file backup saat startup
    if let Err(e) = state.load_cache_from_file().await {
        tracing::warn!("Failed to load cache from file: {:?}", e);
    }

    let app = Router::new()
        .route("/", get(root))
        .route("/echo", post(echo))
        .route("/data", post(get_data_by_from_date_to_date))
        .route("/data-debug", post(get_data_by_from_date_to_date_debugging))
        .route("/start-fetch", post(start_fetch_data))
        .route("/data-cached", post(get_cached_data))
        .route("/force-refresh", post(force_refresh_data))
        .route("/force-empty", post(force_empty_cache))
        .route("/login", get(get_login_status).post(post_login))
        .with_state(state)
        .layer(
            CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any)
        );

    // Get host and port from environment variables
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let bind_addr = format!("{}:{}", host, port);
    
    let listener = tokio::net::TcpListener::bind(&bind_addr).await.unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}