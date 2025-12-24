use anyhow::{Result, anyhow};
use log::{debug, error, info};
use shadowsocks_service::server::ServerBuilder;
use shadowsocks_service::shadowsocks::config::{Mode, ServerUser, ServerUserManager};
use shadowsocks_service::shadowsocks::{ServerConfig as ShadowsocksConfig, crypto::CipherKind};
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use crate::v2board::{ServerConfig, UserInfo};

/// Manages the Shadowsocks server lifecycle
pub struct ShadowsocksServerManager {
    server_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
    users: Arc<RwLock<Vec<UserInfo>>>,
    current_config: Arc<RwLock<Option<ServerConfig>>>,
    user_manager: Arc<ServerUserManager>,
}

impl ShadowsocksServerManager {
    pub fn new() -> Self {
        Self {
            server_handle: Arc::new(RwLock::new(None)),
            users: Arc::new(RwLock::new(Vec::new())),
            current_config: Arc::new(RwLock::new(None)),
            user_manager: Arc::new(ServerUserManager::new()),
        }
    }

    /// Add users to the given user manager with the specified cipher
    fn add_users_to_manager(manager: &ServerUserManager, users: &[UserInfo], cipher: Option<&str>) {
        let psw_length = if cipher == Some("2022-blake3-aes-128-gcm") {
            16
        } else {
            32
        };
        debug!("Using password length: {}", psw_length);
        for user in users.iter() {
            // UUID is used as both the user name and key for Shadowsocks 2022
            manager.add_user(ServerUser::new(&user.uuid, user.uuid.as_bytes()[..psw_length].to_vec()));
            debug!("Added user {} with UUID {}", user.id, user.uuid);
        }
    }

    /// Stop the currently running server if any
    pub async fn stop_server(&self) {
        let mut handle = self.server_handle.write().await;
        if let Some(h) = handle.take() {
            h.abort();
            let _ = h.await;
            info!("Shadowsocks server stopped");
        }
    }

    /// Start a new server with the given configuration
    pub async fn start_server(&self, config: ServerConfig) -> Result<()> {
        // Stop existing server first
        self.stop_server().await;

        info!(
            "Starting Shadowsocks server on port {} with cipher {:?}",
            config.server_port, config.cipher
        );

        // Parse cipher
        let cipher = if let Some(cipher_str) = &config.cipher {
            CipherKind::from_str(cipher_str)
                .map_err(|_| anyhow!("Invalid cipher: {}", cipher_str))?
        } else {
            return Err(anyhow!("Cipher not specified in server config"));
        };

        // Get server key
        let server_key = config
            .server_key
            .as_ref()
            .ok_or_else(|| anyhow!("Server key not specified in config"))?;
        debug!("Server key: {}", server_key);

        // Create server address - bind to all interfaces
        let listen_addr =
            SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), config.server_port as u16);

        // Create shadowsocks config
        let mut ss_config = ShadowsocksConfig::new(listen_addr, server_key.as_str(), cipher)?;
        ss_config.set_mode(Mode::TcpAndUdp);

        // Build user manager from stored users
        let users_guard = self.users.read().await;

        let manager = self.user_manager.clone();
        manager.clear_users();
        Self::add_users_to_manager(&manager, &users_guard, config.cipher.as_deref());
        drop(users_guard);

        ss_config.set_user_manager(manager.clone());

        // Build and start server
        let server = ServerBuilder::new(ss_config).build().await?;

        // Spawn server in background
        let handle = tokio::spawn(async move {
            if let Err(e) = server.run().await {
                error!("Shadowsocks server error: {}", e);
            }
        });

        // Store the handle
        let mut server_handle = self.server_handle.write().await;
        *server_handle = Some(handle);

        // Store current config
        let mut current_config = self.current_config.write().await;
        *current_config = Some(config);

        info!("Shadowsocks server started successfully");
        Ok(())
    }

    /// Update users in the server
    /// Note: Since ServerUserManager cannot be modified after the server starts,
    /// we need to restart the server with updated users
    pub async fn update_users(&self, users: Vec<UserInfo>) {
        info!("Updating {} users in Shadowsocks server", users.len());

        // Update stored users
        let mut users_list = self.users.write().await;
        *users_list = users;

        // Rebuild stored manager only if we have an active config
        let current_config = self.current_config.read().await.clone();
        if let Some(cfg) = current_config {
            let manager = self.user_manager.clone();
            manager.clear_users();
            Self::add_users_to_manager(&manager, &users_list, cfg.cipher.as_deref());
        } else {
            debug!("No active config; user manager rebuild skipped");
        }
    }
}
