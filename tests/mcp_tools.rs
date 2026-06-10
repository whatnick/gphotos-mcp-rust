use std::{path::PathBuf, sync::Arc};

use axum::{Json, extract::State, http::StatusCode};
use gphotos_mcp_rust::{
    AppState,
    auth::OAuthManager,
    config::Config,
    mcp::{RpcRequest, mcp_entry},
    photos::GooglePhotosClient,
};
use serde_json::json;

fn test_config(token_path: PathBuf) -> Arc<Config> {
    Arc::new(Config {
        host: "127.0.0.1".to_string(),
        port: 0,
        google_client_id: "test-client".to_string(),
        google_client_secret: "test-secret".to_string(),
        google_redirect_uri: "http://localhost:3000/auth/callback".to_string(),
        token_path,
        google_photos_base_url: "http://127.0.0.1:1".to_string(),
        google_photos_picker_base_url: "http://127.0.0.1:1".to_string(),
        google_oauth_base_url: "http://127.0.0.1:1".to_string(),
    })
}

#[tokio::test]
async fn tools_list_contains_oauth_and_picker_tools() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let config = test_config(tmp.path().join("tokens.json"));
    let oauth = Arc::new(OAuthManager::new(config.clone()));
    let photos = Arc::new(GooglePhotosClient::new(config, oauth.clone()));
    let state = Arc::new(AppState { oauth, photos });

    let req = RpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "tools/list".to_string(),
        params: json!({}),
    };

    let (status, Json(resp)) = mcp_entry(State(state), Json(req)).await;
    assert_eq!(status, StatusCode::OK);

    let result = resp.result.expect("result");
    let tools = result["tools"].as_array().expect("tools array");
    let names: Vec<&str> = tools
        .iter()
        .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
        .collect();
    assert!(names.contains(&"start_auth"));
    assert!(names.contains(&"auth_status"));
    assert!(names.contains(&"create_picker_session"));
    assert!(names.contains(&"poll_picker_session"));
}

#[tokio::test]
async fn unknown_tool_returns_method_not_found_error() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let config = test_config(tmp.path().join("tokens.json"));
    let oauth = Arc::new(OAuthManager::new(config.clone()));
    let photos = Arc::new(GooglePhotosClient::new(config, oauth.clone()));
    let state = Arc::new(AppState { oauth, photos });

    let req = RpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(9)),
        method: "tools/call".to_string(),
        params: json!({"name":"does_not_exist","arguments":{}}),
    };

    let (status, Json(resp)) = mcp_entry(State(state), Json(req)).await;
    assert_eq!(status, StatusCode::OK);
    let err = resp.error.expect("error");
    assert_eq!(err.code, -32601);
}
