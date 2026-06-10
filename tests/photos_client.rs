use std::sync::Arc;

use async_trait::async_trait;
use gphotos_mcp_rust::{auth::AccessTokenProvider, photos::GooglePhotosClient};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{header, method, path, query_param},
};

struct StaticTokenProvider;

#[async_trait]
impl AccessTokenProvider for StaticTokenProvider {
    async fn get_access_token(&self) -> anyhow::Result<String> {
        Ok("token-123".to_string())
    }

    async fn has_tokens(&self) -> anyhow::Result<bool> {
        Ok(true)
    }
}

#[tokio::test]
async fn list_albums_uses_bearer_auth_and_pagination() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/albums"))
        .and(query_param("pageSize", "50"))
        .and(query_param("pageToken", "abc"))
        .and(header("authorization", "Bearer token-123"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            r#"{"albums":[{"id":"a1","title":"album"}],"nextPageToken":"next"}"#,
            "application/json",
        ))
        .mount(&server)
        .await;

    let client = GooglePhotosClient::with_base_url(server.uri(), Arc::new(StaticTokenProvider));
    let out = client
        .list_albums(Some(50), Some("abc"))
        .await
        .expect("list albums");
    assert_eq!(out["albums"][0]["id"], "a1");
    assert_eq!(out["nextPageToken"], "next");
}

#[tokio::test]
async fn create_picker_session_uses_picker_api_base_url() {
    let library_server = MockServer::start().await;
    let picker_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/sessions"))
        .and(header("authorization", "Bearer token-123"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            r#"{"id":"session-1","pickerUri":"https://photos.google.com/picker"}"#,
            "application/json",
        ))
        .mount(&picker_server)
        .await;

    let client = GooglePhotosClient::with_base_urls(
        library_server.uri(),
        picker_server.uri(),
        Arc::new(StaticTokenProvider),
    );
    let out = client
        .create_picker_session()
        .await
        .expect("create picker session");
    assert_eq!(out["id"], "session-1");
}

#[tokio::test]
async fn poll_picker_session_uses_picker_session_and_media_endpoints() {
    let library_server = MockServer::start().await;
    let picker_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sessions/session-1"))
        .and(header("authorization", "Bearer token-123"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            r#"{"id":"session-1","mediaItemsSet":true}"#,
            "application/json",
        ))
        .mount(&picker_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/mediaItems"))
        .and(query_param("sessionId", "session-1"))
        .and(query_param("pageSize", "10"))
        .and(query_param("pageToken", "page-1"))
        .and(header("authorization", "Bearer token-123"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            r#"{"mediaItems":[{"id":"media-1"}],"nextPageToken":"next"}"#,
            "application/json",
        ))
        .mount(&picker_server)
        .await;

    let client = GooglePhotosClient::with_base_urls(
        library_server.uri(),
        picker_server.uri(),
        Arc::new(StaticTokenProvider),
    );
    let out = client
        .poll_picker_session("session-1", Some(10), Some("page-1"))
        .await
        .expect("poll picker session");
    assert_eq!(out["session"]["id"], "session-1");
    assert_eq!(out["selected_media_items"][0]["id"], "media-1");
    assert_eq!(out["nextPageToken"], "next");
}
