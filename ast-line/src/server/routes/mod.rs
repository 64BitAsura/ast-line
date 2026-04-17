use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{sse::{Event, Sse}, IntoResponse, Json},
};
use futures::stream::{self, Stream};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, Mutex};
use tokio_stream::StreamExt as _;
use tokio_stream::wrappers::BroadcastStream;
use uuid::Uuid;

use ast_line_core::ingestion::pipeline::{PipelineOptions, run_pipeline_from_repo};
use ast_line_core::search::{grep_files, index_graph, search};
use ast_line_core::storage::repo_manager::{load_graph, unregister_repo};
use ast_line_core::types::pipeline::PipelineProgress;
use ast_line_core::types::graph::NodeKind;

// ── Shared State ─────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    pub analyze_jobs: Arc<Mutex<HashMap<String, AnalyzeJob>>>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Running,
    Done,
    Error(String),
    Cancelled,
}

#[derive(Debug)]
pub struct AnalyzeJob {
    pub job_id: String,
    pub status: JobStatus,
    pub progress: Option<PipelineProgress>,
    pub progress_tx: Option<broadcast::Sender<PipelineProgress>>,
}

// ── Query Params ──────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct NameParam {
    pub name: Option<String>,
}

#[derive(Deserialize)]
pub struct FileParam {
    pub name: String,
    pub path: String,
}

#[derive(Deserialize)]
pub struct GrepParam {
    pub name: String,
    pub query: String,
    #[serde(rename = "caseSensitive", default)]
    pub case_sensitive: bool,
}

#[derive(Deserialize)]
pub struct ProcessParam {
    pub name: String,
    pub id: String,
}

// ── Heartbeat SSE ─────────────────────────────────────────────────────────────

pub async fn heartbeat() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let s = stream::unfold((), |_| async {
        tokio::time::sleep(Duration::from_secs(5)).await;
        let event = Event::default().data(r#"{"type":"heartbeat"}"#);
        Some((Ok::<Event, Infallible>(event), ()))
    });
    Sse::new(s).keep_alive(axum::response::sse::KeepAlive::default())
}

// ── Info ──────────────────────────────────────────────────────────────────────

pub async fn info() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "version": "0.1.0",
        "launchContext": "cli",
        "nodeVersion": null,
        "rustVersion": "1.87"
    }))
}

// ── Repos ─────────────────────────────────────────────────────────────────────

pub async fn list_repos() -> impl IntoResponse {
    match ast_line_core::storage::repo_manager::list_repos().await {
        Ok(repos) => Json(serde_json::to_value(&repos).unwrap_or_default()).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn get_repo(Query(params): Query<NameParam>) -> impl IntoResponse {
    let Some(name) = params.name else {
        return (StatusCode::BAD_REQUEST, "missing name").into_response();
    };
    match ast_line_core::storage::repo_manager::list_repos().await {
        Ok(repos) => {
            if let Some(repo) = repos.into_iter().find(|r| r.name == name) {
                Json(serde_json::to_value(&repo).unwrap_or_default()).into_response()
            } else {
                (StatusCode::NOT_FOUND, "repo not found").into_response()
            }
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn delete_repo(Query(params): Query<NameParam>) -> impl IntoResponse {
    let Some(name) = params.name else {
        return (StatusCode::BAD_REQUEST, "missing name").into_response();
    };
    match unregister_repo(&name).await {
        Ok(_) => Json(serde_json::json!({"ok": true})).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// ── Graph SSE ──────────────────────────────────────────────────────────────────

pub async fn get_graph(Query(params): Query<NameParam>) -> impl IntoResponse {
    let Some(name) = params.name else {
        return (StatusCode::BAD_REQUEST, "missing name").into_response();
    };

    let repo_path = match get_repo_path(&name).await {
        Ok(Some(p)) => p,
        Ok(None) => return (StatusCode::NOT_FOUND, "repo not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    let graph = match load_graph(&repo_path).await {
        Ok(g) => g,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    let mut events: Vec<Event> = Vec::new();
    for node in graph.all_nodes_json() {
        let data = serde_json::to_string(&serde_json::json!({"type":"node","data":node})).unwrap_or_default();
        events.push(Event::default().data(data));
    }
    for rel in graph.all_relationships_json() {
        let data = serde_json::to_string(&serde_json::json!({"type":"relationship","data":rel})).unwrap_or_default();
        events.push(Event::default().data(data));
    }
    events.push(Event::default().data("{\"type\":\"done\"}"));

    let s = stream::iter(events.into_iter().map(Ok::<Event, Infallible>));
    Sse::new(s).into_response()
}

// ── Query ─────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct QueryBody {
    pub query: String,
    pub name: String,
    pub limit: Option<usize>,
}

pub async fn query(Json(body): Json<QueryBody>) -> impl IntoResponse {
    let repo_path = match get_repo_path(&body.name).await {
        Ok(Some(p)) => p,
        Ok(None) => return (StatusCode::NOT_FOUND, "repo not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    let graph = match load_graph(&repo_path).await {
        Ok(g) => g,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    let index = index_graph(&graph);
    let results = search(&index, &body.query, body.limit.unwrap_or(20));
    let results_json: Vec<_> = results.iter().map(|r| serde_json::json!({
        "nodeId": r.node_id,
        "score": r.score,
        "snippet": r.snippet,
    })).collect();

    Json(serde_json::json!({"results": results_json})).into_response()
}

// ── Symbol Search ──────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SearchBody {
    pub symbol: String,
    pub name: String,
}

pub async fn symbol_search(Json(body): Json<SearchBody>) -> impl IntoResponse {
    let repo_path = match get_repo_path(&body.name).await {
        Ok(Some(p)) => p,
        Ok(None) => return (StatusCode::NOT_FOUND, "repo not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    let graph = match load_graph(&repo_path).await {
        Ok(g) => g,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    let q = body.symbol.to_lowercase();
    let results: Vec<_> = graph.nodes()
        .filter(|n| n.name.to_lowercase().contains(&q))
        .map(|n| serde_json::to_value(n).unwrap_or_default())
        .collect();

    Json(serde_json::json!({"results": results})).into_response()
}

// ── File ──────────────────────────────────────────────────────────────────────

pub async fn get_file(Query(params): Query<FileParam>) -> impl IntoResponse {
    let repo_path = match get_repo_path(&params.name).await {
        Ok(Some(p)) => p,
        Ok(None) => return (StatusCode::NOT_FOUND, "repo not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    let file_path = repo_path.join(&params.path);
    match tokio::fs::read_to_string(&file_path).await {
        Ok(content) => Json(serde_json::json!({"content": content, "path": params.path})).into_response(),
        Err(e) => (StatusCode::NOT_FOUND, e.to_string()).into_response(),
    }
}

// ── Grep ──────────────────────────────────────────────────────────────────────

pub async fn grep(Query(params): Query<GrepParam>) -> impl IntoResponse {
    let repo_path = match get_repo_path(&params.name).await {
        Ok(Some(p)) => p,
        Ok(None) => return (StatusCode::NOT_FOUND, "repo not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    let case_insensitive = !params.case_sensitive;
    match grep_files(&repo_path, &params.query, case_insensitive) {
        Ok(results) => Json(serde_json::json!({"results": results})).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// ── Processes ────────────────────────────────────────────────────────────────

pub async fn get_processes(Query(params): Query<NameParam>) -> impl IntoResponse {
    let Some(name) = params.name else {
        return (StatusCode::BAD_REQUEST, "missing name").into_response();
    };
    let repo_path = match get_repo_path(&name).await {
        Ok(Some(p)) => p,
        Ok(None) => return (StatusCode::NOT_FOUND, "repo not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    let graph = match load_graph(&repo_path).await {
        Ok(g) => g,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    let nodes: Vec<_> = graph.nodes_of_kind(&NodeKind::Process)
        .map(|n| serde_json::to_value(n).unwrap_or_default())
        .collect();
    Json(serde_json::json!({"nodes": nodes})).into_response()
}

pub async fn get_process(Query(params): Query<ProcessParam>) -> impl IntoResponse {
    let repo_path = match get_repo_path(&params.name).await {
        Ok(Some(p)) => p,
        Ok(None) => return (StatusCode::NOT_FOUND, "repo not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    let graph = match load_graph(&repo_path).await {
        Ok(g) => g,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    match graph.get_node(&params.id) {
        Some(node) => Json(serde_json::to_value(node).unwrap_or_default()).into_response(),
        None => (StatusCode::NOT_FOUND, "node not found").into_response(),
    }
}

// ── Clusters/Communities ──────────────────────────────────────────────────────

pub async fn get_clusters(Query(params): Query<NameParam>) -> impl IntoResponse {
    let Some(name) = params.name else {
        return (StatusCode::BAD_REQUEST, "missing name").into_response();
    };
    let repo_path = match get_repo_path(&name).await {
        Ok(Some(p)) => p,
        Ok(None) => return (StatusCode::NOT_FOUND, "repo not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    let graph = match load_graph(&repo_path).await {
        Ok(g) => g,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    let nodes: Vec<_> = graph.nodes_of_kind(&NodeKind::Community)
        .map(|n| serde_json::to_value(n).unwrap_or_default())
        .collect();
    Json(serde_json::json!({"nodes": nodes})).into_response()
}

pub async fn get_cluster(Query(params): Query<ProcessParam>) -> impl IntoResponse {
    let repo_path = match get_repo_path(&params.name).await {
        Ok(Some(p)) => p,
        Ok(None) => return (StatusCode::NOT_FOUND, "repo not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    let graph = match load_graph(&repo_path).await {
        Ok(g) => g,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    match graph.get_node(&params.id) {
        Some(node) => Json(serde_json::to_value(node).unwrap_or_default()).into_response(),
        None => (StatusCode::NOT_FOUND, "node not found").into_response(),
    }
}

// ── Analyze Jobs ──────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct AnalyzeBody {
    pub path: String,
}

pub async fn start_analyze(
    State(state): State<AppState>,
    Json(body): Json<AnalyzeBody>,
) -> impl IntoResponse {
    let job_id = Uuid::new_v4().to_string();
    let (tx, _rx) = broadcast::channel::<PipelineProgress>(100);

    {
        let mut jobs = state.analyze_jobs.lock().await;
        jobs.insert(job_id.clone(), AnalyzeJob {
            job_id: job_id.clone(),
            status: JobStatus::Running,
            progress: None,
            progress_tx: Some(tx.clone()),
        });
    }

    let repo_path = std::path::PathBuf::from(body.path);
    let state_clone = state.clone();
    let jid = job_id.clone();
    let tx_clone = tx.clone();

    tokio::spawn(async move {
        let (progress_tx, mut progress_rx) = tokio::sync::mpsc::channel::<PipelineProgress>(100);

        let state_for_progress = state_clone.clone();
        let jid_for_progress = jid.clone();
        let tx_for_broadcast = tx_clone.clone();
        tokio::spawn(async move {
            while let Some(progress) = progress_rx.recv().await {
                let _ = tx_for_broadcast.send(progress.clone());
                let mut jobs = state_for_progress.analyze_jobs.lock().await;
                if let Some(job) = jobs.get_mut(&jid_for_progress) {
                    job.progress = Some(progress);
                }
            }
        });

        let result = run_pipeline_from_repo(
            &repo_path,
            PipelineOptions::default(),
            move |progress| {
                let _ = progress_tx.blocking_send(progress);
            },
        ).await;

        let mut jobs = state_clone.analyze_jobs.lock().await;
        if let Some(job) = jobs.get_mut(&jid) {
            job.status = match result {
                Ok(_) => JobStatus::Done,
                Err(e) => JobStatus::Error(e.to_string()),
            };
        }
    });

    Json(serde_json::json!({"jobId": job_id, "status": "running"})).into_response()
}

pub async fn get_analyze_job(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> impl IntoResponse {
    let jobs = state.analyze_jobs.lock().await;
    match jobs.get(&job_id) {
        Some(job) => {
            let status_str = match &job.status {
                JobStatus::Running => "running",
                JobStatus::Done => "done",
                JobStatus::Error(_) => "error",
                JobStatus::Cancelled => "cancelled",
            };
            Json(serde_json::json!({
                "jobId": job.job_id,
                "status": status_str,
                "progress": job.progress,
            })).into_response()
        }
        None => (StatusCode::NOT_FOUND, "job not found").into_response(),
    }
}

pub async fn cancel_analyze_job(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> impl IntoResponse {
    let mut jobs = state.analyze_jobs.lock().await;
    match jobs.get_mut(&job_id) {
        Some(job) => {
            job.status = JobStatus::Cancelled;
            Json(serde_json::json!({"ok": true})).into_response()
        }
        None => (StatusCode::NOT_FOUND, "job not found").into_response(),
    }
}

pub async fn analyze_job_progress(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> impl IntoResponse {
    let rx = {
        let jobs = state.analyze_jobs.lock().await;
        jobs.get(&job_id).and_then(|j| j.progress_tx.as_ref().map(|tx| tx.subscribe()))
    };

    let Some(rx) = rx else {
        return (StatusCode::NOT_FOUND, "job not found").into_response();
    };

    let stream = BroadcastStream::new(rx)
        .filter_map(|r| r.ok())
        .map(|progress| {
            let data = serde_json::to_string(&progress).unwrap_or_default();
            Ok::<Event, Infallible>(Event::default().data(data))
        });

    Sse::new(stream).into_response()
}

// ── Embed Stubs ───────────────────────────────────────────────────────────────

pub async fn embed_stub() -> impl IntoResponse {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({"error": "embedding not supported", "supported": false})),
    )
}

pub async fn embed_job_stub() -> impl IntoResponse {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(serde_json::json!({"error": "embedding not supported", "supported": false})),
    )
}

// ── Helpers ───────────────────────────────────────────────────────────────────

async fn get_repo_path(name: &str) -> anyhow::Result<Option<std::path::PathBuf>> {
    let repos = ast_line_core::storage::repo_manager::list_repos().await?;
    Ok(repos.into_iter().find(|r| r.name == *name).map(|r| std::path::PathBuf::from(r.repo_path)))
}
