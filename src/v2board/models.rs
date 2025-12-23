use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: i32,
    pub uuid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserTraffic {
    pub id: i32,
    pub upload: i64,
    pub download: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub api_host: String,
    pub node_id: i32,
    pub key: String,
    pub timeout: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub server_port: u32,
    pub cipher: Option<String>,
    #[serde(rename = "server_key")]
    pub server_key: Option<String>,
    pub base_config: Option<BaseConfig>
}

#[derive(Debug, Clone, Deserialize)]
pub struct BaseConfig {
    pub push_interval: Option<u32>,
    pub pull_interval: Option<u32>,
}
