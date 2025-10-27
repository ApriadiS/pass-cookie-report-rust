use axum::{http::StatusCode, Json, extract::State, debug_handler};
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::state::AppState;
use crate::services::AuthService;
use tracing::{info, error};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[debug_handler]
pub async fn get_login_status(
    State(state): State<AppState>,
) -> (StatusCode, Json<serde_json::Value>) {
    let is_unauthorized = state.is_unauthorized().await;
    
    if is_unauthorized {
        (StatusCode::UNAUTHORIZED, Json(json!({
            "status": "unauthorized",
            "message": "Session expired or invalid cookie"
        })))
    } else {
        (StatusCode::OK, Json(json!({
            "status": "authorized",
            "message": "Session is valid"
        })))
    }
}

#[debug_handler]
pub async fn post_login(
    State(state): State<AppState>,
    Json(login_req): Json<LoginRequest>,
) -> (StatusCode, Json<serde_json::Value>) {
    // If already authorized, return success
    if !state.is_unauthorized().await {
        return (StatusCode::OK, Json(json!({
            "status": "authorized",
            "message": "Already logged in"
        })));
    }

    // Perform login with credentials from frontend
    match AuthService::perform_login(&login_req.username, &login_req.password).await {
        Ok(cookie) => {
            // Set authorized state
            state.set_unauthorized(false).await;
            
            info!("[AUTH] Login successful for user: {}", login_req.username);
            (StatusCode::OK, Json(json!({
                "status": "authorized",
                "message": "Login successful",
                "cookie": cookie
            })))
        }
        Err(e) => {
            error!("[AUTH] Login failed: {:?}", e);
            (StatusCode::UNAUTHORIZED, Json(json!({
                "status": "unauthorized",
                "message": format!("Login failed: {:?}", e)
            })))
        }
    }
}