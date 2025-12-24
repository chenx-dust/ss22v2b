use super::server::ShadowsocksServerManager;
use crate::v2board::{ServerConfig, UserInfo};
use shadowsocks_service::shadowsocks::config::ServerUserManager;

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
