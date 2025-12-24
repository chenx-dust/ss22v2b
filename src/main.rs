mod manager;
mod v2board;

use async_trait::async_trait;
use log::{error, info};
use std::{error::Error, sync::Arc};

use crate::manager::ShadowsocksServerManager;
use crate::v2board::{ApiClient, ApiConfig, EventCallback, ServerConfig, UserInfo, UserTraffic};

/// Example callback implementation
struct ServerCallback {
    server_manager: Arc<ShadowsocksServerManager>,
}

impl ServerCallback {
    fn new(server_manager: Arc<ShadowsocksServerManager>) -> Self {
        Self { server_manager }
    }
}

#[async_trait]
impl EventCallback for ServerCallback {
    fn on_server_config_updated(&self, config: ServerConfig) {
        info!(
            "[Callback] Server config updated: port={}, cipher={:?}",
            config.server_port, config.cipher
        );

        let server_manager = self.server_manager.clone();
        tokio::spawn(async move {
            if let Err(e) = server_manager.start_server(config).await {
                error!("Failed to start server: {}", e);
            }
        });
    }

    fn on_users_updated(&self, users: Vec<UserInfo>) {
        info!("[Callback] Users updated: {} users", users.len());

        let server_manager = self.server_manager.clone();

        tokio::spawn(async move {
            // Update users in the Shadowsocks server
            server_manager.update_users(users.clone()).await;
        });
    }

    async fn get_traffic_data(&self) -> Option<Vec<UserTraffic>> {
        self.server_manager.collect_user_traffic().await
    }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    info!("Starting Shadowsocks V2Board server...");

    // Load configuration
    let config_str = std::fs::read_to_string("config.toml")?;
    let api_config: ApiConfig = toml::from_str(&config_str)?;
    let mut api_client = ApiClient::new(api_config)?;

    // Create server manager
    let server_manager = Arc::new(ShadowsocksServerManager::new());

    // Register callback
    let callback = Arc::new(ServerCallback::new(server_manager.clone()));
    api_client.set_callback(callback);

    info!("Starting API client...");
    api_client.run().await?;

    Ok(())
}
