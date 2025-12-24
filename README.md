# ss22v2b

A small controller that talks to a V2Board-compatible API, pulls node and user data, spins up a Shadowsocks 2022 server (via `shadowsocks-service`), and periodically reports per-user traffic back.

## Quick start
1. Install Rust (edition 2024 compatible; `rustup default stable`).
2. Copy `config.toml` and fill in your values:

   ```toml
   api_host = "https://your-v2board.example"  # Base URL of the panel API
   node_id  = 123                              # Node ID configured on the panel
   key      = "your-api-token"                # Token for this node
   timeout  = 30                               # Optional HTTP timeout (seconds)
   ```

3. Run with logging:

   ```bash
   RUST_LOG=info cargo run --release
   ```

   The service will:
   - fetch the node config and users from V2Board
   - start a Shadowsocks server on the reported port/cipher/key
   - push user traffic stats on the configured interval

## How it works
- `src/main.rs` wires up the API client and server manager, registers callbacks, and drives the loop.
- `src/v2board/` handles API calls, parsing server/user data, and traffic reporting.
- `src/manager/` wraps `shadowsocks-service` to start/stop the server and update users.

## Notes
- Only Shadowsocks 2022 ciphers are expected; the code currently truncates UUIDs to the proper key length.
- TCP Fast Open and TCP_NODELAY are enabled by default.
- TCP and UDP servers are enabled by default.
- Use `RUST_LOG=debug` for more visibility when troubleshooting.
