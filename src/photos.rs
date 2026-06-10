use std::sync::Arc;

use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use url::Url;

use crate::auth::AccessTokenProvider;
use crate::config::Config;

#[derive(thiserror::Error, Debug)]
pub enum GooglePhotosError {
    #[error("authentication required")]
    NotAuthenticated,
    #[error("api error ({status}): {message}")]
    ApiError { status: u16, message: String },
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

#[derive(Clone)]
pub struct GooglePhotosClient {
    http: Client,
    base_url: String,
    picker_base_url: String,
    token_provider: Arc<dyn AccessTokenProvider>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SearchResponse {
    #[serde(default)]
    pub media_items: Vec<Value>,
    #[serde(default)]
    pub next_page_token: Option<String>,
}

impl GooglePhotosClient {
    pub fn new(config: Arc<Config>, token_provider: Arc<dyn AccessTokenProvider>) -> Self {
        Self {
            http: Client::new(),
            base_url: config.google_photos_base_url.clone(),
            picker_base_url: config.google_photos_picker_base_url.clone(),
            token_provider,
        }
    }

    pub fn with_base_url(base_url: String, token_provider: Arc<dyn AccessTokenProvider>) -> Self {
        Self {
            http: Client::new(),
            base_url: base_url.clone(),
            picker_base_url: base_url,
            token_provider,
        }
    }

    pub fn with_base_urls(
        base_url: String,
        picker_base_url: String,
        token_provider: Arc<dyn AccessTokenProvider>,
    ) -> Self {
        Self {
            http: Client::new(),
            base_url,
            picker_base_url,
            token_provider,
        }
    }

    async fn authed_get(&self, path: &str) -> Result<Value, GooglePhotosError> {
        let url = format!("{}{}", self.base_url, path);
        self.authed_get_full(&url).await
    }

    async fn authed_get_full(&self, url: &str) -> Result<Value, GooglePhotosError> {
        let token = self
            .token_provider
            .get_access_token()
            .await
            .map_err(|_| GooglePhotosError::NotAuthenticated)?;
        let resp = self.http.get(url).bearer_auth(token).send().await?;
        map_response(resp).await
    }

    async fn authed_post(&self, path: &str, body: Value) -> Result<Value, GooglePhotosError> {
        let token = self
            .token_provider
            .get_access_token()
            .await
            .map_err(|_| GooglePhotosError::NotAuthenticated)?;
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .http
            .post(url)
            .bearer_auth(token)
            .json(&body)
            .send()
            .await?;
        map_response(resp).await
    }

    async fn authed_picker_get(&self, path: &str) -> Result<Value, GooglePhotosError> {
        let url = format!("{}{}", self.picker_base_url, path);
        self.authed_picker_get_full(&url).await
    }

    async fn authed_picker_get_full(&self, url: &str) -> Result<Value, GooglePhotosError> {
        let token = self
            .token_provider
            .get_access_token()
            .await
            .map_err(|_| GooglePhotosError::NotAuthenticated)?;
        let resp = self.http.get(url).bearer_auth(token).send().await?;
        map_response(resp).await
    }

    async fn authed_picker_post(
        &self,
        path: &str,
        body: Value,
    ) -> Result<Value, GooglePhotosError> {
        let token = self
            .token_provider
            .get_access_token()
            .await
            .map_err(|_| GooglePhotosError::NotAuthenticated)?;
        let url = format!("{}{}", self.picker_base_url, path);
        let resp = self
            .http
            .post(url)
            .bearer_auth(token)
            .json(&body)
            .send()
            .await?;
        map_response(resp).await
    }

    pub async fn list_albums(
        &self,
        page_size: Option<u32>,
        page_token: Option<&str>,
    ) -> Result<Value, GooglePhotosError> {
        let path = build_paginated_path(&self.base_url, "/albums", page_size, page_token)?;
        self.authed_get_full(&path).await
    }

    pub async fn get_album(&self, album_id: &str) -> Result<Value, GooglePhotosError> {
        validate_id(album_id, "albumId")?;
        self.authed_get(&format!("/albums/{album_id}")).await
    }

    pub async fn list_media_items(
        &self,
        page_size: Option<u32>,
        page_token: Option<&str>,
    ) -> Result<Value, GooglePhotosError> {
        let path = build_paginated_path(&self.base_url, "/mediaItems", page_size, page_token)?;
        self.authed_get_full(&path).await
    }

    pub async fn get_photo(&self, media_item_id: &str) -> Result<Value, GooglePhotosError> {
        validate_id(media_item_id, "mediaItemId")?;
        self.authed_get(&format!("/mediaItems/{media_item_id}"))
            .await
    }

    pub async fn search_photos(
        &self,
        query: &str,
        page_size: Option<u32>,
        page_token: Option<&str>,
    ) -> Result<SearchResponse, GooglePhotosError> {
        let mut body = json!({
            "pageSize": page_size.unwrap_or(25),
            "filters": {
                "contentFilter": {
                    "includedContentCategories": ["NONE"]
                }
            },
            "language": "en-US",
        });
        body["pageSize"] = json!(page_size.unwrap_or(25));
        body["filters"] = json!({
            "featureFilter": {
                "includedFeatures": [query]
            }
        });
        if let Some(token) = page_token {
            body["pageToken"] = json!(token);
        }
        let raw = self.authed_post("/mediaItems:search", body).await?;
        Ok(SearchResponse {
            media_items: raw
                .get("mediaItems")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default(),
            next_page_token: raw
                .get("nextPageToken")
                .and_then(|v| v.as_str())
                .map(ToString::to_string),
        })
    }

    pub async fn search_media_by_filter(
        &self,
        filter_body: Value,
    ) -> Result<Value, GooglePhotosError> {
        self.authed_post("/mediaItems:search", filter_body).await
    }

    pub async fn list_album_photos(
        &self,
        album_id: &str,
        page_size: Option<u32>,
        page_token: Option<&str>,
    ) -> Result<Value, GooglePhotosError> {
        let mut body = json!({
            "albumId": album_id,
            "pageSize": page_size.unwrap_or(25),
        });
        if let Some(token) = page_token {
            body["pageToken"] = json!(token);
        }
        self.authed_post("/mediaItems:search", body).await
    }

    pub async fn create_album(&self, title: &str) -> Result<Value, GooglePhotosError> {
        self.authed_post("/albums", json!({ "album": { "title": title } }))
            .await
    }

    pub async fn create_picker_session(&self) -> Result<Value, GooglePhotosError> {
        self.authed_picker_post("/sessions", json!({})).await
    }

    pub async fn poll_picker_session(
        &self,
        session_id: &str,
        page_size: Option<u32>,
        page_token: Option<&str>,
    ) -> Result<Value, GooglePhotosError> {
        validate_id(session_id, "sessionId")?;
        let session = self
            .authed_picker_get(&format!("/sessions/{session_id}"))
            .await?;
        let items_path =
            build_picker_items_path(&self.picker_base_url, session_id, page_size, page_token)?;
        let items = self.authed_picker_get_full(&items_path).await?;
        Ok(json!({
            "session": session,
            "selected_media_items": items.get("mediaItems").cloned().unwrap_or_else(|| json!([])),
            "nextPageToken": items.get("nextPageToken").cloned().unwrap_or(Value::Null)
        }))
    }
}

fn validate_id(id: &str, field: &str) -> Result<(), GooglePhotosError> {
    if id.is_empty() {
        return Err(GooglePhotosError::InvalidInput(format!(
            "{field} must not be empty"
        )));
    }
    if id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err(GooglePhotosError::InvalidInput(format!(
            "{field} contains forbidden characters"
        )));
    }
    Ok(())
}

fn build_paginated_path(
    base_url: &str,
    path: &str,
    page_size: Option<u32>,
    page_token: Option<&str>,
) -> Result<String, GooglePhotosError> {
    let full = format!("{base_url}{path}");
    let mut url = Url::parse(&full)
        .map_err(|e| GooglePhotosError::InvalidInput(format!("Invalid base URL: {e}")))?;
    url.query_pairs_mut()
        .append_pair("pageSize", &page_size.unwrap_or(25).to_string());
    if let Some(token) = page_token {
        url.query_pairs_mut().append_pair("pageToken", token);
    }
    Ok(url.to_string())
}

fn build_picker_items_path(
    picker_base_url: &str,
    session_id: &str,
    page_size: Option<u32>,
    page_token: Option<&str>,
) -> Result<String, GooglePhotosError> {
    let full = format!("{picker_base_url}/mediaItems");
    let mut url = Url::parse(&full)
        .map_err(|e| GooglePhotosError::InvalidInput(format!("Invalid picker base URL: {e}")))?;
    url.query_pairs_mut()
        .append_pair("sessionId", session_id)
        .append_pair("pageSize", &page_size.unwrap_or(25).to_string());
    if let Some(token) = page_token {
        url.query_pairs_mut().append_pair("pageToken", token);
    }
    Ok(url.to_string())
}

async fn map_response(resp: reqwest::Response) -> Result<Value, GooglePhotosError> {
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if status.is_success() {
        if text.is_empty() {
            return Ok(json!({}));
        }
        let parsed = serde_json::from_str::<Value>(&text)?;
        return Ok(parsed);
    }

    if status == StatusCode::UNAUTHORIZED {
        return Err(GooglePhotosError::NotAuthenticated);
    }

    Err(GooglePhotosError::ApiError {
        status: status.as_u16(),
        message: text,
    })
}
