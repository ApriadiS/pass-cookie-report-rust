use axum::{http::StatusCode, response::IntoResponse, Json, extract::State};
use serde_json::json;
use crate::models::{Payload, response::{TransaksiResponse, CachedDataResponse}};
use crate::services::{CacheService, DateService, TransactionService};
use crate::state::AppState;
use crate::errors::DebugAppError;
use tracing::{info, error};

pub async fn start_fetch_data(
    State(state): State<AppState>,
    Json(payload): Json<Payload>,
) -> impl IntoResponse {
    // Cleanup old jobs
    state.cleanup_old_jobs().await;

    // Validasi payload
    if payload.cookie.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json!({
            "status": "invalid_cookie",
            "message": "Cookie is required"
        })));
    }

    // Validasi range tanggal
    if let Ok(dates) = DateService::get_date_range(&payload.from, &payload.to) {
        if dates.len() > 365 {
            return (StatusCode::BAD_REQUEST, Json(json!({
                "status": "range_too_large",
                "message": "Date range cannot exceed 365 days"
            })));
        }
    } else {
        return (StatusCode::BAD_REQUEST, Json(json!({
            "status": "invalid_date_range",
            "message": "Invalid date format. Use DD/MM/YYYY format"
        })));
    }

    // Start job dengan per-range tracking
    let job_id = match state.start_job(payload.clone()).await {
        Ok(id) => id,
        Err(msg) => {
            return (StatusCode::TOO_MANY_REQUESTS, Json(json!({
                "status": "rejected",
                "message": msg
            })));
        }
    };

    let state_clone = state.clone();
    let payload_clone = payload.clone();
    let job_id_clone = job_id.clone();

    tokio::spawn(async move {
        let result = CacheService::fetch_and_cache_date_range_background(
            payload_clone, state_clone.clone(), job_id_clone.clone()
        ).await;
        
        let final_status = match result {
            Ok(_) => {
                info!("[JOB:{}] Completed successfully", job_id_clone);
                crate::state::JobStatus::Completed
            }
            Err(e) => {
                error!("[JOB:{}] Failed: {:?}", job_id_clone, e);
                crate::state::JobStatus::Failed(format!("{:?}", e))
            }
        };
        
        state_clone.complete_job(&job_id_clone, final_status).await;
    });

    (StatusCode::OK, Json(json!({
        "status": "started",
        "job_id": job_id
    })))
}

pub async fn force_empty_cache(
    State(state): State<AppState>,
    Json(payload): Json<Payload>,
) -> impl IntoResponse {
    // Check if force_empty is already running
    if !state.start_admin_operation("force_empty").await {
        return (StatusCode::CONFLICT, Json(json!({
            "status": "already_running",
            "message": "Force empty operation is already in progress"
        })));
    }

    // Cleanup old jobs
    state.cleanup_old_jobs().await;

    // Validasi payload
    if payload.cookie.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json!({
            "status": "invalid_cookie",
            "message": "Cookie is required"
        })));
    }

    // Cek dan hapus hanya cache yang kosong
    let mut empty_dates = Vec::new();
    if let Ok(dates) = DateService::get_date_range(&payload.from, &payload.to) {
        let mut cache_write = state.cache.write().await;
        for date in &dates {
            if let Some(transactions) = cache_write.get(date) {
                if transactions.is_empty() {
                    cache_write.remove(date);
                    empty_dates.push(date.clone());
                }
            }
        }
    }

    if empty_dates.is_empty() {
        return (StatusCode::OK, Json(json!({
            "status": "no_empty_cache_found",
            "message": "No empty cache entries found for the specified date range"
        })));
    }

    // Start job dengan per-range tracking
    let job_id = match state.start_job(payload.clone()).await {
        Ok(id) => id,
        Err(msg) => {
            return (StatusCode::TOO_MANY_REQUESTS, Json(json!({
                "status": "rejected",
                "message": msg
            })));
        }
    };

    let state_clone = state.clone();
    let payload_clone = payload.clone();
    let job_id_clone = job_id.clone();

    tokio::spawn(async move {
        let result = CacheService::fetch_and_cache_date_range_background(
            payload_clone, state_clone.clone(), job_id_clone.clone()
        ).await;
        
        let final_status = match result {
            Ok(_) => {
                info!("[JOB:{}] Force empty cache completed successfully", job_id_clone);
                crate::state::JobStatus::Completed
            }
            Err(e) => {
                error!("[JOB:{}] Force empty cache failed: {:?}", job_id_clone, e);
                crate::state::JobStatus::Failed(format!("{:?}", e))
            }
        };
        
        state_clone.complete_job(&job_id_clone, final_status).await;
        
        // Mark admin operation as completed
        state_clone.complete_admin_operation("force_empty").await;
    });

    (StatusCode::OK, Json(json!({
        "status": "force_empty_started",
        "job_id": job_id,
        "cleared_dates": empty_dates.len(),
        "dates": empty_dates
    })))
}

pub async fn force_refresh_data(
    State(state): State<AppState>,
    Json(payload): Json<Payload>,
) -> impl IntoResponse {
    // Check if force_refresh is already running
    if !state.start_admin_operation("force_refresh").await {
        return (StatusCode::CONFLICT, Json(json!({
            "status": "already_running",
            "message": "Force refresh operation is already in progress"
        })));
    }

    // Cleanup old jobs
    state.cleanup_old_jobs().await;

    // Validasi payload
    if payload.cookie.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json!({
            "status": "invalid_cookie",
            "message": "Cookie is required"
        })));
    }

    // Hapus cache untuk range ini agar dipaksa fetch ulang
    if let Ok(dates) = DateService::get_date_range(&payload.from, &payload.to) {
        let mut cache_write = state.cache.write().await;
        for date in &dates {
            cache_write.remove(date);
        }
        info!("[FORCE_REFRESH] Cleared cache for {} dates", dates.len());
    }

    // Start job dengan per-range tracking
    let job_id = match state.start_job(payload.clone()).await {
        Ok(id) => id,
        Err(msg) => {
            return (StatusCode::TOO_MANY_REQUESTS, Json(json!({
                "status": "rejected",
                "message": msg
            })));
        }
    };

    let state_clone = state.clone();
    let payload_clone = payload.clone();
    let job_id_clone = job_id.clone();

    tokio::spawn(async move {
        let result = CacheService::fetch_and_cache_date_range_background(
            payload_clone, state_clone.clone(), job_id_clone.clone()
        ).await;
        
        let final_status = match result {
            Ok(_) => {
                info!("[JOB:{}] Force refresh completed successfully", job_id_clone);
                crate::state::JobStatus::Completed
            }
            Err(e) => {
                error!("[JOB:{}] Force refresh failed: {:?}", job_id_clone, e);
                crate::state::JobStatus::Failed(format!("{:?}", e))
            }
        };
        
        state_clone.complete_job(&job_id_clone, final_status).await;
        
        // Mark admin operation as completed
        state_clone.complete_admin_operation("force_refresh").await;
    });

    (StatusCode::OK, Json(json!({
        "status": "force_refresh_started",
        "job_id": job_id
    })))
}

pub async fn get_cached_data(
    State(state): State<AppState>,
    Json(payload): Json<Payload>,
) -> impl IntoResponse {
    // Cleanup old jobs
    state.cleanup_old_jobs().await;

    // Always try with provided cookie (don't check global unauthorized state)
    match TransactionService::fetch_direct_two_loops(&payload).await {
        Ok(response) => {
            // Reset unauthorized state on success
            state.set_unauthorized(false).await;
            
            let transaksi_response = TransaksiResponse {
                total_transaksi: response.total_transaksi,
                data: response.data,
            };
            let response = CachedDataResponse {
                status: "completed".to_string(),
                job_id: AppState::generate_job_id(&payload),
                data: transaksi_response,
                message: None,
            };
            (StatusCode::OK, Json(serde_json::to_value(response).unwrap()))
        }
        Err(e) => {
            if matches!(e, DebugAppError::Unauthorized(_)) {
                // Set unauthorized state only after actual failure
                state.set_unauthorized(true).await;
                return (StatusCode::UNAUTHORIZED, Json(json!({
                    "status": "unauthorized",
                    "message": "Session expired or invalid cookie"
                })));
            }
            
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({
                "status": "error",
                "message": format!("{:?}", e)
            })))
        }
    }
}