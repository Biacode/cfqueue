use tokio::time::Instant;

use crate::repository::job_repository::InMemoryJobRepository;

pub mod repository;
pub mod web;

#[derive(Clone)]
pub struct AppState {
    pub job_repository: InMemoryJobRepository,
    pub uptime: Instant,
}
