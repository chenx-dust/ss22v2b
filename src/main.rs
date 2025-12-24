mod v2board;
mod server;

use async_trait::async_trait;
use log::{error, info};
use std::{collections::HashMap, error::Error, sync::Arc};
use tokio::sync::RwLock;

use crate::v2board::{ApiClient, ApiConfig, EventCallback, ServerConfig, UserInfo, UserTraffic};
use crate::server::ShadowsocksServerManager;

/// Example callback implementation
struct ServerCallback {
    user_traffic: Arc<RwLock<HashMap<i32, UserTraffic>>>,
    server_manager: Arc<ShadowsocksServerManager>,
}

impl ServerCallback {
    fn new(server_manager: Arc<ShadowsocksServerManager>) -> Self {
        Self {
            user_traffic: Arc::new(RwLock::new(HashMap::new())),
            server_manager,
        }
    }
}

#[async_trait]
impl EventCallback for ServerCallback {
    fn on_server_config_updated(&self, config: ServerConfig) {
        info!("[Callback] Server config updated: port={}, cipher={:?}", 
            config.server_port, config.cipher);
        
        let server_manager = self.server_manager.clone();
        tokio::spawn(async move {
            if let Err(e) = server_manager.start_server(config).await {
                error!("Failed to start server: {}", e);
            }
        });
    }

    fn on_users_updated(&self, users: Vec<UserInfo>) {
        info!("[Callback] Users updated: {} users", users.len());
        
        let user_traffic = self.user_traffic.clone();
        let server_manager = self.server_manager.clone();
        
        tokio::spawn(async move {
            // Update users in the Shadowsocks server
            server_manager.update_users(users.clone()).await;
            
            // Update traffic tracking
            let mut traffic_data = user_traffic.write().await;
            
            // Initialize traffic data for new users
            for user in &users {
                traffic_data.entry(user.id).or_insert(UserTraffic {
                    id: user.id,
                    upload: 0,
                    download: 0,
                });
            }

            // Remove users not in the list
            let user_ids: Vec<i32> = users.iter().map(|u| u.id).collect();
            traffic_data.retain(|id, _| user_ids.contains(id));
        });
    }

    async fn get_traffic_data(&self) -> Option<Vec<UserTraffic>> {
        let mut traffic_data = self.user_traffic.write().await;
        if traffic_data.is_empty() {
            return None;
        }
        
        let result: Vec<UserTraffic> = traffic_data.values().cloned().collect();
        // Clear after collecting
        traffic_data.clear();
        Some(result)
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
