use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use serde_json::json;
use tracing::info;

#[derive(Debug)]
pub struct AppError(pub reqwest::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let error_message = self.0.to_string();
        info!("Error occurred: {}", error_message);
        let body = Json(json!({
            "error": "Failed to process request",
            "details": error_message,
        }));
        (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
    }
}

impl From<reqwest::Error> for AppError {
    fn from(error: reqwest::Error) -> Self {
        AppError(error)
    }
}

#[derive(Debug)]
pub enum DebugAppError {
    Reqwest(reqwest::Error),
    Serde(serde_json::Error),
    DateParse(String),
    Serialization(String),
    FileWrite(String),
}

impl IntoResponse for DebugAppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            DebugAppError::Reqwest(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Reqwest error: {}", e),
            ),
            DebugAppError::Serde(e) => (
                StatusCode::BAD_GATEWAY,
                format!("Failed to parse response from upstream: {}", e),
            ),
            DebugAppError::DateParse(e) => (
                StatusCode::BAD_REQUEST,
                format!("Date parse error: {}", e),
            ),
            DebugAppError::Serialization(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Serialization error: {}", e),
            ),
            DebugAppError::FileWrite(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("File write error: {}", e),
            ),
        };

        info!("[DEBUG] Error occurred: {}", error_message);
        let body = Json(json!({
            "error": "An error occurred in the debugging function",
            "details": error_message,
        }));
        (status, body).into_response()
    }
}

impl From<reqwest::Error> for DebugAppError {
    fn from(error: reqwest::Error) -> Self {
        DebugAppError::Reqwest(error)
    }
}

impl From<serde_json::Error> for DebugAppError {
    fn from(error: serde_json::Error) -> Self {
        DebugAppError::Serde(error)
    }
}