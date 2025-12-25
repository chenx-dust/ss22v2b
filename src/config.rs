use serde::{Deserialize, Serialize};
use shadowsocks_service::shadowsocks::{config::Mode, relay::tcprelay::proxy_stream::protocol::v2::SERVER_STREAM_TIMESTAMP_MAX_DIFF};
use std::time::Duration;

use crate::v2board::ApiConfig;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// V2Board API configuration
    #[serde(flatten)]
    pub api: ApiConfig,

    /// Shadowsocks server settings
    #[serde(default)]
    pub shadowsocks: ShadowsocksConfig,
}

impl Config {
    /// Load config from TOML file
    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }
}

/// Shadowsocks server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowsocksConfig {
    /// Relay Shadowsocks server URL (e.g., "ss://method:password@host:port")
    /// If specified, traffic will be relayed through this server
    pub relay: Option<String>,

    /// TCP connection timeout in seconds (default: 300)
    #[serde(default = "default_timeout")]
    pub timeout: u64,

    /// UDP timeout in seconds (default: 300)
    #[serde(default = "default_udp_timeout")]
    pub udp_timeout: u64,

    /// Set TCP_NODELAY socket option (default: false)
    #[serde(default)]
    pub no_delay: bool,

    /// Set TCP_FASTOPEN socket option (default: false)
    #[serde(default)]
    pub fast_open: bool,

    /// TCP Keep-Alive duration in seconds (default: None)
    pub keep_alive: Option<u64>,

    /// Enable Multipath TCP (default: false)
    #[serde(default)]
    pub mptcp: bool,

    /// DNS server address (e.g., "8.8.8.8", "1.1.1.1")
    pub dns: Option<String>,

    /// Use IPv6 addresses first (default: false)
    #[serde(default)]
    pub ipv6_first: bool,

    /// Maximum number of UDP associations (default: None)
    pub udp_max_associations: Option<usize>,

    /// Maximum Transmission Unit (MTU) size for UDP packets (default: 1500)
    #[serde(default = "default_udp_mtu")]
    pub udp_mtu: usize,

    /// Shadowsocks server mode: "tcp_only", "udp_only", or "tcp_and_udp" (default: "tcp_and_udp")
    #[serde(default = "default_mode")]
    pub mode: Mode,

    // AEAD 2022 timestamp limit in seconds (default: 30)
    #[serde(default = "default_timestamp_limit")]
    pub timestamp_limit: u64,

    // AEAD 2022 complying with incoming timestamp (default: false)
    pub comply_with_incoming: bool,
}

impl Default for ShadowsocksConfig {
    fn default() -> Self {
        Self {
            relay: None,
            timeout: default_timeout(),
            udp_timeout: default_udp_timeout(),
            no_delay: false,
            fast_open: false,
            keep_alive: None,
            mptcp: false,
            dns: None,
            ipv6_first: false,
            udp_max_associations: None,
            udp_mtu: default_udp_mtu(),
            mode: default_mode(),
            timestamp_limit: default_timestamp_limit(),
            comply_with_incoming: false,
        }
    }
}

impl ShadowsocksConfig {
    /// Get timeout as Duration
    pub fn timeout_duration(&self) -> Duration {
        Duration::from_secs(self.timeout)
    }

    /// Get UDP timeout as Duration
    pub fn udp_timeout_duration(&self) -> Duration {
        Duration::from_secs(self.udp_timeout)
    }

    /// Get keep-alive as Duration
    pub fn keep_alive_duration(&self) -> Option<Duration> {
        self.keep_alive.map(Duration::from_secs)
    }
}

fn default_timeout() -> u64 {
    300
}

fn default_udp_timeout() -> u64 {
    300
}

fn default_udp_mtu() -> usize {
    1500
}

fn default_mode() -> Mode {
    Mode::TcpAndUdp
}

fn default_timestamp_limit() -> u64 {
    SERVER_STREAM_TIMESTAMP_MAX_DIFF
}
