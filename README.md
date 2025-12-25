# ss22v2b

[![Rust](https://img.shields.io/badge/rust-1.83%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A lightweight V2Board node controller supporting Shadowsocks 2022 protocol. Automatically fetches node configuration and user data from V2Board API, starts Shadowsocks servers, and periodically reports traffic statistics.

## ✨ Features

- 🔄 **Auto Sync** - Periodically fetch node configuration and user list from V2Board
- 📊 **Traffic Statistics** - Real-time traffic monitoring and reporting to V2Board
- 🚀 **High Performance** - Based on `shadowsocks-service`, supports latest Shadowsocks 2022 protocol
- 🔧 **Flexible Configuration** - Supports TCP/UDP, TCP Fast Open, TCP No Delay and other optimization options
- 📝 **Detailed Logging** - Multi-level logging output for debugging and monitoring
- 🚇 **Relay Support** - Support relaying through another Shadowsocks server

## 🚀 Quick Start

```bash
sudo curl -fL https://raw.githubusercontent.com/chenx-dust/ss22v2b/main/install.sh | sudo bash
```

After installation:

```bash
# Edit configuration
sudo nano /usr/local/etc/ss22v2b/config.toml

# Start service
sudo systemctl start ss22v2b
sudo systemctl enable ss22v2b

# View logs
sudo journalctl -u ss22v2b -f
```

## 📖 How It Works

```plaintext
┌─────────────┐         ┌──────────────┐         ┌─────────────┐
│  V2Board    │◄───────►│   ss22v2b    │◄───────►│   Clients   │
│   Panel     │   API   │  Controller  │  SS2022 │  (Users)    │
└─────────────┘         └──────────────┘         └─────────────┘
      │                       │
      │  1. Fetch config      │
      │  2. Pull users        │
      │  3. Report traffic    │
      │                       ├─ Shadowsocks Server
      │                       ├─ User Management
      └───────────────────────┴─ Traffic Statistics
```

### Core Modules

- **`src/main.rs`** - Main entry point, coordinates API client and server manager
- **`src/v2board/`** - V2Board API interaction module
  - `client.rs` - HTTP client for API requests
  - `models.rs` - Data model definitions
  - `callback.rs` - Traffic callback handling
- **`src/manager/`** - Shadowsocks server management
  - `server.rs` - Server start/stop and user management

## ⚙️ Configuration

### V2Board API Configuration

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `api_host` | String | ✅ | V2Board panel URL |
| `node_id` | Integer | ✅ | Node ID (configured in panel) |
| `key` | String | ✅ | API communication key |
| `timeout` | Integer | ❌ | HTTP request timeout (seconds), default 30 |

### Shadowsocks Server Configuration

All configuration items in `[shadowsocks]` section are optional:

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `timeout` | Integer | 300 | TCP connection timeout (seconds) |
| `udp_timeout` | Integer | 300 | UDP association timeout (seconds) |
| `no_delay` | Boolean | false | Enable TCP_NODELAY for lower latency |
| `fast_open` | Boolean | false | Enable TCP Fast Open |
| `keep_alive` | Integer | - | TCP Keep-Alive time (seconds) |
| `mptcp` | Boolean | false | Enable multipath TCP |
| `udp_max_associations` | Integer | - | Maximum UDP concurrent connections per user |
| `udp_mtu` | Integer | 1500 | UDP MTU size (bytes) |
| `ipv6_first` | Boolean | false | Prefer IPv6 addresses |
| `relay` | String | - | Relay Shadowsocks server URL |

## 🔍 Logging Levels

Control log output with `RUST_LOG` environment variable:

```bash
# Error messages
RUST_LOG=error cargo run

# Warning messages
RUST_LOG=warn cargo run

# General information (recommended)
RUST_LOG=info cargo run

# Debug information
RUST_LOG=debug cargo run

# Detailed tracing
RUST_LOG=trace cargo run

# Module-level control
RUST_LOG=ss22v2b=debug,shadowsocks=info cargo run
```

## 📝 Notes

- ⚠️ **Shadowsocks 2022 Protocol Only** - Does not support legacy Shadowsocks protocol
- 🔑 **UUID Key Handling** - Code automatically truncates UUID to appropriate key length
- 🌐 **TCP/UDP Support** - Both TCP and UDP servers enabled by default
- 🚀 **TCP Optimization** - TCP_NODELAY and TCP Fast Open enabled by default
- 🐧 **Platform Support** - Linux, macOS, Windows (TCP Fast Open requires kernel support)

## 🛠️ Troubleshooting

### Connection Failures

```bash
# Check firewall
sudo ufw allow <port>/tcp
sudo ufw allow <port>/udp

# Check port usage
ss -tulnp | grep <port>
```

### Debug Logging

```bash
# Enable detailed logging
RUST_LOG=debug cargo run

# View shadowsocks-service logs
RUST_LOG=shadowsocks=debug cargo run
```

### V2Board API Errors

- Verify `api_host` is correct (include `https://`)
- Verify `node_id` and `key` match panel configuration
- Check network connectivity and firewall settings

## 📄 License

MIT License

## 🤝 Contributing

Issues and Pull Requests are welcome!
