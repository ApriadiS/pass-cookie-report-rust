use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transaksi {
    pub tanggal_transaksi: Option<NaiveDate>,
    pub waktu_transaksi: Option<NaiveDateTime>,
    pub keterangan: String,
    pub total_tagihan: u64,
    pub no_nota: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Payload {
    pub from: String,
    pub to: String,
    pub cookie: String,
}