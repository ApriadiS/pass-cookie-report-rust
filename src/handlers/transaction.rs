#![allow(dead_code)]
#![allow(unused_imports)]


use axum::Json;
use crate::errors::{AppError, DebugAppError};
// DatatableResponse tidak digunakan di handler ini, comment out
// use crate::model::DatatableResponse;
use crate::models::{DebugResponse, Payload};
use crate::services::TransactionService;
use tracing::info;

pub async fn get_data_by_from_date_to_date(
    Json(payload): Json<Payload>,
) -> Result<Json<serde_json::Value>, AppError> {
    info!("Received payload: from={}, to={}, cookie={}", payload.from, payload.to, payload.cookie);

    let client = reqwest::Client::new();
    let encoded_from_date = urlencoding::encode(&payload.from);
    let encoded_to_date = urlencoding::encode(&payload.to);

    info!("Encoded dates: from={}, to={}", encoded_from_date, encoded_to_date);

    let response = client
        .get(format!("https://kasir.doran.id/transaction-report/datatables?draw=1&start=0&length=10&search%5Bvalue%5D&search%5Bregex%5D=false&pelanggan_id&id&tglAwal={}&tglAkhir={}&lunas&serial&nama_barang&store_id=263&_=1760460287629", encoded_from_date, encoded_to_date))
        .header("Accept", "*/*")
        .header("Cookie", payload.cookie)
        .send()
        .await?;

    info!("HTTP request sent, awaiting response...");
    let data = response.json::<serde_json::Value>().await?;
    info!("Successfully parsed response");

    Ok(Json(data))
}

pub async fn get_data_by_from_date_to_date_debugging(
    Json(payload): Json<Payload>,
) -> Result<Json<DebugResponse>, DebugAppError> {
    let response = TransactionService::fetch_single_page(&payload).await?;
    Ok(Json(response))
}