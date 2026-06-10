use std::{env, path::PathBuf};

#[derive(Clone, Debug)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub google_client_id: String,
    pub google_client_secret: String,
    pub google_redirect_uri: String,
    pub token_path: PathBuf,
    pub google_photos_base_url: String,
    pub google_photos_picker_base_url: String,
    pub google_oauth_base_url: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let default_token_path = format!("{home}/.config/gphotos-mcp-rust/tokens.json");
        Ok(Self {
            host: env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: env::var("PORT")
                .ok()
                .and_then(|v| v.parse::<u16>().ok())
                .unwrap_or(3000),
            google_client_id: env::var("GOOGLE_CLIENT_ID")?,
            google_client_secret: env::var("GOOGLE_CLIENT_SECRET")?,
            google_redirect_uri: env::var("GOOGLE_REDIRECT_URI")
                .unwrap_or_else(|_| "http://localhost:3000/auth/callback".to_string()),
            token_path: env::var("TOKENS_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from(default_token_path)),
            google_photos_base_url: env::var("GOOGLE_PHOTOS_BASE_URL")
                .unwrap_or_else(|_| "https://photoslibrary.googleapis.com/v1".to_string()),
            google_photos_picker_base_url: env::var("GOOGLE_PHOTOS_PICKER_BASE_URL")
                .unwrap_or_else(|_| "https://photospicker.googleapis.com/v1".to_string()),
            google_oauth_base_url: env::var("GOOGLE_OAUTH_BASE_URL")
                .unwrap_or_else(|_| "https://oauth2.googleapis.com".to_string()),
        })
    }
}
