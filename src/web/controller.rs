use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};

use crate::job_repository::{Job, JobRepository, JobType};
use crate::web::{AppError, AppJson};
use crate::AppState;

#[derive(Deserialize)]
pub(super) struct EnqueueRequest {
    #[serde(rename = "Type")]
    pub(super) job_type: JobType,
}

#[derive(Serialize, Clone)]
pub(super) struct EnqueueResponse {
    #[serde(rename = "ID")]
    pub(super) id: usize,
}

#[derive(Serialize, Clone)]
pub(super) struct StatsResponse {
    #[serde(rename = "Queued")]
    pub(super) queued: usize,
    #[serde(rename = "InProgress")]
    pub(super) in_progress: usize,
    #[serde(rename = "Concluded")]
    pub(super) concluded: usize,
    #[serde(rename = "UptimeMillis")]
    pub(super) uptime: usize,
}

pub(super) async fn enqueue(
    State(state): State<AppState>,
    AppJson(payload): AppJson<EnqueueRequest>,
) -> Result<AppJson<EnqueueResponse>, AppError> {
    let job = state.job_repository.enqueue(payload.job_type).await?;
    Ok(AppJson(EnqueueResponse { id: job.id }))
}

pub(super) async fn dequeue(State(state): State<AppState>) -> Result<AppJson<Job>, AppError> {
    Ok(AppJson(state.job_repository.dequeue().await?))
}

pub(super) async fn conclude(
    Path(job_id): Path<usize>,
    State(state): State<AppState>,
) -> Result<AppJson<Job>, AppError> {
    Ok(AppJson(state.job_repository.conclude(job_id).await?))
}

pub(super) async fn find(
    Path(job_id): Path<usize>,
    State(state): State<AppState>,
) -> Result<AppJson<Job>, AppError> {
    Ok(AppJson(state.job_repository.find(job_id).await?))
}

pub(super) async fn stats(
    State(state): State<AppState>,
) -> Result<AppJson<StatsResponse>, AppError> {
    let (queued, in_progress, concluded) = state.job_repository.stats().await;
    let uptime = state.uptime.elapsed().as_millis() as usize;
    Ok(AppJson(StatsResponse {
        queued,
        in_progress,
        concluded,
        uptime,
    }))
}
