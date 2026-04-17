use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

impl JsonRpcResponse {
    fn ok(id: Option<Value>, result: Value) -> Self {
        Self { jsonrpc: "2.0".into(), id, result: Some(result), error: None }
    }
    fn err(id: Option<Value>, code: i32, message: impl Into<String>) -> Self {
        Self { jsonrpc: "2.0".into(), id, result: None, error: Some(JsonRpcError { code, message: message.into() }) }
    }
}

pub async fn start_mcp_server() -> Result<()> {
    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 { break; }

        let req: JsonRpcRequest = match serde_json::from_str(line.trim()) {
            Ok(r) => r,
            Err(e) => {
                let resp = JsonRpcResponse::err(None, -32700, format!("Parse error: {e}"));
                let out = serde_json::to_string(&resp)? + "\n";
                stdout.write_all(out.as_bytes()).await?;
                continue;
            }
        };

        let resp = handle_request(req).await;
        let out = serde_json::to_string(&resp)? + "\n";
        stdout.write_all(out.as_bytes()).await?;
        stdout.flush().await?;
    }

    Ok(())
}

async fn handle_request(req: JsonRpcRequest) -> JsonRpcResponse {
    match req.method.as_str() {
        "initialize" => JsonRpcResponse::ok(req.id, serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {},
                "resources": {}
            },
            "serverInfo": {
                "name": "ast-line",
                "version": "0.1.0"
            }
        })),

        "tools/list" => JsonRpcResponse::ok(req.id, serde_json::json!({
            "tools": [
                {"name": "list_repos", "description": "List all indexed repositories", "inputSchema": {"type": "object", "properties": {}}},
                {"name": "query", "description": "BM25 search over graph nodes", "inputSchema": {"type": "object", "properties": {"name": {"type": "string"}, "query": {"type": "string"}, "limit": {"type": "number"}}, "required": ["name", "query"]}},
                {"name": "search", "description": "Symbol name search", "inputSchema": {"type": "object", "properties": {"name": {"type": "string"}, "symbol": {"type": "string"}}, "required": ["name", "symbol"]}},
                {"name": "grep", "description": "File content grep", "inputSchema": {"type": "object", "properties": {"name": {"type": "string"}, "query": {"type": "string"}}, "required": ["name", "query"]}},
                {"name": "analyze_status", "description": "Get analysis status", "inputSchema": {"type": "object", "properties": {"name": {"type": "string"}}, "required": ["name"]}},
            ]
        })),

        "tools/call" => {
            let params = req.params.as_ref().and_then(|p| p.as_object());
            let tool_name = params.and_then(|p| p.get("name")).and_then(|v| v.as_str()).unwrap_or("");
            let args = params.and_then(|p| p.get("arguments")).cloned().unwrap_or_default();
            call_tool(req.id, tool_name, args).await
        }

        "resources/list" => JsonRpcResponse::ok(req.id, serde_json::json!({
            "resources": []
        })),

        "resources/read" => JsonRpcResponse::err(req.id, -32601, "resource not found"),

        _ => JsonRpcResponse::err(req.id, -32601, "Method not found"),
    }
}

async fn call_tool(id: Option<Value>, name: &str, args: Value) -> JsonRpcResponse {
    match name {
        "list_repos" => {
            match ast_line_core::storage::repo_manager::list_repos().await {
                Ok(repos) => JsonRpcResponse::ok(id, serde_json::json!({
                    "content": [{"type": "text", "text": serde_json::to_string_pretty(&repos).unwrap_or_default()}]
                })),
                Err(e) => JsonRpcResponse::err(id, -32000, e.to_string()),
            }
        }
        "query" => {
            let repo_name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;

            let repos = match ast_line_core::storage::repo_manager::list_repos().await {
                Ok(r) => r,
                Err(e) => return JsonRpcResponse::err(id, -32000, e.to_string()),
            };
            let repo = match repos.into_iter().find(|r| r.name == repo_name) {
                Some(r) => r,
                None => return JsonRpcResponse::err(id, -32000, "repo not found"),
            };
            let graph = match ast_line_core::storage::repo_manager::load_graph(std::path::Path::new(&repo.repo_path)).await {
                Ok(g) => g,
                Err(e) => return JsonRpcResponse::err(id, -32000, e.to_string()),
            };
            let index = ast_line_core::search::index_graph(&graph);
            let results = ast_line_core::search::search(&index, query, limit);
            JsonRpcResponse::ok(id, serde_json::json!({
                "content": [{"type": "text", "text": serde_json::to_string_pretty(&results.iter().map(|r| serde_json::json!({"nodeId": r.node_id, "score": r.score, "snippet": r.snippet})).collect::<Vec<_>>()).unwrap_or_default()}]
            }))
        }
        _ => JsonRpcResponse::err(id, -32601, "unknown tool"),
    }
}
