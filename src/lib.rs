use tokio::time::Instant;

use crate::job_repository::InMemoryJobRepository;

pub mod job_repository;
pub mod web;

#[derive(Clone)]
pub struct AppState {
    pub job_repository: InMemoryJobRepository,
    pub uptime: Instant,
}
