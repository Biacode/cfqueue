use std::env;

use clap::Parser;
use tokio::time::Instant;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use cfqueue::repository::job_repository::InMemoryJobRepository;
use cfqueue::web;
use cfqueue::web::AppState;

#[derive(Parser, Debug)]
#[command(
    version,
    about,
    long_about = "An extremely simple in-memory job queue implementation."
)]
struct Args {
    /// Server address.
    #[arg(short, long, default_value_t = String::from("127.0.0.1"))]
    addr: String,

    /// Server port.
    #[arg(short, long, default_value_t = 3000)]
    port: u16,

    /// Root logging level.
    #[arg(short, long, default_value_t = String::from("debug"))]
    log_level: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let server_addr = env::var("CFQUEUE_ADDR").unwrap_or(args.addr);
    let server_port = env::var("CFQUEUE_PORT")
        .map(|it| it.parse::<u16>().unwrap())
        .unwrap_or(args.port);
    let log_level = env::var("CFQUEUE_LOG_LEVEL").unwrap_or(args.log_level);
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("cfqueue={log_level},tower_http={log_level}").into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state = AppState {
        job_repository: InMemoryJobRepository::new(),
        uptime: Instant::now(),
    };

    let app = web::build_server(state);

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", server_addr, server_port))
        .await
        .unwrap();
    tracing::info!("🚀🚀🚀CFQueue is up and running🚀🚀🚀");
    tracing::info!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
