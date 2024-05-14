use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::Mutex;

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
}

#[derive(Default, Clone)]
pub struct InMemoryJobRepository {
    jobs: Arc<Mutex<HashMap<usize, Job>>>,
    queue: Arc<Mutex<VecDeque<Job>>>,
}

impl InMemoryJobRepository {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    /// Collect the current queue and job stats.
    ///
    /// The output is a tuple with the following format (`<queue len>`, `<queued jobs>`, `<in progress jobs>`, `<concluded jobs>`)
    ///
    /// The method is not included in public API as it is primarily used for debugging purposes.
    pub(crate) async fn stats(&self) -> (usize, usize, usize) {
        let jobs = self.jobs.lock().await;
        let job_stats = jobs.values().fold(
            (0usize, 0usize, 0usize),
            |(queued, in_progress, concluded), job| match job.status {
                JobStatus::Queued => (queued + 1, in_progress, concluded),
                JobStatus::InProgress => (queued, in_progress + 1, concluded),
                JobStatus::Concluded => (queued, in_progress, concluded + 1),
            },
        );
        (job_stats.0, job_stats.1, job_stats.2)
    }
}

#[async_trait]
impl JobRepository for InMemoryJobRepository {
    async fn enqueue(&self, job_type: JobType) -> Result<Job, JobRepositoryError> {
        let mut jobs = self.jobs.lock().await;
        let id = jobs.len() + 1;
        let job = Job {
            id,
            job_type,
            status: JobStatus::Queued,
        };
        let mut queue = self.queue.lock().await;
        queue.push_back(job.clone());
        jobs.insert(id, job.clone());
        Ok(job)
    }

    async fn dequeue(&self) -> Result<Job, JobRepositoryError> {
        match self.queue.lock().await.pop_back() {
            Some(queued_job) => {
                let mut jobs = self.jobs.lock().await;
                match jobs.get_mut(&queued_job.id) {
                    Some(job) if job.status == JobStatus::Queued => {
                        job.status = JobStatus::InProgress;
                        Ok(job.clone())
                    }
                    _ => Err(JobRepositoryError::Unknown),
                }
            }
            None => Err(JobRepositoryError::Empty),
        }
    }

    async fn conclude(&self, id: usize) -> Result<Job, JobRepositoryError> {
        let mut jobs = self.jobs.lock().await;
        match jobs.get_mut(&id) {
            Some(job) if job.status == JobStatus::InProgress => {
                job.status = JobStatus::Concluded;
                Ok(job.clone())
            }
            Some(_) => Err(JobRepositoryError::InvalidStatus(id)),
            None => Err(JobRepositoryError::NotFound(id)),
        }
    }

    async fn find(&self, id: usize) -> Result<Job, JobRepositoryError> {
        self.jobs
            .lock()
            .await
            .get(&id)
            .cloned()
            .ok_or(JobRepositoryError::NotFound(id))
    }
}

mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[tokio::test]
    async fn test_enqueue() {
        // given
        let job_repository = InMemoryJobRepository::new();
        // when
        let job = job_repository.enqueue(JobType::TimeCritical).await.unwrap();
        // then
        assert_eq!(job.id, 1);
        assert_eq!(job.job_type, JobType::TimeCritical);
        assert_eq!(job.status, JobStatus::Queued);
    }

    #[tokio::test]
    async fn test_check_id_increment() {
        // given
        let job_repository = InMemoryJobRepository::new();
        for _ in 0..1000 {
            job_repository.enqueue(JobType::TimeCritical).await.unwrap();
        }
        // when
        let job = job_repository
            .enqueue(JobType::NotTimeCritical)
            .await
            .unwrap();
        // then
        assert_eq!(job.id, 1001);
        assert_eq!(job.job_type, JobType::NotTimeCritical);
        assert_eq!(job.status, JobStatus::Queued);
    }

    #[tokio::test]
    async fn test_dequeue_when_empty() {
        // given
        let job_repository = InMemoryJobRepository::new();
        // when
        let err = job_repository.dequeue().await.expect_err("fail");
        // then
        assert_eq!(err, JobRepositoryError::Empty);
    }

    #[tokio::test]
    async fn test_double_dequeue_when_empty() {
        // given
        let job_repository = InMemoryJobRepository::new();
        job_repository.enqueue(JobType::TimeCritical).await.unwrap();
        // when
        job_repository.dequeue().await.unwrap();
        // and
        let err = job_repository.dequeue().await.expect_err("fail");
        // then
        assert_eq!(err, JobRepositoryError::Empty);
    }

    #[tokio::test]
    async fn test_dequeue() {
        // given
        let job_repository = InMemoryJobRepository::new();
        let enq_job = job_repository.enqueue(JobType::TimeCritical).await.unwrap();
        // when
        let deq_job = job_repository.dequeue().await.unwrap();
        // then
        assert_eq!(deq_job.id, enq_job.id);
        assert_eq!(deq_job.id, 1);
        assert_eq!(deq_job.job_type, enq_job.job_type);
        assert_eq!(deq_job.job_type, JobType::TimeCritical);
        assert_eq!(enq_job.status, JobStatus::Queued);
        assert_eq!(deq_job.status, JobStatus::InProgress);
    }

    #[tokio::test]
    async fn test_conclude_when_not_found() {
        // given
        let job_repository = InMemoryJobRepository::new();
        let id = 1;
        // when
        let error = job_repository.conclude(id).await.expect_err("fail");
        // then
        assert_eq!(error, JobRepositoryError::NotFound(id));
    }

    #[tokio::test]
    async fn test_conclude_when_not_in_progress() {
        // given
        let job_repository = InMemoryJobRepository::new();
        let job = job_repository.enqueue(JobType::TimeCritical).await.unwrap();
        // when
        let error = job_repository
            .conclude(job.id)
            .await
            .expect_err("fail");
        // then
        assert_eq!(error, JobRepositoryError::InvalidStatus(job.id));
    }

    #[tokio::test]
    async fn test_conclude() {
        // given
        let job_repository = InMemoryJobRepository::new();
        job_repository
            .enqueue(JobType::NotTimeCritical)
            .await
            .unwrap();
        let job = job_repository.dequeue().await.unwrap();
        // when
        let job = job_repository.conclude(job.id).await.unwrap();
        // then
        assert_eq!(job.id, 1);
        assert_eq!(job.status, JobStatus::Concluded);
        assert_eq!(job.job_type, JobType::NotTimeCritical);
    }

    #[tokio::test]
    async fn test_find() {
        let job_repository = InMemoryJobRepository::new();
        let job = job_repository.enqueue(JobType::TimeCritical).await.unwrap();
        let job = job_repository.find(job.id).await.unwrap();
        assert_eq!(job.id, 1);
        assert_eq!(job.status, JobStatus::Queued);
        assert_eq!(job.job_type, JobType::TimeCritical);
        let job = job_repository.dequeue().await.unwrap();
        let job = job_repository.find(job.id).await.unwrap();
        assert_eq!(job.id, 1);
        assert_eq!(job.status, JobStatus::InProgress);
        assert_eq!(job.job_type, JobType::TimeCritical);
        let job = job_repository.conclude(job.id).await.unwrap();
        assert_eq!(job.id, 1);
        assert_eq!(job.status, JobStatus::Concluded);
        assert_eq!(job.job_type, JobType::TimeCritical);
        let job = job_repository.find(job.id).await.unwrap();
        let job = job_repository.find(job.id).await.unwrap();
        assert_eq!(job.id, 1);
        assert_eq!(job.status, JobStatus::Concluded);
        assert_eq!(job.job_type, JobType::TimeCritical);
    }

    #[tokio::test]
    async fn test_full_flow() {
        let job_repository = InMemoryJobRepository::new();
        let job = job_repository.enqueue(JobType::TimeCritical).await.unwrap();
        assert_eq!(job.id, 1);
        assert_eq!(job.status, JobStatus::Queued);
        assert_eq!(job.job_type, JobType::TimeCritical);
        let job = job_repository.dequeue().await.unwrap();
        assert_eq!(job.id, 1);
        assert_eq!(job.status, JobStatus::InProgress);
        assert_eq!(job.job_type, JobType::TimeCritical);
        let job = job_repository.conclude(job.id).await.unwrap();
        assert_eq!(job.id, 1);
        assert_eq!(job.status, JobStatus::Concluded);
        assert_eq!(job.job_type, JobType::TimeCritical);
        let job = job_repository.find(job.id).await.unwrap();
        assert_eq!(job.id, 1);
        assert_eq!(job.status, JobStatus::Concluded);
        assert_eq!(job.job_type, JobType::TimeCritical);
    }

    #[tokio::test]
    async fn test_stats() {
        let job_repository = InMemoryJobRepository::new();
        // given
        job_repository.enqueue(JobType::TimeCritical).await.unwrap();
        job_repository.enqueue(JobType::TimeCritical).await.unwrap();
        job_repository.enqueue(JobType::TimeCritical).await.unwrap();
        job_repository.enqueue(JobType::TimeCritical).await.unwrap();
        job_repository.enqueue(JobType::TimeCritical).await.unwrap();
        job_repository.enqueue(JobType::TimeCritical).await.unwrap();

        job_repository.dequeue().await.unwrap();
        job_repository.dequeue().await.unwrap();
        let job = job_repository.dequeue().await.unwrap();

        job_repository.conclude(job.id).await.unwrap();
        // when
        let (queued, in_progress, concluded) = job_repository.stats().await;
        // then
        assert_eq!(queued, 3);
        assert_eq!(in_progress, 2);
        assert_eq!(concluded, 1);
    }
}
