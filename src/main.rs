mod config;
mod manager;
mod v2board;

use async_trait::async_trait;
use clap::Parser;
use log::{debug, info};
use std::{error::Error, sync::Arc};

use crate::config::Config;
use crate::manager::ShadowsocksServerManager;
use crate::v2board::{ApiClient, EventCallback, ServerConfig, UserInfo, UserTraffic};

/// Command line arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the configuration file
    #[arg(short, long, default_value = "config.toml")]
    config: String,
}

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
                panic!("Failed to start server: {}", e);
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

    // Parse command line arguments
    let args = Args::parse();

    info!("Starting Shadowsocks V2Board server...");
    info!("Loading configuration from: {}", args.config);

    // Load configuration
    let config = Config::load_from_file(&args.config)?;
    
    debug!("Shadowsocks settings: {:?}", config.shadowsocks);
    
    // Create API client
    let mut api_client = ApiClient::new(config.api.clone())?;

    // Create server manager with shadowsocks config
    let server_manager = Arc::new(ShadowsocksServerManager::new(config.shadowsocks.clone()));

    // Register callback
    let callback = Arc::new(ServerCallback::new(server_manager.clone()));
    api_client.set_callback(callback);

    info!("Starting API client...");
    api_client.run().await?;

    Ok(())
}
