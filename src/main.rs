mod v2board;

use async_trait::async_trait;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::v2board::{ApiClient, ApiConfig, EventCallback, ServerConfig, UserInfo, UserTraffic};

/// Example callback implementation
struct MyCallback {
    user_traffic: Arc<RwLock<HashMap<i32, UserTraffic>>>,
}

impl MyCallback {
    fn new() -> Self {
        Self {
            user_traffic: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl EventCallback for MyCallback {
    fn on_server_config_updated(&self, config: ServerConfig) {
        println!("[Callback] Server config updated: port={}, cipher={:?}", 
            config.server_port, config.cipher);
    }

    fn on_users_updated(&self, users: Vec<UserInfo>) {
        let user_traffic = self.user_traffic.clone();
        tokio::spawn(async move {
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let config_str = std::fs::read_to_string("config.toml")?;
    let api_config: ApiConfig = toml::from_str(&config_str)?;
    let mut api_client = ApiClient::new(api_config)?;

    // Register callback
    let callback = Arc::new(MyCallback::new());
    api_client.set_callback(callback);

    println!("Starting client...");
    api_client.run().await?;

    Ok(())
}
