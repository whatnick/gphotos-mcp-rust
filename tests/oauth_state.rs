use std::sync::Arc;

use gphotos_mcp_rust::{auth::OAuthManager, config::Config};
use tempfile::tempdir;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

#[tokio::test]
async fn oauth_state_is_single_use() {
    let oauth_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            r#"{"access_token":"abc","refresh_token":"def","expires_in":3600,"token_type":"Bearer","scope":"openid email profile"}"#,
            "application/json",
        ))
        .expect(1)
        .mount(&oauth_server)
        .await;

    let tmp = tempdir().expect("tempdir");
    let token_path = tmp.path().join("tokens.json");

    let config = Arc::new(Config {
        host: "127.0.0.1".to_string(),
        port: 0,
        google_client_id: "test-client".to_string(),
        google_client_secret: "test-secret".to_string(),
        google_redirect_uri: "http://localhost:3000/auth/callback".to_string(),
        token_path,
        google_photos_base_url: "http://localhost:1".to_string(),
        google_photos_picker_base_url: "http://localhost:1".to_string(),
        google_oauth_base_url: oauth_server.uri(),
    });

    let manager = OAuthManager::new(config);
    let auth_url = manager.start_auth().await.expect("auth url");
    let state = auth_url
        .query_pairs()
        .find_map(|(k, v)| {
            if k == "state" {
                Some(v.to_string())
            } else {
                None
            }
        })
        .expect("state in url");

    manager
        .exchange_callback_code("good-code", &state)
        .await
        .expect("first callback succeeds");

    let second = manager.exchange_callback_code("good-code", &state).await;
    assert!(second.is_err(), "state reuse must fail");
}
