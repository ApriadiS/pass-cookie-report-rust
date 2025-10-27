use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::env;
use tokio::sync::RwLock;
use tokio::time::Instant;
use crate::models::{Transaksi, Payload};
use tracing::{info, warn};
// Removed unused serde imports

#[derive(Debug, Clone)]
pub struct JobInfo {
    #[allow(dead_code)]
    pub payload: Payload,
    pub start_time: Instant,
    pub status: JobStatus,
}

#[derive(Debug, Clone)]
pub enum JobStatus {
    Running,
    Completed,
    Failed(String),
}

#[derive(Clone)]
pub struct AppState {
    pub cache: Arc<RwLock<HashMap<String, Vec<Transaksi>>>>, // Key: tanggal, Value: transaksi di tanggal itu
    pub processing: Arc<RwLock<HashMap<String, bool>>>, // Key: tanggal yang sedang diproses
    pub jobs: Arc<RwLock<HashMap<String, JobInfo>>>, // Key: job_id, Value: job info
    pub active_jobs_count: Arc<AtomicBool>, // Simple flag untuk backward compatibility
    pub admin_operations: Arc<RwLock<HashMap<String, bool>>>, // Track running admin operations
    pub unauthorized_state: Arc<RwLock<bool>>, // Track unauthorized state
}

impl AppState {
    // Cleanup stuck processing flags (untuk recovery)
    pub async fn cleanup_stuck_processing(&self) {
        let mut processing_write = self.processing.write().await;
        processing_write.clear();
        info!("[CLEANUP] Cleared all stuck processing flags");
    }
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            processing: Arc::new(RwLock::new(HashMap::new())),
            jobs: Arc::new(RwLock::new(HashMap::new())),
            active_jobs_count: Arc::new(AtomicBool::new(false)),
            admin_operations: Arc::new(RwLock::new(HashMap::new())),
            unauthorized_state: Arc::new(RwLock::new(true)),
        }
    }

    pub async fn load_cache_from_file(&self) -> Result<(), Box<dyn std::error::Error>> {
        use crate::services::CacheService;
        
        // Clean empty entries from file first
        match CacheService::clean_empty_cache_entries(self).await {
            Ok(cleaned) => {
                if cleaned > 0 {
                    info!("[STARTUP] Cleaned {} empty cache entries from file", cleaned);
                }
            }
            Err(e) => {
                warn!("[STARTUP] Failed to clean empty entries: {:?}", e);
            }
        }
        
        // Then load cleaned cache to memory
        match CacheService::load_all_from_file_cache(self).await {
            Ok(count) => {
                if count > 0 {
                    info!("[STARTUP] Loaded {} dates from backup file", count);
                } else {
                    info!("[STARTUP] No backup file found, starting with empty cache");
                }
            }
            Err(e) => {
                warn!("[STARTUP] Failed to load cache: {:?}", e);
            }
        }
        
        Ok(())
    }

    pub fn generate_job_id(payload: &Payload) -> String {
        format!("{}-{}", payload.from, payload.to)
    }

    pub async fn start_job(&self, payload: Payload) -> Result<String, String> {
        let job_id = Self::generate_job_id(&payload);
        let mut jobs = self.jobs.write().await;
        
        // Cek apakah job dengan range yang sama sudah berjalan
        if let Some(existing_job) = jobs.get(&job_id) {
            if matches!(existing_job.status, JobStatus::Running)
                && existing_job.start_time.elapsed().as_secs() < 300 {
                return Err("Job already running for this range".to_string());
            }
        }
        
        // Limit concurrent jobs from env variable
        let max_concurrent_jobs: usize = env::var("MAX_CONCURRENT_JOBS")
            .unwrap_or_else(|_| "3".to_string())
            .parse().unwrap_or(3);
        let running_jobs = jobs.values().filter(|j| matches!(j.status, JobStatus::Running)).count();
        if running_jobs >= max_concurrent_jobs {
            return Err("Too many concurrent jobs".to_string());
        }
        
        jobs.insert(job_id.clone(), JobInfo {
            payload,
            start_time: Instant::now(),
            status: JobStatus::Running,
        });
        
        self.active_jobs_count.store(true, Ordering::Relaxed);
        Ok(job_id)
    }

    pub async fn complete_job(&self, job_id: &str, status: JobStatus) {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(job_id) {
            job.status = status;
        }
        
        // Update active jobs flag
        let has_running = jobs.values().any(|j| matches!(j.status, JobStatus::Running));
        self.active_jobs_count.store(has_running, Ordering::Relaxed);
    }

    pub async fn get_job_status(&self, job_id: &str) -> Option<JobStatus> {
        let jobs = self.jobs.read().await;
        jobs.get(job_id).map(|j| j.status.clone())
    }

    pub async fn cleanup_old_jobs(&self) {
        let mut jobs = self.jobs.write().await;
        jobs.retain(|_, job| {
            match job.status {
                JobStatus::Running => job.start_time.elapsed().as_secs() < 300,
                _ => job.start_time.elapsed().as_secs() < 3600, // Keep completed jobs for 1 hour
            }
        });
        
        // Cleanup stuck processing flags juga
        drop(jobs);
        self.cleanup_stuck_processing().await;
    }

    /// Check if admin operation can be started (only one instance allowed)
    pub async fn start_admin_operation(&self, operation: &str) -> bool {
        let mut ops = self.admin_operations.write().await;
        
        // Check if operation is already running
        if ops.get(operation).unwrap_or(&false) == &true {
            info!("[ADMIN] Operation '{}' blocked - already running", operation);
            return false;
        }
        
        // Mark operation as running
        ops.insert(operation.to_string(), true);
        info!("[ADMIN] Operation '{}' started", operation);
        true
    }
    
    /// Mark admin operation as completed
    pub async fn complete_admin_operation(&self, operation: &str) {
        let mut ops = self.admin_operations.write().await;
        ops.insert(operation.to_string(), false);
        info!("[ADMIN] Operation '{}' completed", operation);
    }

    /// Set unauthorized state
    pub async fn set_unauthorized(&self, unauthorized: bool) {
        let mut state = self.unauthorized_state.write().await;
        *state = unauthorized;
        if unauthorized {
            info!("[AUTH] Unauthorized state set - future requests will be rejected");
        } else {
            info!("[AUTH] Unauthorized state cleared");
        }
    }

    /// Check if currently unauthorized
    pub async fn is_unauthorized(&self) -> bool {
        let state = self.unauthorized_state.read().await;
        *state
    }
}