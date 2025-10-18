use serde::Serialize;
use super::Transaksi;

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