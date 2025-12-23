use anyhow::{Result, anyhow};
use reqwest::{Client, Response};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;

use crate::v2board::models::{ApiConfig, ServerConfig, UserInfo, UserTraffic};
use crate::v2board::callback::EventCallback;

pub struct ApiClient {
    client: Client,
    api_host: String,
    node_id: i32,
    key: String,
    server_config: Arc<RwLock<Option<ServerConfig>>>,
    etags: Arc<RwLock<HashMap<String, String>>>,
    callback: Option<Arc<dyn EventCallback>>,
}

impl ApiClient {
    pub fn new(config: ApiConfig) -> Result<Self> {
        let timeout = if config.timeout > 0 {
            Duration::from_secs(config.timeout)
        } else {
            Duration::from_secs(5)
        };

        let client = Client::builder().timeout(timeout).build()?;

        Ok(ApiClient {
            client,
            api_host: config.api_host.clone(),
            node_id: config.node_id,
            key: config.key.clone(),
            server_config: Arc::new(RwLock::new(None)),
            etags: Arc::new(RwLock::new(HashMap::new())),
            callback: None,
        })
    }

    fn assemble_url(&self, path: &str) -> String {
        format!("{}{}", self.api_host, path)
    }

    fn build_query_params(&self) -> HashMap<String, String> {
        let mut params = HashMap::new();
        params.insert("node_id".to_string(), self.node_id.to_string());
        params.insert("node_type".to_string(), "shadowsocks".to_string());
        params.insert("token".to_string(), self.key.clone());
        params
    }

    async fn parse_response(&self, res: Response, path: &str) -> Result<Value> {
        let status = res.status();

        if status.as_u16() > 399 {
            let body = res.text().await?;
            return Err(anyhow!(
                "request {} failed: status {}, body: {}",
                self.assemble_url(path),
                status,
                body
            ));
        }

        let body = res.json::<Value>().await?;
        Ok(body)
    }

    pub async fn get_node_info(&self) -> Result<ServerConfig> {
        let path = "/api/v1/server/UniProxy/config";
        let url = self.assemble_url(path);

        let etags = self.etags.read().await;
        let etag = etags.get("node").cloned();
        drop(etags);

        let mut request = self.client.get(&url).query(&self.build_query_params());

        if let Some(etag) = etag {
            request = request.header("If-None-Match", etag);
        }

        let res = request.send().await?;

        if res.status().as_u16() == 304 {
            return Err(anyhow!("Node not modified"));
        }

        if let Some(new_etag) = res.headers().get("etag") {
            if let Ok(etag_str) = new_etag.to_str() {
                let mut etags = self.etags.write().await;
                etags.insert("node".to_string(), etag_str.to_string());
            }
        }

        let json_data = self.parse_response(res, path).await?;
        let server: ServerConfig = serde_json::from_value(json_data)?;

        if server.server_port == 0 {
            return Err(anyhow!("server port must > 0"));
        }

        let mut config = self.server_config.write().await;
        *config = Some(server.clone());
        drop(config);

        Ok(server)
    }

    pub async fn get_server_config(&self) -> Option<ServerConfig> {
        let config = self.server_config.read().await;
        config.clone()
    }

    pub async fn get_user_list(&self) -> Result<Vec<UserInfo>> {
        let path = "/api/v1/server/UniProxy/user";
        let url = self.assemble_url(path);

        let etags = self.etags.read().await;
        let etag = etags.get("users").cloned();
        drop(etags);

        let mut request = self.client.get(&url).query(&self.build_query_params());

        if let Some(etag) = etag {
            request = request.header("If-None-Match", etag);
        }

        let res = request.send().await?;

        if res.status().as_u16() == 304 {
            return Err(anyhow!("User not modified"));
        }

        if let Some(new_etag) = res.headers().get("etag") {
            if let Ok(etag_str) = new_etag.to_str() {
                let mut etags = self.etags.write().await;
                etags.insert("users".to_string(), etag_str.to_string());
            }
        }

        let json_data = self.parse_response(res, path).await?;
        let users: Vec<UserInfo> = serde_json::from_value(
            json_data
                .get("users")
                .ok_or_else(|| anyhow!("users field not found"))?
                .clone(),
        )?;

        if users.is_empty() {
            return Err(anyhow!("users is null"));
        }

        Ok(users)
    }

    pub async fn report_user_traffic(&self, user_traffic: &[UserTraffic]) -> Result<()> {
        let path = "/api/v1/server/UniProxy/push";
        let url = self.assemble_url(path);

        let mut data: HashMap<i32, Vec<i64>> = HashMap::new();
        for traffic in user_traffic {
            data.insert(traffic.id, vec![traffic.upload, traffic.download]);
        }

        let res = self
            .client
            .post(&url)
            .query(&self.build_query_params())
            .json(&data)
            .send()
            .await?;

        self.parse_response(res, path).await?;
        Ok(())
    }

    /// Set the event callback
    pub fn set_callback(&mut self, callback: Arc<dyn EventCallback>) {
        self.callback = Some(callback);
    }

    /// Start the client and run continuously
    pub async fn run(&self) -> Result<()> {
        println!("Fetching node configuration...");
        
        // First time fetching node config
        let server_config = self.get_node_info().await?;

        // Get interval configuration, use default values if not set
        let (pull_interval_secs, push_interval_secs) = if let Some(base_config) = &server_config.base_config {
            let pull = base_config.pull_interval.unwrap_or(60);
            let push = base_config.push_interval.unwrap_or(60);
            (pull as u64, push as u64)
        } else {
            (60u64, 60u64)
        };

        println!("Pull interval: {}s, Push interval: {}s", pull_interval_secs, push_interval_secs);

        // Create scheduled tasks
        let pull_task = self.pull_task(pull_interval_secs);
        let push_task = self.push_task(push_interval_secs);

        // Run both tasks concurrently
        tokio::try_join!(pull_task, push_task)?;

        Ok(())
    }

    /// Periodically pull user list and node configuration
    async fn pull_task(&self, interval_secs: u64) -> Result<()> {
        let mut ticker = interval(Duration::from_secs(interval_secs));
        
        loop {
            ticker.tick().await;
            
            println!("[Pull] Fetching user list...");
            match self.get_user_list().await {
                Ok(users) => {
                    println!("[Pull] Fetched {} users", users.len());
                    if let Some(callback) = &self.callback {
                        callback.on_users_updated(users);
                    }
                }
                Err(e) => {
                    if e.to_string().contains("not modified") {
                        println!("[Pull] User list not modified");
                    } else {
                        eprintln!("[Pull] Failed to fetch user list: {}", e);
                    }
                }
            }

            // Try to update node configuration
            match self.get_node_info().await {
                Ok(config) => {
                    println!("[Pull] Node configuration updated: {:?}", config);
                    if let Some(callback) = &self.callback {
                        callback.on_server_config_updated(config);
                    }
                }
                Err(e) => {
                    if e.to_string().contains("not modified") {
                        println!("[Pull] Node configuration not modified");
                    } else {
                        eprintln!("[Pull] Failed to fetch node configuration: {}", e);
                    }
                }
            }
        }
    }

    /// Periodically push user traffic data
    async fn push_task(&self, interval_secs: u64) -> Result<()> {
        let mut ticker = interval(Duration::from_secs(interval_secs));
        
        loop {
            ticker.tick().await;
            
            if let Some(callback) = &self.callback {
                if let Some(traffic_vec) = callback.get_traffic_data().await {
                    if traffic_vec.is_empty() {
                        println!("[Push] No traffic data to push");
                        continue;
                    }

                    println!("[Push] Pushing traffic data for {} users...", traffic_vec.len());
                    match self.report_user_traffic(&traffic_vec).await {
                        Ok(_) => {
                            println!("[Push] Traffic data pushed successfully");
                        }
                        Err(e) => {
                            eprintln!("[Push] Failed to push traffic data: {}", e);
                        }
                    }
                } else {
                    println!("[Push] No traffic data to push");
                }
            } else {
                println!("[Push] No callback registered");
            }
        }
    }
}
