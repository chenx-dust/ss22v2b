# ss22v2b

[![Rust](https://img.shields.io/badge/rust-1.83%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

一个轻量级的 V2Board 节点控制器，支持 Shadowsocks 2022 协议。自动从 V2Board API 拉取节点配置和用户数据，启动 Shadowsocks 服务器，并定期上报流量统计。

## ✨ 特性

- 🔄 **自动同步** - 定期从 V2Board 拉取节点配置和用户列表
- 📊 **流量统计** - 实时统计用户流量并上报到 V2Board
- 🚀 **高性能** - 基于 `shadowsocks-service`，支持 Shadowsocks 2022 最新协议
- 🔧 **灵活配置** - 支持 TCP/UDP、TCP Fast Open、TCP No Delay 等优化选项
- 📝 **详细日志** - 支持多级日志输出，方便调试和监控

## 🚀 快速开始

### 1. 安装 Rust

需要 Rust 1.83+ 版本：

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable
```

### 2. 配置文件

复制示例配置文件并修改：

```bash
cp config.example.toml config.toml
```

编辑 `config.toml`，填入你的 V2Board 配置：

```toml
# V2Board API 配置
api_host = "https://your-v2board-panel.com"  # V2Board 面板地址
node_id = 1                                   # 节点 ID
key = "your-api-key-here"                    # API 密钥
timeout = 30                                  # 请求超时时间（秒）

# Shadowsocks 服务器配置（可选）
[shadowsocks]
no_delay = true          # 启用 TCP_NODELAY，降低延迟
fast_open = false        # 启用 TCP Fast Open（需要内核支持）
timeout = 300            # TCP 连接超时时间
udp_timeout = 300        # UDP 关联超时时间
udp_mtu = 1500          # UDP MTU 大小
```

### 3. 编译运行

开发模式（调试）：

```bash
RUST_LOG=info cargo run
```

生产模式（优化）：

```bash
RUST_LOG=info cargo run --release
```

### 4. 后台运行

Linux/macOS 使用 systemd：

```bash
# 编译
cargo build --release

# 复制
cp target/release/ss22v2b /usr/local/bin/ss22v2b
cp config.toml /usr/local/etc/ss22v2b/config.toml

# 创建 systemd 服务文件
sudo nano /etc/systemd/system/ss22v2b.service
```

添加以下内容：

```ini
[Unit]
Description=SS22V2B Server
After=network.target

[Service]
Type=simple
User=nobody
WorkingDirectory=/usr/local/etc/ss22v2b
Environment="RUST_LOG=warn"
ExecStart=/usr/local/bin/ss22v2b
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

启动服务：

```bash
sudo systemctl daemon-reload
sudo systemctl enable ss22v2b
sudo systemctl start ss22v2b
sudo systemctl status ss22v2b
```

## 📖 工作原理

```
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

### 核心模块

- **`src/main.rs`** - 主程序入口，协调 API 客户端和服务器管理器
- **`src/v2board/`** - V2Board API 交互模块
  - `client.rs` - HTTP 客户端，处理 API 请求
  - `models.rs` - 数据模型定义
  - `callback.rs` - 流量回调处理
- **`src/manager/`** - Shadowsocks 服务器管理
  - `server.rs` - 服务器启动、停止、用户管理

### 工作流程

1. **初始化** - 读取配置文件，初始化 API 客户端
2. **拉取配置** - 从 V2Board 获取节点配置（端口、加密方式、密码等）
3. **拉取用户** - 获取所有用户列表及其密钥
4. **启动服务器** - 使用 `shadowsocks-service` 启动 SS2022 服务器
5. **更新用户** - 动态添加/删除用户，无需重启服务器
6. **流量统计** - 实时统计每个用户的上传/下载流量
7. **上报流量** - 定期将流量数据上报到 V2Board
8. **循环** - 持续监听信号，响应配置变更

## ⚙️ 配置说明

### V2Board API 配置

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `api_host` | String | ✅ | V2Board 面板地址 |
| `node_id` | Integer | ✅ | 节点 ID（在面板中配置） |
| `key` | String | ✅ | API 通信密钥 |
| `timeout` | Integer | ❌ | HTTP 请求超时时间（秒），默认 30 |

### Shadowsocks 服务器配置

所有配置项都在 `[shadowsocks]` 段中，均为可选：

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `timeout` | Integer | 300 | TCP 连接超时时间（秒） |
| `udp_timeout` | Integer | 300 | UDP 关联超时时间（秒） |
| `no_delay` | Boolean | false | 启用 TCP_NODELAY，降低延迟 |
| `fast_open` | Boolean | false | 启用 TCP Fast Open |
| `keep_alive` | Integer | - | TCP Keep-Alive 时间（秒） |
| `mptcp` | Boolean | false | 启用多路径 TCP |
| `udp_max_associations` | Integer | - | 每用户最大 UDP 并发连接数 |
| `udp_mtu` | Integer | 1500 | UDP MTU 大小（字节） |
| `ipv6_first` | Boolean | false | 优先使用 IPv6 地址 |

## 🔍 日志等级

通过 `RUST_LOG` 环境变量控制日志输出：

```bash
# 错误信息
RUST_LOG=error cargo run

# 警告信息
RUST_LOG=warn cargo run

# 一般信息（推荐）
RUST_LOG=info cargo run

# 调试信息
RUST_LOG=debug cargo run

# 详细追踪
RUST_LOG=trace cargo run

# 模块级别控制
RUST_LOG=ss22v2b=debug,shadowsocks=info cargo run
```

## 📝 注意事项

- ⚠️ **仅支持 Shadowsocks 2022 协议** - 不支持旧版 Shadowsocks 协议
- 🔑 **UUID 密钥处理** - 代码会自动将 UUID 截断到合适的密钥长度
- 🌐 **TCP/UDP 支持** - 默认同时启用 TCP 和 UDP 服务器
- 🚀 **TCP 优化** - 默认启用 TCP_NODELAY 和 TCP Fast Open
- 🐧 **平台支持** - Linux、macOS、Windows（TCP Fast Open 需要内核支持）

## 🛠️ 故障排除

### 连接失败

```bash
# 检查防火墙
sudo ufw allow <port>/tcp
sudo ufw allow <port>/udp

# 检查端口占用
ss -tulnp | grep <port>
```

### 日志调试

```bash
# 启用详细日志
RUST_LOG=debug cargo run

# 查看 shadowsocks-service 日志
RUST_LOG=shadowsocks=debug cargo run
```

### V2Board API 错误

- 检查 `api_host` 是否正确（包含 `https://`）
- 检查 `node_id` 和 `key` 是否匹配面板配置
- 检查网络连接和防火墙设置

## 📄 许可证

MIT License

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！
