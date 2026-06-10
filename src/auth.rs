use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;
use rand::{Rng, distr::Alphanumeric};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use url::Url;

use crate::config::Config;

const SCOPES: [&str; 6] = [
    "https://www.googleapis.com/auth/photoslibrary.readonly",
    "https://www.googleapis.com/auth/photoslibrary.readonly.appcreateddata",
    "https://www.googleapis.com/auth/photoslibrary.appendonly",
    "https://www.googleapis.com/auth/photoslibrary.edit.appcreateddata",
    "https://www.googleapis.com/auth/photospicker.mediaitems.readonly",
    "openid email profile",
];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: Option<String>,
    pub scope: Option<String>,
    pub expires_at_epoch: i64,
    pub retrieved_at_epoch: i64,
    pub user_email: Option<String>,
    pub user_id: Option<String>,
}

impl StoredTokens {
    pub fn is_expired(&self) -> bool {
        let now = now_epoch();
        now >= self.expires_at_epoch.saturating_sub(60)
    }
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    token_type: Option<String>,
    scope: Option<String>,
    expires_in: i64,
    id_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenInfo {
    email: Option<String>,
    sub: Option<String>,
}

#[derive(Default)]
struct AuthState {
    // state => expiry epoch
    states: HashMap<String, i64>,
}

#[derive(Clone)]
pub struct OAuthManager {
    config: Arc<Config>,
    client: Client,
    state: Arc<RwLock<AuthState>>,
}

#[async_trait]
pub trait AccessTokenProvider: Send + Sync {
    async fn get_access_token(&self) -> anyhow::Result<String>;
    async fn has_tokens(&self) -> anyhow::Result<bool>;
}

impl OAuthManager {
    pub fn new(config: Arc<Config>) -> Self {
        Self {
            config,
            client: Client::new(),
            state: Arc::new(RwLock::new(AuthState::default())),
        }
    }

    pub async fn start_auth(&self) -> anyhow::Result<Url> {
        let state = random_state();
        let expires_at = now_epoch() + Duration::from_secs(600).as_secs() as i64;

        {
            let mut lock = self.state.write().await;
            lock.states.insert(state.clone(), expires_at);
            lock.states.retain(|_, exp| *exp > now_epoch());
        }

        let mut url = Url::parse("https://accounts.google.com/o/oauth2/v2/auth")?;
        url.query_pairs_mut()
            .append_pair("response_type", "code")
            .append_pair("client_id", &self.config.google_client_id)
            .append_pair("redirect_uri", &self.config.google_redirect_uri)
            .append_pair("scope", &SCOPES.join(" "))
            .append_pair("access_type", "offline")
            .append_pair("prompt", "consent")
            .append_pair("state", &state);
        Ok(url)
    }

    pub async fn exchange_callback_code(&self, code: &str, state: &str) -> anyhow::Result<()> {
        if !self.consume_valid_state(state).await {
            anyhow::bail!("Invalid OAuth state");
        }

        let token_url = format!("{}/token", self.config.google_oauth_base_url);
        let resp = self
            .client
            .post(token_url)
            .form(&[
                ("code", code),
                ("client_id", self.config.google_client_id.as_str()),
                ("client_secret", self.config.google_client_secret.as_str()),
                ("redirect_uri", self.config.google_redirect_uri.as_str()),
                ("grant_type", "authorization_code"),
            ])
            .send()
            .await?
            .error_for_status()?
            .json::<TokenResponse>()
            .await?;

        let now = now_epoch();
        let mut stored = StoredTokens {
            access_token: resp.access_token,
            refresh_token: resp.refresh_token.unwrap_or_default(),
            token_type: resp.token_type,
            scope: resp.scope,
            expires_at_epoch: now + resp.expires_in,
            retrieved_at_epoch: now,
            user_email: None,
            user_id: None,
        };

        if let Some(id_token) = resp.id_token
            && let Ok(info) = self.fetch_token_info(&id_token).await
        {
            stored.user_email = info.email;
            stored.user_id = info.sub;
        }

        self.save_tokens(&stored).await?;
        Ok(())
    }

    async fn fetch_token_info(&self, id_token: &str) -> anyhow::Result<TokenInfo> {
        let url = format!("{}/tokeninfo", self.config.google_oauth_base_url);
        let info = self
            .client
            .post(url)
            .form(&[("id_token", id_token)])
            .send()
            .await?
            .error_for_status()?
            .json::<TokenInfo>()
            .await?;
        Ok(info)
    }

    async fn consume_valid_state(&self, state: &str) -> bool {
        let mut lock = self.state.write().await;
        let now = now_epoch();
        lock.states.retain(|_, exp| *exp > now);
        matches!(lock.states.remove(state), Some(expiry) if expiry > now)
    }

    async fn load_tokens(&self) -> anyhow::Result<Option<StoredTokens>> {
        if !self.config.token_path.exists() {
            return Ok(None);
        }
        let raw = fs::read_to_string(&self.config.token_path)?;
        let parsed = serde_json::from_str::<StoredTokens>(&raw)?;
        Ok(Some(parsed))
    }

    async fn save_tokens(&self, tokens: &StoredTokens) -> anyhow::Result<()> {
        if let Some(parent) = self.config.token_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let serialized = serde_json::to_string_pretty(tokens)?;
        fs::write(&self.config.token_path, serialized)?;
        set_strict_permissions(&self.config.token_path)?;
        Ok(())
    }

    async fn refresh_token(&self, refresh_token: &str) -> anyhow::Result<StoredTokens> {
        let token_url = format!("{}/token", self.config.google_oauth_base_url);
        let resp = self
            .client
            .post(token_url)
            .form(&[
                ("refresh_token", refresh_token),
                ("client_id", self.config.google_client_id.as_str()),
                ("client_secret", self.config.google_client_secret.as_str()),
                ("grant_type", "refresh_token"),
            ])
            .send()
            .await?
            .error_for_status()?
            .json::<TokenResponse>()
            .await?;

        let now = now_epoch();
        let current = self.load_tokens().await?;
        let refreshed = StoredTokens {
            access_token: resp.access_token,
            refresh_token: resp
                .refresh_token
                .or_else(|| current.as_ref().map(|t| t.refresh_token.clone()))
                .unwrap_or_default(),
            token_type: resp.token_type,
            scope: resp.scope,
            expires_at_epoch: now + resp.expires_in,
            retrieved_at_epoch: now,
            user_email: current.as_ref().and_then(|t| t.user_email.clone()),
            user_id: current.as_ref().and_then(|t| t.user_id.clone()),
        };
        self.save_tokens(&refreshed).await?;
        Ok(refreshed)
    }

    pub async fn auth_status(&self) -> anyhow::Result<AuthStatus> {
        let token = self.load_tokens().await?;
        match token {
            None => Ok(AuthStatus::unauthenticated()),
            Some(t) => Ok(AuthStatus {
                authenticated: true,
                expired: t.is_expired(),
                expires_at_epoch: Some(t.expires_at_epoch),
                user_email: t.user_email,
            }),
        }
    }
}

#[async_trait]
impl AccessTokenProvider for OAuthManager {
    async fn get_access_token(&self) -> anyhow::Result<String> {
        let maybe = self.load_tokens().await?;
        let Some(tokens) = maybe else {
            anyhow::bail!("Not authenticated. Run start_auth first.")
        };

        if tokens.is_expired() {
            if tokens.refresh_token.is_empty() {
                anyhow::bail!("Access token expired and no refresh token available")
            }
            let refreshed = self.refresh_token(&tokens.refresh_token).await?;
            return Ok(refreshed.access_token);
        }

        Ok(tokens.access_token)
    }

    async fn has_tokens(&self) -> anyhow::Result<bool> {
        Ok(self.load_tokens().await?.is_some())
    }
}

#[derive(Debug, Serialize)]
pub struct AuthStatus {
    pub authenticated: bool,
    pub expired: bool,
    pub expires_at_epoch: Option<i64>,
    pub user_email: Option<String>,
}

impl AuthStatus {
    fn unauthenticated() -> Self {
        Self {
            authenticated: false,
            expired: false,
            expires_at_epoch: None,
            user_email: None,
        }
    }
}

fn random_state() -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(40)
        .map(char::from)
        .collect()
}

fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(unix)]
fn set_strict_permissions(path: &PathBuf) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let metadata = fs::metadata(path)?;
    let mut perms = metadata.permissions();
    perms.set_mode(0o600);
    fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_strict_permissions(path: &PathBuf) -> anyhow::Result<()> {
    let _ = fs::metadata(path);
    Ok(())
}
