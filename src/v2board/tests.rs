use super::*;
use std::fs;

fn load_config_from_toml() -> ApiConfig {
    let config_str = fs::read_to_string("config.toml").expect("cannot read config.toml");
    let config: ApiConfig = toml::from_str(&config_str).expect("cannot parse config.toml");

    assert!(!config.api_host.is_empty());
    assert!(config.node_id > 0);
    assert!(!config.key.is_empty());
    assert!(config.timeout > 0);

    config
}

#[test]
fn load_config_test() {
    let config = load_config_from_toml();

    println!("API Host: {}", config.api_host);
    println!("Node ID: {}", config.node_id);
    println!("Key: {}", config.key);
    println!("Timeout: {}", config.timeout);
}

#[tokio::test]
async fn get_node_info_test() {
    let config = load_config_from_toml();
    let client = ApiClient::new(config).expect("cannot new ApiClient");
    let node_info = client.get_node_info().await.expect("cannot get node info");

    println!("{:?}", node_info);
}

#[tokio::test]
async fn get_user_list_test() {
    let config = load_config_from_toml();
    let client = ApiClient::new(config).expect("cannot new ApiClient");
    let user_list = client.get_user_list().await.expect("cannot get user list");

    println!("{:?}", user_list);
}
