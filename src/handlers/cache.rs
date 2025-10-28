use axum::{http::StatusCode, response::IntoResponse, Json, extract::State};
use serde_json::json;
use crate::models::{Payload, response::{TransaksiResponse, CachedDataResponse}};
use crate::services::{cache_service::CacheService, DateService, TransactionService};
use crate::state::AppState;
use crate::errors::DebugAppError;
use tracing::{info, error};



pub async fn force_refresh_data(
    State(state): State<AppState>,
    Json(payload): Json<Payload>,
) -> impl IntoResponse {
    // Validasi payload
    if payload.cookie.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json!({
            "success": false,
            "message": "Cookie is required"
        })));
    }

    // Normalize dates
    let from_normalized = match DateService::normalize_date_for_api(&payload.from) {
        Ok(d) => d,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(json!({
                "success": false,
                "message": "Invalid from date format"
            })));
        }
    };
    let to_normalized = match DateService::normalize_date_for_api(&payload.to) {
        Ok(d) => d,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(json!({
                "success": false,
                "message": "Invalid to date format"
            })));
        }
    };

    info!("[FORCE_REFRESH] Request: {} to {}", from_normalized, to_normalized);

    // Clear cache for this range
    if let Ok(dates) = DateService::get_date_range(&from_normalized, &to_normalized) {
        let mut cache_write = state.cache.write().await;
        for date in &dates {
            cache_write.remove(date);
        }
        info!("[FORCE_REFRESH] Cleared {} dates from cache", dates.len());
    }

    // Fetch directly with 2-loop (synchronous)
    let refresh_payload = Payload {
        from: from_normalized.clone(),
        to: to_normalized.clone(),
        cookie: payload.cookie.clone(),
    };

    match TransactionService::fetch_direct_two_loops(&refresh_payload).await {
        Ok(response) => {
            if !response.data.is_empty() {
                let range_dates = DateService::get_date_range(&from_normalized, &to_normalized).unwrap();
                let mut cache = state.cache.write().await;
                let mut total_cached = 0;
                
                for date in range_dates {
                    let date_data: Vec<_> = response.data.iter()
                        .filter(|t| {
                            if let Ok(normalized) = DateService::normalize_date_for_api(&t.tanggal_transaksi) {
                                normalized == date
                            } else {
                                false
                            }
                        })
                        .cloned()
                        .collect();
                    if !date_data.is_empty() {
                        cache.insert(date, date_data);
                        total_cached += 1;
                    }
                }
                info!("[FORCE_REFRESH] Cached {} dates, {} transactions", total_cached, response.data.len());
                drop(cache);
                
                if let Err(e) = CacheService::save_cache_to_file(&state).await {
                    error!("[FORCE_REFRESH] Failed to save cache: {:?}", e);
                }
            }

            state.set_unauthorized(false).await;
            (StatusCode::OK, Json(json!({
                "success": true,
                "message": "Cache refreshed successfully",
                "total_transactions": response.data.len()
            })))
        }
        Err(e) => {
            if matches!(e, DebugAppError::Unauthorized(_)) {
                state.set_unauthorized(true).await;
                return (StatusCode::UNAUTHORIZED, Json(json!({
                    "success": false,
                    "message": "Session expired or invalid cookie"
                })));
            }
            error!("[FORCE_REFRESH] Failed: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({
                "success": false,
                "message": "Failed to refresh cache"
            })))
        }
    }
}

pub async fn get_cached_data(
    State(state): State<AppState>,
    Json(payload): Json<Payload>,
) -> impl IntoResponse {
    state.cleanup_old_jobs().await;

    // Normalize dates to DD/MM/YYYY format first
    let from_normalized = match DateService::normalize_date_for_api(&payload.from) {
        Ok(d) => d,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(json!({
                "status": "invalid_date_format",
                "message": "Invalid from date format"
            })));
        }
    };
    let to_normalized = match DateService::normalize_date_for_api(&payload.to) {
        Ok(d) => d,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(json!({
                "status": "invalid_date_format",
                "message": "Invalid to date format"
            })));
        }
    };

    info!("[CACHE_CHECK] Request: {} to {} -> Normalized: {} to {}", 
        payload.from, payload.to, from_normalized, to_normalized);

    // Get date range using normalized dates
    let dates = match DateService::get_date_range(&from_normalized, &to_normalized) {
        Ok(d) => d,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, Json(json!({
                "status": "invalid_date_range",
                "message": "Invalid date format"
            })));
        }
    };

    info!("[CACHE_CHECK] Checking {} dates in cache", dates.len());

    // Check cache and find missing dates
    let mut missing_dates = Vec::new();
    let mut cached_data = Vec::new();
    {
        let cache = state.cache.read().await;
        for date in &dates {
            if let Some(transactions) = cache.get(date) {
                info!("[CACHE_HIT] Found {} transactions for {}", transactions.len(), date);
                cached_data.extend(transactions.clone());
            } else {
                info!("[CACHE_MISS] No data for {}", date);
                missing_dates.push(date.clone());
            }
        }
    }

    // If all cached, return immediately
    if missing_dates.is_empty() {
        state.set_unauthorized(false).await;
        let response = CachedDataResponse {
            status: "completed".to_string(),
            job_id: AppState::generate_job_id(&payload),
            data: TransaksiResponse {
                total_transaksi: cached_data.len(),
                data: cached_data,
            },
            message: Some("All data from cache".to_string()),
        };
        return (StatusCode::OK, Json(serde_json::to_value(response).unwrap()));
    }

    info!("[SMART_FETCH] Missing {} dates, fetching...", missing_dates.len());

    // Group consecutive missing dates into ranges
    let mut ranges: Vec<(String, String)> = Vec::new();
    let mut range_start = missing_dates[0].clone();
    let mut range_end = missing_dates[0].clone();

    for i in 1..missing_dates.len() {
        let prev_date = DateService::parse_date(&missing_dates[i - 1]).unwrap();
        let curr_date = DateService::parse_date(&missing_dates[i]).unwrap();
        
        if (curr_date - prev_date).num_days() == 1 {
            range_end = missing_dates[i].clone();
        } else {
            ranges.push((range_start.clone(), range_end.clone()));
            range_start = missing_dates[i].clone();
            range_end = missing_dates[i].clone();
        }
    }
    ranges.push((range_start, range_end));

    let ranges_count = ranges.len();
    info!("[SMART_FETCH] Grouped into {} ranges", ranges_count);

    // Fetch each range with 2-step pagination
    for (from, to) in ranges {
        let range_payload = Payload {
            from: from.clone(),
            to: to.clone(),
            cookie: payload.cookie.clone(),
        };

        match TransactionService::fetch_direct_two_loops(&range_payload).await {
            Ok(response) => {
                // Only cache if data is not empty
                if !response.data.is_empty() {
                    // Debug: Log sample transaction date format
                    if let Some(first_tx) = response.data.first() {
                        info!("[DEBUG] Sample transaction date from API: '{}'", first_tx.tanggal_transaksi);
                    }
                    
                    let range_dates = DateService::get_date_range(&from, &to).unwrap();
                    info!("[DEBUG] Expected cache key format (first 3): {:?}", range_dates.iter().take(3).collect::<Vec<_>>());
                    
                    let mut cache = state.cache.write().await;
                    let mut total_cached = 0;
                    for date in range_dates {
                        let date_data: Vec<_> = response.data.iter()
                            .filter(|t| {
                                // Normalize transaction date to DD/MM/YYYY for comparison
                                if let Ok(normalized) = DateService::normalize_date_for_api(&t.tanggal_transaksi) {
                                    normalized == date
                                } else {
                                    false
                                }
                            })
                            .cloned()
                            .collect();
                        if !date_data.is_empty() {
                            info!("[CACHE_INSERT] Inserting {} transactions for key '{}'", date_data.len(), date);
                            cache.insert(date, date_data);
                            total_cached += 1;
                        }
                    }
                    info!("[CACHE_SUMMARY] Cached {} dates out of {} fetched transactions", total_cached, response.data.len());
                    drop(cache);
                    // Save to file after caching
                    if let Err(e) = CacheService::save_cache_to_file(&state).await {
                        error!("[SMART_FETCH] Failed to save cache to file: {:?}", e);
                    }
                }
                cached_data.extend(response.data);
            }
            Err(e) => {
                if matches!(e, DebugAppError::Unauthorized(_)) {
                    state.set_unauthorized(true).await;
                    return (StatusCode::UNAUTHORIZED, Json(json!({
                        "status": "unauthorized",
                        "message": "Session expired or invalid cookie"
                    })));
                }
                error!("[SMART_FETCH] Failed to fetch range {}-{}: {:?}", from, to, e);
            }
        }
    }

    state.set_unauthorized(false).await;
    let response = CachedDataResponse {
        status: "completed".to_string(),
        job_id: AppState::generate_job_id(&payload),
        data: TransaksiResponse {
            total_transaksi: cached_data.len(),
            data: cached_data,
        },
        message: Some(format!("Fetched {} missing ranges", ranges_count)),
    };
    (StatusCode::OK, Json(serde_json::to_value(response).unwrap()))
}