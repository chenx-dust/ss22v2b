use crate::v2board::models::{UserInfo, UserTraffic, ServerConfig};
use async_trait::async_trait;

/// Callback trait for handling events
#[async_trait]
pub trait EventCallback: Send + Sync {
    /// Called when server configuration is updated
    fn on_server_config_updated(&self, config: ServerConfig);
    
    /// Called when users are fetched or updated
    fn on_users_updated(&self, users: Vec<UserInfo>);
    
    /// Called to get traffic data for pushing. Return None to skip push.
    async fn get_traffic_data(&self) -> Option<Vec<UserTraffic>>;
}
