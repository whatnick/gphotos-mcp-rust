use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Query, State},
    response::IntoResponse,
    routing::{get, post},
};
use gphotos_mcp_rust::{
    AppState,
    auth::OAuthManager,
    config::Config,
    mcp::{RpcResponse, mcp_entry},
    photos::GooglePhotosClient,
};
use serde::Deserialize;
use serde_json::json;
use tower_http::trace::TraceLayer;
use tracing::info;

#[derive(Debug, Deserialize)]
struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,gphotos_mcp_rust=debug".into()),
        )
        .init();

    let config = Arc::new(Config::from_env()?);
    let oauth = Arc::new(OAuthManager::new(config.clone()));
    let photos = Arc::new(GooglePhotosClient::new(config.clone(), oauth.clone()));
    let app_state = Arc::new(AppState { oauth, photos });

    let app = Router::new()
        .route("/", get(index))
        .route("/health", get(health))
        .route("/auth/start", get(auth_start))
        .route("/auth/callback", get(auth_callback))
        .route("/mcp", post(mcp_entry))
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("gphotos-mcp-rust listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn index() -> impl IntoResponse {
    Json(json!({
        "name": "gphotos-mcp-rust",
        "description": "Rust Google Photos MCP bootstrap server",
        "routes": {
            "auth_start": "/auth/start",
            "auth_callback": "/auth/callback",
            "mcp": "/mcp",
            "health": "/health"
        }
    }))
}

async fn health() -> impl IntoResponse {
    Json(json!({ "status": "ok" }))
}

async fn auth_start(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.oauth.start_auth().await {
        Ok(auth_url) => Json(json!({
            "auth_url": auth_url.to_string(),
            "instruction": "Open auth_url in a browser, then Google redirects to /auth/callback."
        }))
        .into_response(),
        Err(err) => Json(error_json(
            -32000,
            format!("Could not start OAuth flow: {err}"),
        ))
        .into_response(),
    }
}

async fn auth_callback(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CallbackQuery>,
) -> impl IntoResponse {
    let Some(code) = query.code else {
        return Json(error_json(
            -32602,
            "Missing query parameter: code".to_string(),
        ))
        .into_response();
    };
    let Some(csrf_state) = query.state else {
        return Json(error_json(
            -32602,
            "Missing query parameter: state".to_string(),
        ))
        .into_response();
    };

    match state.oauth.exchange_callback_code(&code, &csrf_state).await {
        Ok(_) => Json(json!({
            "status": "authenticated",
            "message": "OAuth tokens saved. You can now use MCP tools."
        }))
        .into_response(),
        Err(err) => {
            Json(error_json(-32001, format!("OAuth callback failed: {err}"))).into_response()
        }
    }
}

fn error_json(code: i32, message: String) -> RpcResponse {
    RpcResponse {
        jsonrpc: "2.0",
        id: None,
        result: None,
        error: Some(gphotos_mcp_rust::mcp::RpcError { code, message }),
    }
}
