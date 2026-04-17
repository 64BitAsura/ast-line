use anyhow::Result;
use axum::{Router, routing::{get, post}};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{AllowOrigin, CorsLayer, Any};
use axum::http::Method;

pub mod routes;

use routes::AppState;

pub async fn start_server(port: u16) -> Result<()> {
    let state = AppState {
        analyze_jobs: Arc::new(Mutex::new(HashMap::new())),
    };

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin, _| {
            let o = origin.as_bytes();
            o.starts_with(b"http://localhost:")
                || o.starts_with(b"http://127.0.0.1:")
                || o.starts_with(b"https://gitnexus.vercel.app")
                || is_local_network(o)
        }))
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
        .allow_headers(Any);

    let app = Router::new()
        .route("/api/heartbeat", get(routes::heartbeat))
        .route("/api/info", get(routes::info))
        .route("/api/repos", get(routes::list_repos))
        .route("/api/repo", get(routes::get_repo).delete(routes::delete_repo))
        .route("/api/graph", get(routes::get_graph))
        .route("/api/query", post(routes::query))
        .route("/api/search", post(routes::symbol_search))
        .route("/api/file", get(routes::get_file))
        .route("/api/grep", get(routes::grep))
        .route("/api/processes", get(routes::get_processes))
        .route("/api/process", get(routes::get_process))
        .route("/api/clusters", get(routes::get_clusters))
        .route("/api/cluster", get(routes::get_cluster))
        .route("/api/analyze", post(routes::start_analyze))
        .route("/api/analyze/:job_id", get(routes::get_analyze_job).delete(routes::cancel_analyze_job))
        .route("/api/analyze/:job_id/progress", get(routes::analyze_job_progress))
        .route("/api/embed", post(routes::embed_stub))
        .route("/api/embed/:job_id", get(routes::embed_job_stub).delete(routes::embed_job_stub))
        .route("/api/embed/:job_id/progress", get(routes::embed_job_stub))
        .layer(cors)
        .with_state(state);

    let addr = format!("0.0.0.0:{port}");
    println!("ast-line server listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn is_local_network(origin: &[u8]) -> bool {
    if origin.starts_with(b"http://192.168.") { return true; }
    if origin.starts_with(b"http://10.") { return true; }
    false
}
