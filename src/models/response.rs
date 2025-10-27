#[allow(dead_code)]

use serde::Serialize;
use super::Transaksi;

#[derive(Serialize, Debug, Clone)]
pub struct TransaksiResponse {
    pub total_transaksi: usize,
    pub data: Vec<Transaksi>,
}

#[derive(Serialize, Debug, Clone)]
pub struct DebugResponse {
    pub total_transaksi: usize,
    pub data: Vec<Transaksi>,
}

#[derive(Serialize)]
pub struct Health {
    pub status: &'static str,
}

#[derive(Serialize)]
pub struct EchoResponse {
    pub echoed: String,
}

#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub status: String,
    pub job_id: Option<String>,
    pub data: Option<T>,
    pub message: Option<String>,
}

#[derive(Serialize)]
pub struct CachedDataResponse {
    pub status: String,
    pub job_id: String,
    pub data: TransaksiResponse,
    pub message: Option<String>,
}