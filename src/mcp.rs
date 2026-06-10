use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    AppState,
    photos::{GooglePhotosClient, GooglePhotosError},
};

#[derive(Debug, Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Serialize)]
pub struct RpcResponse {
    pub jsonrpc: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

#[derive(Debug, Serialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
}

pub async fn mcp_entry(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RpcRequest>,
) -> (StatusCode, Json<RpcResponse>) {
    if req.jsonrpc != "2.0" {
        return (
            StatusCode::BAD_REQUEST,
            Json(error_response(req.id, -32600, "jsonrpc must be 2.0")),
        );
    }

    let response = match req.method.as_str() {
        "initialize" => ok_response(
            req.id,
            json!({
                "serverInfo": {
                    "name": "gphotos-mcp-rust",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "capabilities": {
                    "tools": {}
                }
            }),
        ),
        "tools/list" => ok_response(req.id, json!({ "tools": list_tools() })),
        "tools/call" => {
            match call_tool(state.photos.clone(), state.oauth.clone(), req.params).await {
                Ok(v) => ok_response(req.id, v),
                Err(err) => error_response(req.id, err.code, &err.message),
            }
        }
        _ => error_response(req.id, -32601, "Method not found"),
    };

    (StatusCode::OK, Json(response))
}

fn list_tools() -> Vec<Value> {
    vec![
        json!({"name":"auth_status","description":"Check auth status","inputSchema":{"type":"object","properties":{}}}),
        json!({"name":"start_auth","description":"Start OAuth browser flow","inputSchema":{"type":"object","properties":{}}}),
        json!({"name":"search_photos","description":"Search photos by query","inputSchema":{"type":"object","properties":{"query":{"type":"string"},"pageSize":{"type":"number"},"pageToken":{"type":"string"}},"required":["query"]}}),
        json!({"name":"search_media_by_filter","description":"Search photos by structured filters","inputSchema":{"type":"object","properties":{"filter":{"type":"object"}},"required":["filter"]}}),
        json!({"name":"get_photo","description":"Get media item details","inputSchema":{"type":"object","properties":{"photoId":{"type":"string"}},"required":["photoId"]}}),
        json!({"name":"list_albums","description":"List albums","inputSchema":{"type":"object","properties":{"pageSize":{"type":"number"},"pageToken":{"type":"string"}}}}),
        json!({"name":"get_album","description":"Get album by id","inputSchema":{"type":"object","properties":{"albumId":{"type":"string"}},"required":["albumId"]}}),
        json!({"name":"create_album","description":"Create album","inputSchema":{"type":"object","properties":{"title":{"type":"string"}},"required":["title"]}}),
        json!({"name":"list_album_photos","description":"List album photos","inputSchema":{"type":"object","properties":{"albumId":{"type":"string"},"pageSize":{"type":"number"},"pageToken":{"type":"string"}},"required":["albumId"]}}),
        json!({"name":"list_media_items","description":"List media items","inputSchema":{"type":"object","properties":{"pageSize":{"type":"number"},"pageToken":{"type":"string"}}}}),
        json!({"name":"create_picker_session","description":"Create Google Photos picker session","inputSchema":{"type":"object","properties":{}}}),
        json!({"name":"poll_picker_session","description":"Poll picker session and fetch selected media items","inputSchema":{"type":"object","properties":{"sessionId":{"type":"string"},"pageSize":{"type":"number"},"pageToken":{"type":"string"}},"required":["sessionId"]}}),
    ]
}

#[derive(Debug)]
struct ToolError {
    code: i32,
    message: String,
}

async fn call_tool(
    photos: Arc<GooglePhotosClient>,
    oauth: Arc<crate::auth::OAuthManager>,
    params: Value,
) -> Result<Value, ToolError> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| ToolError {
            code: -32602,
            message: "Missing tool name".to_string(),
        })?;
    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| Value::Object(Default::default()));

    match name {
        "auth_status" => {
            let status = oauth.auth_status().await.map_err(internal_err)?;
            Ok(
                json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&status).map_err(internal_err)? }]}),
            )
        }
        "start_auth" => {
            let auth_url = oauth.start_auth().await.map_err(internal_err)?;
            Ok(
                json!({"content":[{"type":"text","text":format!("Open this URL to authenticate:\n{}", auth_url)}],"authUrl": auth_url.to_string()}),
            )
        }
        "search_photos" => {
            let query = required_string(&args, "query")?;
            let page_size = optional_u32(&args, "pageSize");
            let page_token = optional_string(&args, "pageToken");
            let out = photos
                .search_photos(&query, page_size, page_token.as_deref())
                .await
                .map_err(map_gp_err)?;
            Ok(
                json!({"content":[{"type":"text","text":serde_json::to_string_pretty(&out).map_err(internal_err)?}]}),
            )
        }
        "search_media_by_filter" => {
            let filter = args.get("filter").cloned().ok_or_else(|| ToolError {
                code: -32602,
                message: "Missing filter object".to_string(),
            })?;
            let out = photos
                .search_media_by_filter(filter)
                .await
                .map_err(map_gp_err)?;
            Ok(
                json!({"content":[{"type":"text","text":serde_json::to_string_pretty(&out).map_err(internal_err)?}]}),
            )
        }
        "get_photo" => {
            let photo_id = required_string(&args, "photoId")?;
            let out = photos.get_photo(&photo_id).await.map_err(map_gp_err)?;
            Ok(
                json!({"content":[{"type":"text","text":serde_json::to_string_pretty(&out).map_err(internal_err)?}]}),
            )
        }
        "list_albums" => {
            let out = photos
                .list_albums(
                    optional_u32(&args, "pageSize"),
                    optional_string(&args, "pageToken").as_deref(),
                )
                .await
                .map_err(map_gp_err)?;
            Ok(
                json!({"content":[{"type":"text","text":serde_json::to_string_pretty(&out).map_err(internal_err)?}]}),
            )
        }
        "get_album" => {
            let album_id = required_string(&args, "albumId")?;
            let out = photos.get_album(&album_id).await.map_err(map_gp_err)?;
            Ok(
                json!({"content":[{"type":"text","text":serde_json::to_string_pretty(&out).map_err(internal_err)?}]}),
            )
        }
        "create_album" => {
            let title = required_string(&args, "title")?;
            let out = photos.create_album(&title).await.map_err(map_gp_err)?;
            Ok(
                json!({"content":[{"type":"text","text":serde_json::to_string_pretty(&out).map_err(internal_err)?}]}),
            )
        }
        "list_album_photos" => {
            let album_id = required_string(&args, "albumId")?;
            let out = photos
                .list_album_photos(
                    &album_id,
                    optional_u32(&args, "pageSize"),
                    optional_string(&args, "pageToken").as_deref(),
                )
                .await
                .map_err(map_gp_err)?;
            Ok(
                json!({"content":[{"type":"text","text":serde_json::to_string_pretty(&out).map_err(internal_err)?}]}),
            )
        }
        "list_media_items" => {
            let out = photos
                .list_media_items(
                    optional_u32(&args, "pageSize"),
                    optional_string(&args, "pageToken").as_deref(),
                )
                .await
                .map_err(map_gp_err)?;
            Ok(
                json!({"content":[{"type":"text","text":serde_json::to_string_pretty(&out).map_err(internal_err)?}]}),
            )
        }
        "create_picker_session" => {
            let out = photos.create_picker_session().await.map_err(map_gp_err)?;
            Ok(
                json!({"content":[{"type":"text","text":serde_json::to_string_pretty(&out).map_err(internal_err)?}]}),
            )
        }
        "poll_picker_session" => {
            let session_id = required_string(&args, "sessionId")?;
            let out = photos
                .poll_picker_session(
                    &session_id,
                    optional_u32(&args, "pageSize"),
                    optional_string(&args, "pageToken").as_deref(),
                )
                .await
                .map_err(map_gp_err)?;
            Ok(
                json!({"content":[{"type":"text","text":serde_json::to_string_pretty(&out).map_err(internal_err)?}]}),
            )
        }
        _ => Err(ToolError {
            code: -32601,
            message: format!("Unknown tool: {name}"),
        }),
    }
}

fn map_gp_err(err: GooglePhotosError) -> ToolError {
    match err {
        GooglePhotosError::NotAuthenticated => ToolError {
            code: -32001,
            message: "Not authenticated. Use start_auth and complete /auth/callback.".to_string(),
        },
        GooglePhotosError::ApiError { status, message } => ToolError {
            code: -32010,
            message: format!("Google Photos API error ({status}): {message}"),
        },
        GooglePhotosError::Http(e) => internal_err(e),
        GooglePhotosError::Json(e) => internal_err(e),
        GooglePhotosError::Unexpected(e) => internal_err(e),
        GooglePhotosError::InvalidInput(msg) => ToolError {
            code: -32602,
            message: format!("Invalid input: {msg}"),
        },
    }
}

fn internal_err<E: std::fmt::Display>(err: E) -> ToolError {
    ToolError {
        code: -32000,
        message: format!("Internal error: {err}"),
    }
}

fn ok_response(id: Option<Value>, result: Value) -> RpcResponse {
    RpcResponse {
        jsonrpc: "2.0",
        id,
        result: Some(result),
        error: None,
    }
}

fn error_response(id: Option<Value>, code: i32, message: &str) -> RpcResponse {
    RpcResponse {
        jsonrpc: "2.0",
        id,
        result: None,
        error: Some(RpcError {
            code,
            message: message.to_string(),
        }),
    }
}

fn required_string(args: &Value, key: &str) -> Result<String, ToolError> {
    args.get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| ToolError {
            code: -32602,
            message: format!("Missing required argument: {key}"),
        })
}

fn optional_string(args: &Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn optional_u32(args: &Value, key: &str) -> Option<u32> {
    args.get(key).and_then(Value::as_u64).map(|n| n as u32)
}
