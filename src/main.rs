use axum::extract::Path;
use axum::routing::{get, put};
use axum::{
    extract::{rejection::JsonRejection, FromRequest, MatchedPath, Request, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
use serde::{Deserialize, Serialize};
use tokio::time::Instant;
use tower_http::trace::TraceLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::job_repository::{
    InMemoryJobRepository, Job, JobRepository, JobRepositoryError, JobType,
};

mod job_repository;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "cqueue=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state = AppState {
        job_repository: InMemoryJobRepository::new(),
        uptime: Instant::now(),
    };

    let app = Router::new()
        .route("/jobs/enqueue", put(enqueue))
        .route("/jobs/dequeue", post(dequeue))
        .route("/jobs/conclude/:job_id", post(conclude))
        .route("/jobs/:job_id", get(find))
        .route("/jobs/stats", get(stats))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|req: &Request| {
                    let method = req.method();
                    let uri = req.uri();
                    let matched_path = req
                        .extensions()
                        .get::<MatchedPath>()
                        .map(|matched_path| matched_path.as_str());
                    tracing::debug_span!("request", %method, %uri, matched_path)
                })
                .on_failure(()),
        )
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("localhost:3000")
        .await
        .unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

#[derive(Clone)]
struct AppState {
    job_repository: InMemoryJobRepository,
    uptime: Instant,
}

#[derive(Deserialize)]
struct EnqueueRequest {
    #[serde(rename = "Type")]
    job_type: JobType,
}

#[derive(Serialize, Clone)]
struct EnqueueResponse {
    #[serde(rename = "ID")]
    id: usize,
}

#[derive(Serialize, Clone)]
struct StatsResponse {
    #[serde(rename = "Queued")]
    queued: usize,
    #[serde(rename = "InProgress")]
    in_progress: usize,
    #[serde(rename = "Concluded")]
    concluded: usize,
    #[serde(rename = "UptimeMillis")]
    uptime: usize,
}

async fn enqueue(
    State(state): State<AppState>,
    AppJson(payload): AppJson<EnqueueRequest>,
) -> Result<AppJson<EnqueueResponse>, AppError> {
    let job = state.job_repository.enqueue(payload.job_type).await?;
    Ok(AppJson(EnqueueResponse { id: job.id }))
}

async fn dequeue(State(state): State<AppState>) -> Result<AppJson<Job>, AppError> {
    Ok(AppJson(state.job_repository.dequeue().await?))
}

async fn conclude(
    Path(job_id): Path<usize>,
    State(state): State<AppState>,
) -> Result<AppJson<Job>, AppError> {
    Ok(AppJson(state.job_repository.conclude(job_id).await?))
}

async fn find(
    Path(job_id): Path<usize>,
    State(state): State<AppState>,
) -> Result<AppJson<Job>, AppError> {
    Ok(AppJson(state.job_repository.find(job_id).await?))
}

async fn stats(State(state): State<AppState>) -> Result<AppJson<StatsResponse>, AppError> {
    let (queued, in_progress, concluded) = state.job_repository.stats().await;
    let uptime = state.uptime.elapsed().as_millis() as usize;
    Ok(AppJson(StatsResponse {
        queued,
        in_progress,
        concluded,
        uptime,
    }))
}

#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(AppError))]
struct AppJson<T>(T);

impl<T> IntoResponse for AppJson<T>
where
    axum::Json<T>: IntoResponse,
{
    fn into_response(self) -> Response {
        axum::Json(self.0).into_response()
    }
}

enum AppError {
    JsonRejection(JsonRejection),
    JobQueueError(JobRepositoryError),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct ErrorResponse {
            message: String,
        }

        let (status, message) = match self {
            AppError::JsonRejection(rejection) => {
                // This error is caused by bad user input so don't log it
                (rejection.status(), rejection.body_text())
            }
            AppError::JobQueueError(err) => {
                // Because `TraceLayer` wraps each request in a span that contains the request
                // method, uri, etc. we don't need to include those details here
                tracing::error!(%err, "error from job repository");
                match err {
                    JobRepositoryError::NotFound(_) => (StatusCode::NOT_FOUND, err.to_string()),
                    JobRepositoryError::Empty => (StatusCode::BAD_REQUEST, err.to_string()),
                    JobRepositoryError::InvalidStatus(_) => (StatusCode::CONFLICT, err.to_string()),
                    JobRepositoryError::Unknown => {
                        (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
                    }
                }
            }
        };

        (status, AppJson(ErrorResponse { message })).into_response()
    }
}

impl From<JsonRejection> for AppError {
    fn from(rejection: JsonRejection) -> Self {
        Self::JsonRejection(rejection)
    }
}

impl From<JobRepositoryError> for AppError {
    fn from(error: JobRepositoryError) -> Self {
        Self::JobQueueError(error)
    }
}
