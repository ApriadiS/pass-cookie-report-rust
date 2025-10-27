use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transaksi {
    pub tanggal_transaksi: String,
    pub waktu_transaksi: String,
    pub keterangan: String,
    pub total_tagihan: i64,
    pub no_nota: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Payload {
    pub from: String,
    pub to: String,
    pub cookie: String,
}