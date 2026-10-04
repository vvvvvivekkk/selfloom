//! Minimal MCP server over stdio.
//!
//! Speaks JSON-RPC 2.0, one message per line of stdin. Handles the handful of
//! methods a client (Claude Desktop, Claude Code, Cursor, etc.) actually calls
//! on first connection:
//!   - initialize
//!   - notifications/initialized   (no response)
//!   - tools/list
//!   - tools/call
//!
//! All other methods respond with method_not_found.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};

use crate::index::Index;
use crate::memory::{Memory, Tier};
use crate::vault::Vault;

const PROTOCOL_VERSION: &str = "2025-06-18";
const SERVER_NAME: &str = "selfloom";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn serve_stdio(vault: Vault, mut index: Index) -> Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = stdout.lock();

    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            // EOF — the client went away.
            return Ok(());
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let msg: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                let err = rpc_error(None, -32700, &format!("parse error: {e}"));
                send(&mut writer, &err)?;
                continue;
            }
        };

        if let Some(resp) = handle_message(&vault, &mut index, &msg) {
            send(&mut writer, &resp)?;
        }
    }
}

fn send<W: Write>(w: &mut W, msg: &Value) -> Result<()> {
    let line = serde_json::to_string(msg)?;
    writeln!(w, "{line}")?;
    w.flush()?;
    Ok(())
}

fn handle_message(vault: &Vault, index: &mut Index, msg: &Value) -> Option<Value> {
    let method = msg.get("method").and_then(|v| v.as_str()).unwrap_or("");
    let id = msg.get("id").cloned();
    let is_notification = id.is_none();

    // Notifications never get a response.
    if is_notification {
        // We don't need to do anything with "notifications/initialized" or similar.
        return None;
    }

    let params = msg.get("params").cloned().unwrap_or(Value::Null);

    let result = match method {
        "initialize" => Ok(initialize_result()),
        "tools/list" => Ok(tools_list_result()),
        "tools/call" => tools_call(vault, index, &params),
        "ping" => Ok(json!({})),
        other => Err(RpcError {
            code: -32601,
            message: format!("method not found: {other}"),
        }),
    };

    Some(match result {
        Ok(r) => json!({ "jsonrpc": "2.0", "id": id, "result": r }),
        Err(e) => rpc_error(id, e.code, &e.message),
    })
}

fn rpc_error(id: Option<Value>, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    })
}

#[derive(Debug)]
struct RpcError {
    code: i64,
    message: String,
}

fn initialize_result() -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": SERVER_NAME,
            "version": SERVER_VERSION
        }
    })
}

fn tools_list_result() -> Value {
    json!({
        "tools": [
            {
                "name": "search",
                "description": "Hybrid full-text search over the vault. Returns memories ranked by BM25. Use before answering any question that might depend on things the user told you earlier.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query":     { "type": "string", "description": "What to look for. Plain words work best." },
                        "limit":     { "type": "integer", "default": 10, "minimum": 1, "maximum": 50 },
                        "namespace": { "type": "string", "description": "Optional. Restrict to one namespace." },
                        "tier":      { "type": "string", "enum": ["facts", "rules", "incidents", "skills"] }
                    },
                    "required": ["query"]
                }
            },
            {
                "name": "read_memory",
                "description": "Read one memory by its path (as returned by search).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" }
                    },
                    "required": ["path"]
                }
            },
            {
                "name": "write_memory",
                "description": "Save a single memory to the vault. Use this to record something the user said that will matter later: a fact about them, a preference, a decision, a plan. Pick the right tier.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "title":     { "type": "string", "description": "Short handle, 3-8 words." },
                        "body":      { "type": "string", "description": "The memory itself, plain markdown." },
                        "tier":      { "type": "string", "enum": ["facts", "rules", "incidents", "skills"], "default": "facts" },
                        "namespace": { "type": "string", "default": "default" },
                        "tags":      { "type": "array", "items": { "type": "string" } }
                    },
                    "required": ["title", "body"]
                }
            },
            {
                "name": "list_namespaces",
                "description": "List all namespaces (separate memory spaces) in the vault.",
                "inputSchema": { "type": "object", "properties": {} }
            }
        ]
    })
}

#[derive(Deserialize)]
struct ToolCallParams {
    name: String,
    #[serde(default)]
    arguments: Value,
}

fn tools_call(vault: &Vault, index: &mut Index, params: &Value) -> std::result::Result<Value, RpcError> {
    let p: ToolCallParams = serde_json::from_value(params.clone()).map_err(|e| RpcError {
        code: -32602,
        message: format!("invalid tools/call params: {e}"),
    })?;

    match p.name.as_str() {
        "search" => tool_search(index, &p.arguments),
        "read_memory" => tool_read(vault, &p.arguments),
        "write_memory" => tool_write(vault, index, &p.arguments),
        "list_namespaces" => tool_list_namespaces(vault),
        other => Err(RpcError {
            code: -32601,
            message: format!("unknown tool: {other}"),
        }),
    }
}

fn tool_text(text: String) -> Value {
    json!({ "content": [ { "type": "text", "text": text } ] })
}

fn tool_err(msg: String) -> Value {
    json!({ "content": [ { "type": "text", "text": msg } ], "isError": true })
}

#[derive(Deserialize)]
struct SearchArgs {
    query: String,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    namespace: Option<String>,
    #[serde(default)]
    tier: Option<String>,
}

fn tool_search(index: &Index, args: &Value) -> std::result::Result<Value, RpcError> {
    let a: SearchArgs = serde_json::from_value(args.clone()).map_err(|e| RpcError {
        code: -32602,
        message: format!("bad search args: {e}"),
    })?;
    let hits = index
        .search(
            &a.query,
            a.limit.unwrap_or(10),
            a.namespace.as_deref(),
            a.tier.as_deref(),
        )
        .map_err(|e| RpcError {
            code: -32000,
            message: format!("search failed: {e}"),
        })?;

    if hits.is_empty() {
        return Ok(tool_text(format!("No memories match: {}", a.query)));
    }

    #[derive(Serialize)]
    struct OutHit {
        path: String,
        namespace: String,
        tier: String,
        title: String,
        score: f64,
        snippet: String,
    }
    let out: Vec<OutHit> = hits
        .into_iter()
        .map(|h| OutHit {
            path: h.path,
            namespace: h.namespace,
            tier: h.tier,
            title: h.title,
            score: (h.score * 1000.0).round() / 1000.0,
            snippet: h.snippet,
        })
        .collect();

    Ok(tool_text(serde_json::to_string_pretty(&out).unwrap_or_default()))
}

#[derive(Deserialize)]
struct ReadArgs {
    path: String,
}

fn tool_read(vault: &Vault, args: &Value) -> std::result::Result<Value, RpcError> {
    let a: ReadArgs = serde_json::from_value(args.clone()).map_err(|e| RpcError {
        code: -32602,
        message: format!("bad read args: {e}"),
    })?;
    match vault.read(&a.path) {
        Ok(s) => Ok(tool_text(s)),
        Err(e) => Ok(tool_err(format!("read failed: {e}"))),
    }
}

#[derive(Deserialize)]
struct WriteArgs {
    title: String,
    body: String,
    #[serde(default)]
    tier: Option<String>,
    #[serde(default)]
    namespace: Option<String>,
    #[serde(default)]
    tags: Option<Vec<String>>,
}

fn tool_write(
    vault: &Vault,
    index: &mut Index,
    args: &Value,
) -> std::result::Result<Value, RpcError> {
    let a: WriteArgs = serde_json::from_value(args.clone()).map_err(|e| RpcError {
        code: -32602,
        message: format!("bad write args: {e}"),
    })?;
    let tier = Tier::parse(a.tier.as_deref().unwrap_or("facts")).map_err(|e| RpcError {
        code: -32602,
        message: e.to_string(),
    })?;
    let ns = a.namespace.unwrap_or_else(|| "default".to_string());
    let tags = a.tags.unwrap_or_default();

    let mem = Memory::new(&ns, tier, &a.title, &a.body, tags);
    let saved = vault.write(&mem).map_err(|e| RpcError {
        code: -32000,
        message: format!("write failed: {e}"),
    })?;
    index.upsert(&saved).map_err(|e| RpcError {
        code: -32000,
        message: format!("index failed: {e}"),
    })?;

    Ok(tool_text(format!(
        "Saved to {} (id {})",
        saved.relative_path, saved.id
    )))
}

fn tool_list_namespaces(vault: &Vault) -> std::result::Result<Value, RpcError> {
    let ns = vault.list_namespaces().map_err(|e| RpcError {
        code: -32000,
        message: format!("list failed: {e}"),
    })?;
    Ok(tool_text(ns.join("\n")))
}
