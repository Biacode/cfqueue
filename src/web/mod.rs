use axum::extract::rejection::JsonRejection;
use axum::extract::{FromRequest, MatchedPath, Request};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::Router;
use serde::Serialize;
use tower_http::trace::TraceLayer;

use crate::repository::JobRepositoryError;
use crate::web::controller::{conclude, dequeue, enqueue, find, stats};
use crate::AppState;

mod controller;

pub fn build_server(state: AppState) -> Router {
    Router::new()
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
        .with_state(state)
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
