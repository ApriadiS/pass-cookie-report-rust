use crate::errors::DebugAppError;
use crate::models::{DebugResponse, Payload, Transaksi};
use crate::services::DateService;
use rand::Rng;
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

use std::{env, fs};
use tokio::time::{sleep, Duration};
use tracing::{info, warn};

pub struct TransactionService;

impl TransactionService {
    pub async fn fetch_single_page(payload: &Payload) -> Result<DebugResponse, DebugAppError> {
        info!("[DEBUG] Mengambil data untuk: {} - {}", payload.from, payload.to);

        let client = reqwest::Client::new();
        
        // Normalize dates to YYYY-MM-DD format for API
        let normalized_from = DateService::normalize_date_for_api(&payload.from)
            .map_err(|_| DebugAppError::DateParse(format!("Invalid from date: {}", payload.from)))?;
        let normalized_to = DateService::normalize_date_for_api(&payload.to)
            .map_err(|_| DebugAppError::DateParse(format!("Invalid to date: {}", payload.to)))?;
            
        let encoded_from_date = urlencoding::encode(&normalized_from);
        let encoded_to_date = urlencoding::encode(&normalized_to);

        let base_url = env::var("API_BASE_URL")
            .unwrap_or_else(|_| "https://example.com".to_string());
        let store_id = env::var("STORE_ID")
            .unwrap_or_else(|_| "1".to_string());
        let timestamp = env::var("API_TIMESTAMP")
            .unwrap_or_else(|_| "1234567890".to_string());

        let url = format!("{}/transaction-report/datatables?draw=1&start=0&length=10&search%5Bvalue%5D&search%5Bregex%5D=false&pelanggan_id&id&tglAwal={}&tglAkhir={}&lunas&serial&nama_barang&store_id={}&_={}", base_url, encoded_from_date, encoded_to_date, store_id, timestamp);

        info!("[DEBUG] GET URL: {}", &url);

        let response = client
            .get(&url)
            .header("Accept", "*/*")
            .header("Cookie", &payload.cookie)
            .send()
            .await?;

        let body_text = response.text().await?;
        info!("[DEBUG] Response: {}", &body_text);

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
        
        // Normalize dates to YYYY-MM-DD format for API
        let normalized_from = DateService::normalize_date_for_api(&payload.from)
            .map_err(|_| DebugAppError::DateParse(format!("Invalid from date: {}", payload.from)))?;
        let normalized_to = DateService::normalize_date_for_api(&payload.to)
            .map_err(|_| DebugAppError::DateParse(format!("Invalid to date: {}", payload.to)))?;
            
        let encoded_from_date = urlencoding::encode(&normalized_from);
        let encoded_to_date = urlencoding::encode(&normalized_to);

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
            // Use timestamp from env or generate current timestamp
            let timestamp = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis()
                        .to_string();

            info!("Timestamp: {}", &timestamp);

            let url = format!("{}/transaction-report/datatables?draw={}&start={}&length={}&tglAwal={}&tglAkhir={}&store_id={}&_={}", base_url, draw, start, length, encoded_from_date, encoded_to_date, store_id,timestamp);
            info!("[API] Calling URL: {}", url);
            info!("[API] Cookie (first 50 chars): {}...", &payload.cookie[..std::cmp::min(50, payload.cookie.len())]);
            
            let response = client
                .get(&url)
                .header("Accept", "*/*")
                .header("Cookie", payload.cookie.clone())
                .send()
                .await?;

            let status_code = response.status();
            let body_text = response.text().await?;
            
            // Save detailed request and response info
            let detailed_log = format!(
                "=== REQUEST DETAILS ===\n\
                Method: GET\n\
                URL: {}\n\
                Headers:\n\
                  Accept: */*\n\
                  Cookie: {}\n\n\
                === RESPONSE DETAILS ===\n\
                Status Code: {}\n\
                Body:\n\
                {}\n\n",
                url,
                payload.cookie,
                status_code,
                body_text
            );
            
            let log_filename = format!("request_response_log_{}.txt", draw);
            match fs::write(&log_filename, &detailed_log) {
                Ok(_) => info!("[DEBUG] Saved detailed log to '{}'", log_filename),
                Err(e) => warn!("[DEBUG] Failed to save detailed log: {}", e),
            }
            
            // Debug: Log response untuk troubleshooting
            if body_text.trim().is_empty() {
                warn!("[API] Empty response received for dates {} - {}", normalized_from, normalized_to);
                return Err(DebugAppError::Serialization("Empty response from API".to_string()));
            }
            
            if !body_text.trim_start().starts_with('{') {
                // Check if response contains login page (unauthorized)
                if body_text.contains("<!-- resources/views/auth/login.blade.php -->") {
                    warn!("[API] Unauthorized - redirected to login page");
                    return Err(DebugAppError::Unauthorized("Session expired or invalid cookie".to_string()));
                }
                
                // Write error response to file for debugging
                let error_filename = "last_error_response.txt";
                match fs::write(error_filename, &body_text) {
                    Ok(_) => warn!("[API] Saved error response to '{}'", error_filename),
                    Err(e) => warn!("[API] Failed to save error response: {}", e),
                }
                warn!("[API] Non-JSON response: {}", &body_text[..std::cmp::min(200, body_text.len())]);
                return Err(DebugAppError::Serialization("API returned non-JSON response".to_string()));
            }
            
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

    pub async fn fetch_direct_two_loops(payload: &Payload) -> Result<DebugResponse, DebugAppError> {
        info!("[DIRECT_FETCH] Fetching data dengan 2 loop untuk: {} - {}", payload.from, payload.to);

        let client = reqwest::Client::new();
        
        let normalized_from = DateService::normalize_date_for_api(&payload.from)
            .map_err(|_| DebugAppError::DateParse(format!("Invalid from date: {}", payload.from)))?;
        let normalized_to = DateService::normalize_date_for_api(&payload.to)
            .map_err(|_| DebugAppError::DateParse(format!("Invalid to date: {}", payload.to)))?;
            
        let encoded_from_date = urlencoding::encode(&normalized_from);
        let encoded_to_date = urlencoding::encode(&normalized_to);

        let base_url = env::var("API_BASE_URL")
            .unwrap_or_else(|_| "https://example.com".to_string());
        let store_id = env::var("STORE_ID")
            .unwrap_or_else(|_| "1".to_string());

        // LOOP 1: draw=1, length=10 untuk mendapatkan totalRow
        let timestamp1 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
            .to_string();

        let url1 = format!(
            "{}/transaction-report/datatables?draw=1&start=0&length=10&tglAwal={}&tglAkhir={}&store_id={}&_={}",
            base_url, encoded_from_date, encoded_to_date, store_id, timestamp1
        );

        info!("[DIRECT_FETCH] Loop 1 - URL: {}", url1);

        let response1 = client
            .get(&url1)
            .header("Accept", "*/*")
            .header("Cookie", &payload.cookie)
            .send()
            .await?;

        let body1 = response1.text().await?;
        
        if body1.contains("<!-- resources/views/auth/login.blade.php -->") {
            return Err(DebugAppError::Unauthorized("Session expired or invalid cookie".to_string()));
        }

        let data1: Value = serde_json::from_str(&body1)?;
        let total_row = data1["totalRow"].as_u64().unwrap_or(0) as usize;
        
        info!("[DIRECT_FETCH] Loop 1 - Total row dari server: {}", total_row);

        let mut all_transaksi: Vec<Transaksi> = Vec::new();
        
        // Parse data dari loop 1
        if let Some(records) = data1["data"].as_array() {
            for record in records {
                all_transaksi.push(Self::parse_transaction_record(record));
            }
        }

        // Jika total <= 10, tidak perlu loop 2
        if total_row <= 10 {
            info!("[DIRECT_FETCH] Total row <= 10, tidak perlu loop 2");
            return Ok(DebugResponse {
                total_transaksi: total_row,
                data: all_transaksi,
            });
        }

        // LOOP 2: draw=2, length=total_row untuk mendapatkan semua data
        let delay_ms = rand::thread_rng().gen_range(250..=750);
        sleep(Duration::from_millis(delay_ms)).await;

        let timestamp2 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
            .to_string();

        let url2 = format!(
            "{}/transaction-report/datatables?draw=2&start=0&length={}&tglAwal={}&tglAkhir={}&store_id={}&_={}",
            base_url, total_row, encoded_from_date, encoded_to_date, store_id, timestamp2
        );

        info!("[DIRECT_FETCH] Loop 2 - URL: {}", url2);

        let response2 = client
            .get(&url2)
            .header("Accept", "*/*")
            .header("Cookie", &payload.cookie)
            .send()
            .await?;

        let body2 = response2.text().await?;
        
        if body2.contains("<!-- resources/views/auth/login.blade.php -->") {
            return Err(DebugAppError::Unauthorized("Session expired or invalid cookie".to_string()));
        }

        let data2: Value = serde_json::from_str(&body2)?;
        
        // Clear dan gunakan data dari loop 2 (yang lengkap)
        all_transaksi.clear();
        
        if let Some(records) = data2["data"].as_array() {
            for record in records {
                all_transaksi.push(Self::parse_transaction_record(record));
            }
        }

        info!("[DIRECT_FETCH] Loop 2 - Collected {} transaksi", all_transaksi.len());

        Ok(DebugResponse {
            total_transaksi: total_row,
            data: all_transaksi,
        })
    }

    fn parse_transaction_record(record: &Value) -> Transaksi {
        let tanggal_transaksi = record["tglTrans"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let waktu_transaksi = record["date"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let keterangan = record["xx_keterangan"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let total_tagihan = record["total_tagihan"]
            .as_str()
            .and_then(|s| s.replace(",", "").parse::<i64>().ok())
            .or_else(|| record["total_tagihan"].as_f64().map(|f| f as i64))
            .or_else(|| record["total_tagihan"].as_i64())
            .unwrap_or(0);

        let no_nota = record["xx_no_nota_text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Transaksi {
            tanggal_transaksi,
            waktu_transaksi,
            keterangan,
            total_tagihan,
            no_nota,
        }
    }
}