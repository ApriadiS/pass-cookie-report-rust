use crate::errors::DebugAppError;
use crate::models::{DebugResponse, Payload, Transaksi};
use chrono::{NaiveDate, NaiveDateTime};
use rand::Rng;
use serde_json::Value;
use std::{env, fs};
use tokio::time::{sleep, Duration};
use tracing::info;

pub struct TransactionService;

impl TransactionService {
    pub async fn fetch_single_page(payload: &Payload) -> Result<DebugResponse, DebugAppError> {
        info!("[DEBUG] Mengambil data untuk: {} - {}", payload.from, payload.to);

        let client = reqwest::Client::new();
        let encoded_from_date = urlencoding::encode(&payload.from);
        let encoded_to_date = urlencoding::encode(&payload.to);

        let base_url = env::var("API_BASE_URL")
            .unwrap_or_else(|_| "https://example.com".to_string());
        let store_id = env::var("STORE_ID")
            .unwrap_or_else(|_| "1".to_string());
        let timestamp = env::var("API_TIMESTAMP")
            .unwrap_or_else(|_| "1234567890".to_string());

        let response = client
            .get(format!("{}/transaction-report/datatables?draw=1&start=0&length=10&search%5Bvalue%5D&search%5Bregex%5D=false&pelanggan_id&id&tglAwal={}&tglAkhir={}&lunas&serial&nama_barang&store_id={}&_={}", base_url, encoded_from_date, encoded_to_date, store_id, timestamp))
            .header("Accept", "*/*")
            .header("Cookie", &payload.cookie)
            .send()
            .await?;

        let body_text = response.text().await?;

        let filename = "debug_response.txt";
        match fs::write(filename, &body_text) {
            Ok(_) => info!("[DEBUG] Berhasil menyimpan respons mentah ke '{}'", filename),
            Err(e) => info!("[DEBUG] GAGAL menyimpan respons ke file: {}", e),
        }

        let data_mentah: Value = serde_json::from_str(&body_text)?;
        let mut hasil_bersih: Vec<Transaksi> = Vec::new();

        if let Some(records) = data_mentah["data"].as_array() {
            for record in records {
                hasil_bersih.push(Self::parse_transaction_record(record));
            }
        }

        let total_transaksi = data_mentah["totalRow"].as_u64().unwrap_or(0) as usize;

        info!(
            "[DEBUG] Berhasil memproses {} transaksi bersih. Total dari server: {}",
            hasil_bersih.len(),
            total_transaksi
        );

        Ok(DebugResponse {
            total_transaksi,
            data: hasil_bersih,
        })
    }

    pub async fn fetch_all_pages(payload: &Payload) -> Result<DebugResponse, DebugAppError> {
        info!("[PAGINATION] Mengambil semua data untuk: {} - {}", payload.from, payload.to);

        let client = reqwest::Client::new();
        let encoded_from_date = urlencoding::encode(&payload.from);
        let encoded_to_date = urlencoding::encode(&payload.to);

        let mut all_transaksi: Vec<Transaksi> = Vec::new();
        let mut draw = 1;
        let mut start = 0;
        let length = 10;
        let mut total_transaksi = 0;

        loop {
            info!("[PAGINATION] Request #{} - start: {}, length: {}", draw, start, length);

            let base_url = env::var("API_BASE_URL")
                .unwrap_or_else(|_| "https://example.com".to_string());
            let store_id = env::var("STORE_ID")
                .unwrap_or_else(|_| "1".to_string());
            let timestamp = env::var("API_TIMESTAMP")
                .unwrap_or_else(|_| "1234567890".to_string());

            let response = client
                .get(format!("{}/transaction-report/datatables?draw={}&start={}&length={}&search%5Bvalue%5D&search%5Bregex%5D=false&pelanggan_id&id&tglAwal={}&tglAkhir={}&lunas&serial&nama_barang&store_id={}&_={}", base_url, draw, start, length, encoded_from_date, encoded_to_date, store_id, timestamp))
                .header("Accept", "*/*")
                .header("Cookie", payload.cookie.clone())
                .send()
                .await?;

            let body_text = response.text().await?;
            let data_mentah: Value = serde_json::from_str(&body_text)?;

            if draw == 1 {
                total_transaksi = data_mentah["totalRow"].as_u64().unwrap_or(0) as usize;
                info!("[PAGINATION] Total transaksi dari server: {}", total_transaksi);
            }

            if let Some(records) = data_mentah["data"].as_array() {
                for record in records {
                    all_transaksi.push(Self::parse_transaction_record(record));
                }
            }

            info!("[PAGINATION] Collected {} transaksi so far", all_transaksi.len());

            if all_transaksi.len() >= total_transaksi || start + length >= total_transaksi {
                break;
            }

            let delay_ms = rand::thread_rng().gen_range(250..=750);
            info!("[PAGINATION] Waiting {}ms before next request...", delay_ms);
            sleep(Duration::from_millis(delay_ms)).await;

            draw += 1;
            start += length;
        }

        info!("[PAGINATION] Selesai! Total {} transaksi dikumpulkan", all_transaksi.len());

        Ok(DebugResponse {
            total_transaksi,
            data: all_transaksi,
        })
    }

    fn parse_transaction_record(record: &Value) -> Transaksi {
        let tanggal_transaksi = record["tglTrans"]
            .as_str()
            .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

        let waktu_transaksi = record["date"]
            .as_str()
            .and_then(|s| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").ok());

        let keterangan = record["xx_keterangan"].as_str().unwrap_or("").to_string();

        let total_tagihan = record["total_tagihan"].as_u64().unwrap_or(0);

        let no_nota = record["xx_no_nota_text"].as_str().unwrap_or("").to_string();

        Transaksi {
            tanggal_transaksi,
            waktu_transaksi,
            keterangan,
            total_tagihan,
            no_nota,
        }
    }
}