use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
// Removed unused imports
use crate::models::Payload;
use crate::services::{CacheService, DateService};
use crate::state::AppState;
use tracing::{info, error};

pub async fn start_fetch_data(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(payload): Json<Payload>,
) -> impl IntoResponse {
    // Cleanup old jobs
    state.cleanup_old_jobs().await;

    // Validasi payload
    if payload.cookie.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json!({"status": "invalid_cookie"})));
    }

    // Validasi range tanggal
    if let Ok(dates) = DateService::get_date_range(&payload.from, &payload.to) {
        if dates.len() > 365 {
            return (StatusCode::BAD_REQUEST, Json(json!({"status": "range_too_large"})));
        }
    } else {
        return (StatusCode::BAD_REQUEST, Json(json!({"status": "invalid_date_range"})));
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
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(payload): Json<Payload>,
) -> impl IntoResponse {
    // Cleanup old jobs
    state.cleanup_old_jobs().await;

    // Validasi payload
    if payload.cookie.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json!({"status": "invalid_cookie"})));
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
    });

    (StatusCode::OK, Json(json!({
        "status": "force_empty_started",
        "job_id": job_id,
        "cleared_dates": empty_dates.len(),
        "dates": empty_dates
    })))
}

pub async fn force_refresh_data(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(payload): Json<Payload>,
) -> impl IntoResponse {
    // Cleanup old jobs
    state.cleanup_old_jobs().await;

    // Validasi payload
    if payload.cookie.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json!({"status": "invalid_cookie"})));
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
    });

    (StatusCode::OK, Json(json!({
        "status": "force_refresh_started",
        "job_id": job_id
    })))
}

pub async fn get_cached_data(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(payload): Json<Payload>,
) -> impl IntoResponse {
    // Cleanup old jobs
    state.cleanup_old_jobs().await;

    let job_id = AppState::generate_job_id(&payload);
    
    // Cek status job spesifik untuk range ini
    if let Some(job_status) = state.get_job_status(&job_id).await {
        match job_status {
            crate::state::JobStatus::Running => {
                return (StatusCode::ACCEPTED, Json(json!({
                    "status": "processing",
                    "job_id": job_id
                })));
            }
            crate::state::JobStatus::Failed(error) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({
                    "status": "failed",
                    "job_id": job_id,
                    "error": error
                })));
            }
            crate::state::JobStatus::Completed => {
                // Continue to fetch data
            }
        }
    }

    match CacheService::get_date_range_data(&state, &payload).await {
        Ok(data) => {
            (StatusCode::OK, Json(json!({
                "status": "completed",
                "job_id": job_id,
                "data": data
            })))
        }
        Err(_) => (StatusCode::NOT_FOUND, Json(json!({
            "status": "not_found",
            "job_id": job_id
        })))
    }
}