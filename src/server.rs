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
        // Take the handle out so we don't hold the lock while awaiting
        let handle = {
            let mut guard = self.server_handle.write().await;
            guard.take()
        };

        if let Some(h) = handle {
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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    fn make_users(n: usize) -> Vec<UserInfo> {
        (0..n)
            .map(|i| UserInfo {
                id: i as i32,
                // Ensure UUID string is long enough (> 32 bytes)
                uuid: format!("{}-aaaaaaaa-aaaaaaaa-aaaaaaaa-aaaaaaaa", i),
            })
            .collect()
    }

    #[tokio::test]
    async fn test_add_users_password_length_16_for_2022_128() {
        let manager = ServerUserManager::new();
        let users = make_users(3);

        // Call private helper within the same module
        ShadowsocksServerManager::add_users_to_manager(&manager, &users, Some("2022-blake3-aes-128-gcm"));

        assert_eq!(manager.user_count(), users.len());
        for u in manager.users_iter() {
            assert_eq!(u.key().len(), 16, "expected key length 16 for 2022-128 cipher");
        }
    }

    #[tokio::test]
    async fn test_add_users_password_length_32_for_other_ciphers() {
        let manager = ServerUserManager::new();
        let users = make_users(2);

        // Use a different cipher which should fall back to 32
        ShadowsocksServerManager::add_users_to_manager(&manager, &users, Some("2022-blake3-aes-256-gcm"));

        assert_eq!(manager.user_count(), users.len());
        for u in manager.users_iter() {
            assert_eq!(u.key().len(), 32, "expected key length 32 for non-2022-128 cipher");
        }
    }

    #[tokio::test]
    async fn test_update_users_without_active_config_does_not_touch_manager() {
        let mgr = ShadowsocksServerManager::new();
        // Ensure manager starts empty
        assert_eq!(mgr.user_manager.user_count(), 0);

        let new_users = make_users(5);
        mgr.update_users(new_users).await;

        // Still empty because there is no active config set
        assert_eq!(mgr.user_manager.user_count(), 0);
    }

    #[tokio::test]
    async fn test_update_users_with_active_config_rebuilds_manager() {
        let mgr = ShadowsocksServerManager::new();

        // Seed current_config to simulate an active server configuration
        {
            let mut guard = mgr.current_config.write().await;
            *guard = Some(ServerConfig {
                server_port: 0, // not used in update path
                cipher: Some("2022-blake3-aes-128-gcm".to_string()),
                server_key: Some("dummy-key".to_string()),
                base_config: None,
            });
        }

        // Pre-populate manager with different users to ensure it gets cleared
        let pre_users = make_users(2);
        ShadowsocksServerManager::add_users_to_manager(&mgr.user_manager, &pre_users, Some("2022-blake3-aes-256-gcm"));
        assert_eq!(mgr.user_manager.user_count(), 2);

        // Now update with a new set; should clear and add using active config's cipher (128 -> 16 bytes keys)
        let new_users = make_users(4);
        mgr.update_users(new_users.clone()).await;

        assert_eq!(mgr.user_manager.user_count(), new_users.len());
        for u in mgr.user_manager.users_iter() {
            assert_eq!(u.key().len(), 16, "expected key length 16 per active config cipher");
        }
    }

    #[tokio::test]
    async fn test_start_server_initializes_handle_and_user_manager() {
        let mgr = ShadowsocksServerManager::new();

        // Preload users before starting the server
        {
            let mut guard = mgr.users.write().await;
            *guard = make_users(2);
        }

        let cfg = ServerConfig {
            server_port: 0, // let OS pick an ephemeral port
            cipher: Some("2022-blake3-aes-128-gcm".to_string()),
            // 16-byte base64 key
            server_key: Some("YWJjZGVmZ2hpamtsbW5vcA==".to_string()),
            base_config: None,
        };

        mgr.start_server(cfg.clone()).await.expect("server should start");

        // server_handle set
        assert!(mgr.server_handle.read().await.is_some(), "server handle should exist after start");

        // current_config stored
        let stored = mgr.current_config.read().await;
        let stored_cfg = stored.as_ref().expect("config should be stored");
        assert_eq!(stored_cfg.server_port, cfg.server_port);
        assert_eq!(stored_cfg.cipher, cfg.cipher);
        assert_eq!(stored_cfg.server_key, cfg.server_key);

        // user manager built with correct key length
        assert_eq!(mgr.user_manager.user_count(), 2);
        for u in mgr.user_manager.users_iter() {
            assert_eq!(u.key().len(), 16, "expected key length 16 for 2022-128 cipher");
        }

        // Cleanup
        mgr.stop_server().await;
        assert!(mgr.server_handle.read().await.is_none(), "server handle should be cleared after stop");
    }

    #[tokio::test]
    async fn test_start_server_invalid_cipher_returns_error_and_no_handle() {
        let mgr = ShadowsocksServerManager::new();

        let cfg = ServerConfig {
            server_port: 0,
            cipher: Some("invalid-cipher".to_string()),
            server_key: Some("dummy-key".to_string()),
            base_config: None,
        };

        let err = mgr.start_server(cfg).await.expect_err("invalid cipher should error");
        assert!(err.to_string().contains("Invalid cipher"));

        // Ensure no handle or config is left behind
        assert!(mgr.server_handle.read().await.is_none());
        assert!(mgr.current_config.read().await.is_none());
    }

    #[tokio::test]
    async fn test_update_users_while_server_running() {
        let mgr = ShadowsocksServerManager::new();

        // Seed initial users
        {
            let mut guard = mgr.users.write().await;
            *guard = make_users(2);
        }

        // Start server with valid settings
        let cfg = ServerConfig {
            server_port: 0,
            cipher: Some("2022-blake3-aes-128-gcm".to_string()),
            server_key: Some("YWJjZGVmZ2hpamtsbW5vcA==".to_string()),
            base_config: None,
        };

        mgr.start_server(cfg).await.expect("server should start");

        // Verify initial users loaded into manager
        assert_eq!(mgr.user_manager.user_count(), 2);

        // Update users while server is running
        let new_users = make_users(4);
        mgr.update_users(new_users.clone()).await;

        // Manager should reflect the updated list using the active config's cipher (16-byte keys)
        assert_eq!(mgr.user_manager.user_count(), new_users.len());
        for u in mgr.user_manager.users_iter() {
            assert_eq!(u.key().len(), 16);
        }

        mgr.stop_server().await;
    }

    #[tokio::test]
    async fn test_restart_server_with_new_config() {
        let mgr = ShadowsocksServerManager::new();

        // First start with 128-bit cipher and two users
        {
            let mut guard = mgr.users.write().await;
            *guard = make_users(2);
        }
        let cfg1 = ServerConfig {
            server_port: 0,
            cipher: Some("2022-blake3-aes-128-gcm".to_string()),
            server_key: Some("YWJjZGVmZ2hpamtsbW5vcA==".to_string()), // 16-byte key
            base_config: None,
        };
        mgr.start_server(cfg1.clone()).await.expect("first start should succeed");
        assert_eq!(mgr.user_manager.user_count(), 2);
        for u in mgr.user_manager.users_iter() {
            assert_eq!(u.key().len(), 16);
        }

        // Stop the running server
        mgr.stop_server().await;
        assert!(mgr.server_handle.read().await.is_none());

        // Prepare a new config with 256-bit cipher and a different user set
        {
            let mut guard = mgr.users.write().await;
            *guard = make_users(3);
        }
        let cfg2 = ServerConfig {
            server_port: 0,
            cipher: Some("2022-blake3-aes-256-gcm".to_string()),
            // 32-byte key base64
            server_key: Some("MTIzNDU2Nzg5MDEyMzQ1Njc4OTAxMjM0NTY3ODkwMTI=".to_string()),
            base_config: None,
        };

        mgr.start_server(cfg2.clone()).await.expect("second start should succeed");

        // Validate handle exists and config swapped
        assert!(mgr.server_handle.read().await.is_some());
        let stored = mgr.current_config.read().await;
        let stored_cfg = stored.as_ref().expect("config should be stored after restart");
        assert_eq!(stored_cfg.cipher, cfg2.cipher);
        assert_eq!(stored_cfg.server_key, cfg2.server_key);

        // User manager rebuilt with new users and key length 32
        assert_eq!(mgr.user_manager.user_count(), 3);
        for u in mgr.user_manager.users_iter() {
            assert_eq!(u.key().len(), 32);
        }

        mgr.stop_server().await;
    }
}
