#![allow(dead_code)]
#![allow(unused_imports)]

use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Deserialize;
use crate::models::{Health, EchoResponse};

#[derive(Deserialize)]
pub struct EchoPayload {
    pub message: String,
}

pub async fn root() -> impl IntoResponse {
    Json(Health { status: "ok" })
}

pub async fn echo(Json(payload): Json<EchoPayload>) -> impl IntoResponse {
    (StatusCode::CREATED, Json(EchoResponse { echoed: payload.message }))
}