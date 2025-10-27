#![allow(dead_code)]
#![allow(unused_variables)]

use crate::errors::DebugAppError;
use crate::models::{DebugResponse, Payload, Transaksi};
use crate::services::{TransactionService, DateService};
use crate::state::{AppState, JobStatus};
use tracing::{info, warn, error};
use tokio::time::{sleep, Duration};
use std::collections::HashMap;
use std::env;
use tokio::fs;
use serde_json;

pub struct CacheService;

impl CacheService {
    fn get_cache_file_path() -> String {
        env::var("CACHE_FILE_PATH").unwrap_or_else(|_| "cache_backup.json".to_string())
    }

    // 1. Cek memory cache
    pub async fn get_from_memory_cache(state: &AppState, date: &str) -> Option<Vec<Transaksi>> {
        let cache_read = state.cache.read().await;
        cache_read.get(date).cloned()
    }

    // 2. Load ALL file cache ke memory (one-time operation)
    pub async fn load_all_from_file_cache(state: &AppState) -> Result<usize, DebugAppError> {
        // Clean empty entries first
        Self::clean_empty_cache_entries(state).await?;
        
        let cache_file_path = Self::get_cache_file_path();
        if let Ok(file_content) = fs::read_to_string(&cache_file_path).await {
            if let Ok(file_cache) = serde_json::from_str::<HashMap<String, Vec<Transaksi>>>(&file_content) {
                let count = file_cache.len();
                let mut cache_write = state.cache.write().await;
                
                // Merge dengan existing cache (tidak overwrite), skip empty entries
                for (date, transactions) in file_cache {
                    if !transactions.is_empty() {
                        cache_write.entry(date).or_insert(transactions);
                    }
                }
                
                info!("[FILE_CACHE] Loaded {} dates to memory cache", count);
                return Ok(count);
            }
        }
        Ok(0)
    }

    // Clean empty entries from file cache
    pub async fn clean_empty_cache_entries(state: &AppState) -> Result<usize, DebugAppError> {
        let cache_file_path = Self::get_cache_file_path();
        if let Ok(file_content) = fs::read_to_string(&cache_file_path).await {
            if let Ok(mut file_cache) = serde_json::from_str::<HashMap<String, Vec<Transaksi>>>(&file_content) {
                let original_count = file_cache.len();
                
                // Remove empty entries
                file_cache.retain(|_, transactions| !transactions.is_empty());
                
                let cleaned_count = original_count - file_cache.len();
                
                if cleaned_count > 0 {
                    // Save cleaned cache back to file
                    let json_data = serde_json::to_string_pretty(&file_cache)
                        .map_err(|e| DebugAppError::Serialization(e.to_string()))?;
                    fs::write(&cache_file_path, json_data).await
                        .map_err(|e| DebugAppError::FileWrite(e.to_string()))?;
                    
                    info!("[CACHE_CLEAN] Removed {} empty entries from backup file", cleaned_count);
                }
                
                return Ok(cleaned_count);
            }
        }
        Ok(0)
    }

    // Simple check untuk single date (no file I/O)
    pub async fn get_from_file_cache(state: &AppState, date: &str) -> Option<Vec<Transaksi>> {
        // File cache sudah di-load ke memory saat startup, jadi cek memory saja
        Self::get_from_memory_cache(state, date).await
    }

    // 3. Get missing dates yang perlu di-request
    pub async fn get_missing_dates(state: &AppState, dates: &[String]) -> Vec<String> {
        let mut missing_dates = Vec::new();
        
        for date in dates {
            // Cek apakah data benar-benar ada dan tidak kosong
            match Self::get_from_memory_cache(state, date).await {
                None => missing_dates.push(date.clone()), // Tidak ada data
                Some(transactions) if transactions.is_empty() => missing_dates.push(date.clone()), // Ada tapi kosong
                Some(_) => {} // Ada data yang valid
            }
        }
        
        missing_dates
    }

    // Save cache to file
    pub async fn save_cache_to_file(state: &AppState) -> Result<(), DebugAppError> {
        let cache_read = state.cache.read().await;
        let json_data = serde_json::to_string_pretty(&*cache_read)
            .map_err(|e| DebugAppError::Serialization(e.to_string()))?;
        
        let cache_file_path = Self::get_cache_file_path();
        fs::write(&cache_file_path, json_data).await
            .map_err(|e| DebugAppError::FileWrite(e.to_string()))?;
        
        info!("[FILE_CACHE] Saved {} dates to backup file", cache_read.len());
        Ok(())
    }
    pub async fn is_date_cached(state: &AppState, date: &str) -> bool {
        // Setelah startup, semua file cache sudah di-load ke memory
        // Jadi cukup cek memory cache saja
        Self::get_from_memory_cache(state, date).await.is_some()
    }

    pub async fn is_date_processing(state: &AppState, date: &str) -> bool {
        let processing_read = state.processing.read().await;
        *processing_read.get(date).unwrap_or(&false)
    }

    pub async fn get_cached_transactions_for_date(state: &AppState, date: &str) -> Option<Vec<Transaksi>> {
        // Setelah startup, semua data sudah di-load ke memory
        Self::get_from_memory_cache(state, date).await
    }

    pub async fn set_date_processing(state: &AppState, date: &str, processing: bool) {
        let mut processing_write = state.processing.write().await;
        if processing {
            processing_write.insert(date.to_string(), true);
        } else {
            processing_write.remove(date);
        }
    }

    pub async fn cache_transactions_for_date(state: &AppState, date: &str, transactions: Vec<Transaksi>) {
        // Save to memory cache only - file save akan dilakukan batch
        let mut cache_write = state.cache.write().await;
        cache_write.insert(date.to_string(), transactions);
        drop(cache_write);
        
        info!("[CACHE] Data disimpan ke memory untuk tanggal: {}", date);
    }

    // Batch save untuk efisiensi dan mencegah race condition
    pub async fn save_cache_batch(state: &AppState) -> Result<(), DebugAppError> {
        Self::save_cache_to_file(state).await
    }

    pub async fn get_date_range_transactions(state: &AppState, payload: &Payload) -> Result<Vec<Transaksi>, DebugAppError> {
        let dates = DateService::get_date_range(&payload.from, &payload.to)
            .map_err(|_| DebugAppError::DateParse("Invalid date format".to_string()))?;
        
        let mut all_transactions = Vec::new();
        let mut missing_dates = Vec::new();
        
        // Check cache first
        for date in &dates {
            if let Some(transactions) = Self::get_cached_transactions_for_date(state, date).await {
                all_transactions.extend(transactions);
            } else {
                missing_dates.push(date.clone());
            }
        }
        
        // If no cached data found, return error to trigger fetch
        if all_transactions.is_empty() && !missing_dates.is_empty() {
            return Err(DebugAppError::DateParse("No cached data found".to_string()));
        }
        
        Ok(all_transactions)
    }

    pub async fn get_date_range_data(state: &AppState, payload: &Payload) -> Result<DebugResponse, DebugAppError> {
        let dates = DateService::get_date_range(&payload.from, &payload.to)
            .map_err(|_| DebugAppError::DateParse("Invalid date format".to_string()))?;
        
        let mut all_transactions = Vec::new();
        let mut missing_dates = Vec::new();
        
        for date in &dates {
            if let Some(transactions) = Self::get_cached_transactions_for_date(state, date).await {
                all_transactions.extend(transactions);
                info!("[CACHE] Cache hit untuk tanggal: {}", date);
            } else {
                missing_dates.push(date.clone());
                info!("[CACHE] Cache miss untuk tanggal: {}", date);
            }
        }
        
        if !missing_dates.is_empty() {
            info!("[CACHE] Fetching {} missing dates", missing_dates.len());
            
            for date in missing_dates {
                if Self::is_date_processing(state, &date).await {
                    info!("[CACHE] Tanggal {} sedang diproses, skip", date);
                    continue;
                }
                
                Self::set_date_processing(state, &date, true).await;
                
                let single_date_payload = Payload {
                    from: date.to_string(),
                    to: date.to_string(),
                    cookie: payload.cookie.clone(),
                };
                
                match TransactionService::fetch_all_pages(&single_date_payload).await {
                    Ok(response) => {
                        // Only cache if data is not empty
                        if !response.data.is_empty() {
                            Self::cache_transactions_for_date(state, &date, response.data.clone()).await;
                        }
                        all_transactions.extend(response.data);
                    }
                    Err(e) => {
                        error!("[CACHE] Error fetching data untuk tanggal {}: {:?}", &date, e);
                    }
                }
                
                // PENTING: Selalu reset processing flag, bahkan jika error
                Self::set_date_processing(state, &date, false).await;
            }
        }
        
        Ok(DebugResponse {
            total_transaksi: all_transactions.len(),
            data: all_transactions,
        })
    }

    pub async fn fetch_and_cache_date_range_background(payload: Payload, state: AppState, job_id: String) -> Result<(), DebugAppError> {
        let dates = DateService::get_date_range(&payload.from, &payload.to)
            .map_err(|_| DebugAppError::DateParse("Invalid date format".to_string()))?;
        
        // OPTIMASI: Hanya ambil tanggal yang benar-benar missing
        let missing_dates = Self::get_missing_dates(&state, &dates).await;
        
        if missing_dates.is_empty() {
            info!("[JOB:{}] Semua data sudah tersedia di cache, tidak perlu request", job_id);
            return Ok(());
        }
        
        info!("[JOB:{}] Perlu fetch {} dari {} tanggal", job_id, missing_dates.len(), dates.len());
        
        let batch_size: usize = env::var("BATCH_SIZE")
            .unwrap_or_else(|_| "5".to_string())
            .parse().unwrap_or(5);
        let max_memory_mb: usize = env::var("MAX_MEMORY_MB")
            .unwrap_or_else(|_| "50".to_string())
            .parse().unwrap_or(50);
        
        for batch in missing_dates.chunks(batch_size) {
            let mut batch_memory_usage = 0;
            
            for date in batch {
                // Check if job was cancelled
                if let Some(job_status) = state.get_job_status(&job_id).await {
                    if !matches!(job_status, JobStatus::Running) {
                        info!("[JOB:{}] Job cancelled, stopping", job_id);
                        return Ok(());
                    }
                }
                
                // Double-check cache (might be filled by another job)
                if Self::is_date_cached(&state, date).await {
                    info!("[JOB:{}] Tanggal {} sudah di-cache oleh job lain, skip", job_id, date);
                    continue;
                }
                
                // Atomic check-and-set untuk mencegah race condition
                {
                    let mut processing_write = state.processing.write().await;
                    if processing_write.contains_key(date) {
                        info!("[JOB:{}] Tanggal {} sedang diproses job lain, skip", job_id, date);
                        continue;
                    }
                    processing_write.insert(date.to_string(), true);
                }
                
                let single_date_payload = Payload {
                    from: date.to_string(),
                    to: date.to_string(),
                    cookie: payload.cookie.clone(),
                };
                
                match Self::fetch_with_retry(&single_date_payload, 3).await {
                    Ok(response) => {
                        let estimated_size = response.data.len() * 200;
                        batch_memory_usage += estimated_size;
                        
                        // Only cache if data is not empty
                        if !response.data.is_empty() {
                            Self::cache_transactions_for_date(&state, date, response.data).await;
                            info!("[JOB:{}] Berhasil fetch dan cache tanggal {}", job_id, date);
                        } else {
                            info!("[JOB:{}] Tanggal {} kosong, tidak di-cache", job_id, date);
                        }
                        
                        if batch_memory_usage > max_memory_mb * 1024 * 1024 {
                            info!("[JOB:{}] Memory limit reached, processing batch", job_id);
                            break;
                        }
                    }
                    Err(e) => {
                        error!("[JOB:{}] Failed fetch tanggal {} after retries: {:?}", job_id, date, e);
                        
                        // If unauthorized, set state and stop entire job
                        if matches!(e, DebugAppError::Unauthorized(_)) {
                            error!("[JOB:{}] Unauthorized - stopping entire job", job_id);
                            state.set_unauthorized(true).await;
                            return Err(e);
                        }
                    }
                }
                
                // Reset processing flag dengan proper error handling
                {
                    let mut processing_write = state.processing.write().await;
                    processing_write.remove(date);
                }
            }
            
            // Save cache to file setelah setiap batch (bukan setiap tanggal)
            if let Err(e) = Self::save_cache_batch(&state).await {
                warn!("[JOB:{}] Failed to save cache batch: {:?}", job_id, e);
            }
            
            // Delay between batches
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }
        
        // Final save setelah job selesai
        if let Err(e) = Self::save_cache_batch(&state).await {
            warn!("[JOB:{}] Failed to save final cache: {:?}", job_id, e);
        }
        
        info!("[JOB:{}] Selesai processing {} missing dates", job_id, missing_dates.len());
        Ok(())
    }

    async fn fetch_with_retry(payload: &Payload, max_retries: u32) -> Result<DebugResponse, DebugAppError> {
        let mut last_error = None;
        
        for attempt in 1..=max_retries {
            match TransactionService::fetch_all_pages(payload).await {
                Ok(response) => {
                    if attempt > 1 {
                        info!("[RETRY] Success on attempt {} for {}", attempt, payload.from);
                    }
                    return Ok(response);
                }
                Err(e) => {
                    // If unauthorized, don't retry - return immediately
                    if matches!(e, DebugAppError::Unauthorized(_)) {
                        error!("[RETRY] Unauthorized error for {} - stopping retries", payload.from);
                        return Err(e);
                    }
                    
                    last_error = Some(e);
                    if attempt < max_retries {
                        let delay = Duration::from_millis(1000 * attempt as u64);
                        warn!("[RETRY] Attempt {} failed for {}, retrying in {:?}", attempt, payload.from, delay);
                        sleep(delay).await;
                    }
                }
            }
        }
        
        Err(last_error.unwrap())
    }
}