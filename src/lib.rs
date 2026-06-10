pub mod auth;
pub mod config;
pub mod mcp;
pub mod photos;

use std::sync::Arc;

use auth::OAuthManager;
use photos::GooglePhotosClient;

#[derive(Clone)]
pub struct AppState {
    pub oauth: Arc<OAuthManager>,
    pub photos: Arc<GooglePhotosClient>,
}
