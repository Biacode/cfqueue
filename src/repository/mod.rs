use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod job_repository;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Job {
    #[serde(rename = "ID")]
    pub id: usize,
    #[serde(rename = "Type")]
    pub job_type: JobType,
    #[serde(rename = "Status")]
    pub status: JobStatus,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum JobType {
    #[serde(rename = "TIME_CRITICAL")]
    TimeCritical,
    #[serde(rename = "NOT_TIME_CRITICAL")]
    NotTimeCritical,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum JobStatus {
    #[serde(rename = "QUEUED")]
    Queued,
    #[serde(rename = "IN_PROGRESS")]
    InProgress,
    #[serde(rename = "CONCLUDED")]
    Concluded,
    #[serde(rename = "CANCELLED")]
    Cancelled,
}

#[derive(Debug, Error, Serialize, Deserialize, PartialEq, Clone)]
pub enum JobRepositoryError {
    #[error("Unable to find a job with ID: `{0}`")]
    NotFound(usize),
    #[error("The job repository queue is empty")]
    Empty,
    #[error("Trying to transition to an invalid job status")]
    InvalidStatus(usize),
    #[error("Unknown job repository error")]
    Unknown,
}

#[async_trait]
pub trait JobRepository {
    /// Add a job to the queue
    async fn enqueue(&self, job_type: JobType) -> Result<Job, JobRepositoryError>;
    /// Returns a job from the queue.
    async fn dequeue(&self) -> Result<Job, JobRepositoryError>;
    /// Provided an input of a job ID, finish execution on the job and consider it done
    async fn conclude(&self, id: usize) -> Result<Job, JobRepositoryError>;
    /// Given an input of a job ID, get information about a job tracked by the queue
    async fn find(&self, id: usize) -> Result<Job, JobRepositoryError>;
    /// Given an input of a job ID, get information about a job tracked by the queue
    async fn cancel(&self, id: usize) -> Result<Job, JobRepositoryError>;
}
