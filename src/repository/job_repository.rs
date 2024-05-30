use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::repository::{Job, JobRepository, JobRepositoryError, JobStatus, JobType};

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
    /// The method is not included in public API as it is used for only debugging purpose.
    pub(crate) async fn stats(&self) -> (usize, usize, usize, usize) {
        let jobs = self.jobs.lock().await;
        let job_stats = jobs.values().fold(
            (0usize, 0usize, 0usize, 0usize),
            |(queued, in_progress, concluded, cancelled), job| match job.status {
                JobStatus::Queued => (queued + 1, in_progress, concluded, cancelled),
                JobStatus::InProgress => (queued, in_progress + 1, concluded, cancelled),
                JobStatus::Concluded => (queued, in_progress, concluded + 1, cancelled),
                JobStatus::Cancelled => (queued, in_progress, concluded, cancelled + 1),
            },
        );
        (job_stats.0, job_stats.1, job_stats.2, job_stats.3)
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

    async fn cancel(&self, id: usize) -> Result<Job, JobRepositoryError> {
        let mut queue = self.queue.lock().await;
        let ids: Vec<usize> = queue
            .iter()
            .filter(|job| job.status == JobStatus::Cancelled)
            .enumerate()
            .map(|(idx, _)| idx)
            .collect();
        ids.iter().for_each(|idx| {
            queue.remove(*idx);
        });
        let mut jobs = self.jobs.lock().await;
        return match jobs.get_mut(&id) {
            None => Err(JobRepositoryError::NotFound(id)),
            Some(job) => match job.status {
                JobStatus::Queued | JobStatus::InProgress => {
                    job.status = JobStatus::Cancelled;
                    Ok(job.clone())
                }
                _ => Err(JobRepositoryError::InvalidStatus(id)),
            },
        };
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
        let error = job_repository.conclude(job.id).await.expect_err("fail");
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
        let (queued, in_progress, concluded, cancelled) = job_repository.stats().await;
        // then
        assert_eq!(queued, 3);
        assert_eq!(in_progress, 2);
        assert_eq!(concluded, 1);
        assert_eq!(cancelled, 0);
    }

    #[tokio::test]
    async fn test_cancel_when_job_status_is_queued() {
        // given
        let job_repository = InMemoryJobRepository::new();
        let _ = job_repository.enqueue(JobType::TimeCritical).await;
        let _ = job_repository.enqueue(JobType::TimeCritical).await;
        let _ = job_repository.enqueue(JobType::TimeCritical).await;
        let job = job_repository
            .enqueue(JobType::NotTimeCritical)
            .await
            .unwrap();
        // when
        let cancelled_job = job_repository.cancel(job.id).await.unwrap();
        // then
        assert_eq!(job.id, cancelled_job.id);
        assert_eq!(job.job_type, cancelled_job.job_type);
        assert_eq!(JobStatus::Cancelled, cancelled_job.status);
    }

    #[tokio::test]
    async fn test_cancel_when_job_status_is_in_progress() {
        // given
        let job_repository = InMemoryJobRepository::new();
        let _ = job_repository.enqueue(JobType::TimeCritical).await;
        let _ = job_repository.enqueue(JobType::TimeCritical).await;
        let _ = job_repository.enqueue(JobType::TimeCritical).await;
        let job = job_repository
            .enqueue(JobType::NotTimeCritical)
            .await
            .unwrap();
        let dequeued_job = job_repository.dequeue().await.unwrap();
        // when
        let cancelled_job = job_repository.cancel(dequeued_job.id).await.unwrap();
        // then
        assert_eq!(job.id, cancelled_job.id);
        assert_eq!(job.job_type, cancelled_job.job_type);
        assert_eq!(JobStatus::Cancelled, cancelled_job.status);
    }
}
