use super::*;
use log::info;
use std::fs;
use std::sync::Once;

static INIT_LOGGER: Once = Once::new();

fn init_logger() {
    INIT_LOGGER.call_once(|| {
        let _ = env_logger::builder().is_test(true).try_init();
    });
}

fn load_config_from_toml() -> ApiConfig {
    init_logger();

    let config_str = fs::read_to_string("config.toml").expect("cannot read config.toml");
    let config: ApiConfig = toml::from_str(&config_str).expect("cannot parse config.toml");

    assert!(!config.api_host.is_empty());
    assert!(config.node_id > 0);
    assert!(!config.key.is_empty());
    assert!(config.timeout > 0);

    config
}

#[test]
fn test_load_config() {
    let config = load_config_from_toml();

    info!("API Host: {}", config.api_host);
    info!("Node ID: {}", config.node_id);
    info!("Key: {}", config.key);
    info!("Timeout: {}", config.timeout);
}

#[tokio::test]
async fn test_get_node_info() {
    let config = load_config_from_toml();
    let client = ApiClient::new(config).expect("cannot new ApiClient");
    let node_info = client.get_node_info().await.expect("cannot get node info");

    info!("{:?}", node_info);
}

#[tokio::test]
async fn test_get_user_list() {
    let config = load_config_from_toml();
    let client = ApiClient::new(config).expect("cannot new ApiClient");
    let user_list = client.get_user_list().await.expect("cannot get user list");

    info!("{:?}", user_list);
}

#[tokio::test]
async fn test_combined() {
    let config = load_config_from_toml();
    let client = ApiClient::new(config).expect("cannot new ApiClient");
    let node_info = client.get_node_info().await.expect("cannot get node info");
    info!("{:?}", node_info);

    let user_list = client.get_user_list().await.expect("cannot get user list");
    info!("{:?}", user_list);
}
