#!/bin/bash
set -euo pipefail

# Check if running as root
if [[ $EUID -ne 0 ]]; then
    echo "This script must be run as root"
    exit 1
fi

REPO="chenx-dust/ss22v2b"
BIN_INSTALL_PATH="/usr/local/bin/ss22v2b"
CONFIG_DIR="/usr/local/etc/ss22v2b"
TEMP_DIR=$(mktemp -d)
trap "rm -rf $TEMP_DIR" EXIT

# Detect init system
if command -v systemctl &> /dev/null && systemctl --version &> /dev/null; then
    INIT_SYSTEM="systemd"
    SERVICE_DIR="/etc/systemd/system"
elif command -v rc-update &> /dev/null; then
    INIT_SYSTEM="openrc"
    SERVICE_DIR="/etc/init.d"
    CONF_DIR="/etc/conf.d"
else
    INIT_SYSTEM="unknown"
fi

echo "============================================"
echo "ss22v2b Installer"
echo "============================================"

# Detect system architecture
echo ""
echo "Detecting system architecture..."
ARCH=$(uname -m)
case "$ARCH" in
    x86_64)
        TARGET="x86_64-unknown-linux-gnu"
        ;;
    aarch64)
        TARGET="aarch64-unknown-linux-gnu"
        ;;
    *)
        echo "Error: Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

# Detect libc type (glibc or musl)
# Check for musl by examining libc.so or using ldd
LIBC="glibc"
if [ -f /lib/ld-musl-x86_64.so.1 ] || [ -f /lib/ld-musl-aarch64.so.1 ]; then
    LIBC="musl"
elif command -v ldd &> /dev/null && ldd --version 2>&1 | grep -q musl; then
    LIBC="musl"
elif [ -f /etc/alpine-release ]; then
    # Alpine Linux uses musl by default
    LIBC="musl"
fi

if [ "$LIBC" = "musl" ]; then
    case "$ARCH" in
        x86_64)
            TARGET="x86_64-unknown-linux-musl"
            ;;
        aarch64)
            TARGET="aarch64-unknown-linux-musl"
            ;;
    esac
fi
echo "   ✓ Architecture: $ARCH ($TARGET, $LIBC)"

# Get latest release version
echo ""
echo "Fetching latest release information..."
RELEASE_INFO=$(curl -s "https://api.github.com/repos/$REPO/releases/latest")
VERSION=$(echo "$RELEASE_INFO" | grep -o '"tag_name": "[^"]*' | cut -d'"' -f4)

if [[ -z "$VERSION" ]]; then
    echo "Error: Failed to fetch release information"
    exit 1
fi
echo "   ✓ Latest version: $VERSION"

# Download binary
echo ""
echo "Downloading binary for $TARGET..."
DOWNLOAD_URL="https://github.com/$REPO/releases/download/$VERSION/ss22v2b-$TARGET"
BINARY="$TEMP_DIR/ss22v2b"

if ! curl -fL "$DOWNLOAD_URL" -o "$BINARY" 2>/dev/null; then
    echo "Error: Failed to download binary from $DOWNLOAD_URL"
    echo "Please check your internet connection and try again"
    exit 1
fi

if [[ ! -f "$BINARY" ]]; then
    echo "Error: Binary download failed"
    exit 1
fi

chmod +x "$BINARY"
echo "   ✓ Binary downloaded successfully"

# 1. Install binary
echo ""
echo "1. Installing binary..."
cp "$BINARY" "$BIN_INSTALL_PATH"
chmod 755 "$BIN_INSTALL_PATH"
echo "   ✓ Binary installed to $BIN_INSTALL_PATH"

# 2. Create config directory
echo ""
echo "2. Creating config directory..."
mkdir -p "$CONFIG_DIR"
chmod 755 "$CONFIG_DIR"
echo "   ✓ Config directory: $CONFIG_DIR"

# 3. Install example config
echo ""
echo "3. Installing config file..."
if [[ ! -f "${CONFIG_DIR}/config.toml" ]]; then
    # Download config.example.toml from repository
    if curl -fL "https://raw.githubusercontent.com/$REPO/$VERSION/config.example.toml" -o "${CONFIG_DIR}/config.toml" 2>/dev/null; then
        chmod 644 "${CONFIG_DIR}/config.toml"
        echo "   ✓ Config file installed to ${CONFIG_DIR}/config.toml"
        echo "   ⚠ Please edit this file with your settings"
    else
        echo "   ⚠ Warning: Could not download config file from repository"
        echo "   Please configure ${CONFIG_DIR}/config.toml manually"
    fi
else
    echo "   ℹ Config file already exists, skipping"
fi

# 4. Install service files
echo ""
if [[ "$INIT_SYSTEM" == "systemd" ]]; then
    echo "4. Installing systemd service files..."
    if curl -fL "https://raw.githubusercontent.com/$REPO/$VERSION/ss22v2b.service" -o "${SERVICE_DIR}/ss22v2b.service" 2>/dev/null; then
        chmod 644 "${SERVICE_DIR}/ss22v2b.service"
        echo "   ✓ Installed ss22v2b.service"
    else
        echo "   ⚠ Warning: Could not download ss22v2b.service"
    fi
    
    if curl -fL "https://raw.githubusercontent.com/$REPO/$VERSION/ss22v2b@.service" -o "${SERVICE_DIR}/ss22v2b@.service" 2>/dev/null; then
        chmod 644 "${SERVICE_DIR}/ss22v2b@.service"
        echo "   ✓ Installed ss22v2b@.service (for multi-instance)"
    else
        echo "   ⚠ Warning: Could not download ss22v2b@.service"
    fi
elif [[ "$INIT_SYSTEM" == "openrc" ]]; then
    echo "4. Installing OpenRC service files..."
    if curl -fL "https://raw.githubusercontent.com/$REPO/$VERSION/ss22v2b.openrc" -o "${SERVICE_DIR}/ss22v2b" 2>/dev/null; then
        chmod 755 "${SERVICE_DIR}/ss22v2b"
        echo "   ✓ Installed OpenRC service script to ${SERVICE_DIR}/ss22v2b"
    else
        echo "   ⚠ Warning: Could not download ss22v2b.openrc"
    fi
    
    if curl -fL "https://raw.githubusercontent.com/$REPO/$VERSION/ss22v2b.openrc.confd" -o "${CONF_DIR}/ss22v2b" 2>/dev/null; then
        chmod 644 "${CONF_DIR}/ss22v2b"
        echo "   ✓ Installed OpenRC config to ${CONF_DIR}/ss22v2b"
    else
        echo "   ⚠ Warning: Could not download ss22v2b.openrc.confd"
    fi
else
    echo "4. Skipping service installation (no supported init system detected)"
fi

# 5. Reload service manager
echo ""
if [[ "$INIT_SYSTEM" == "systemd" ]]; then
    echo "5. Reloading systemd configuration..."
    systemctl daemon-reload
    echo "   ✓ systemd reloaded"
elif [[ "$INIT_SYSTEM" == "openrc" ]]; then
    echo "5. OpenRC service installed"
    echo "   ✓ Service ready"
else
    echo "5. Service manager configuration skipped"
fi

echo ""
echo "============================================"
echo "Installation Complete!"
echo "============================================"
echo ""
echo "Quick Start:"
echo ""
echo "1. Edit config file:"
echo "   sudo nano ${CONFIG_DIR}/config.toml"
echo ""

if [[ "$INIT_SYSTEM" == "systemd" ]]; then
    echo "2. Start the service:"
    echo "   sudo systemctl start ss22v2b"
    echo ""
    echo "3. Enable auto-start on boot:"
    echo "   sudo systemctl enable ss22v2b"
    echo ""
    echo "4. Check service status:"
    echo "   sudo systemctl status ss22v2b"
    echo ""
    echo "5. View logs:"
    echo "   sudo journalctl -u ss22v2b -f"
    echo ""
    echo "Multi-Instance Mode (using ss22v2b@.service):"
    echo ""
    echo "1. Create config file:"
    echo "   sudo cp ${CONFIG_DIR}/config.toml ${CONFIG_DIR}/instance1.toml"
    echo "   sudo nano ${CONFIG_DIR}/instance1.toml"
    echo ""
    echo "2. Start instance:"
    echo "   sudo systemctl start ss22v2b@instance1"
    echo ""
    echo "3. Enable auto-start:"
    echo "   sudo systemctl enable ss22v2b@instance1"
    echo ""
elif [[ "$INIT_SYSTEM" == "openrc" ]]; then
    echo "2. Start the service:"
    echo "   sudo rc-service ss22v2b start"
    echo ""
    echo "3. Enable auto-start on boot:"
    echo "   sudo rc-update add ss22v2b default"
    echo ""
    echo "4. Check service status:"
    echo "   sudo rc-service ss22v2b status"
    echo ""
    echo "5. View logs:"
    echo "   sudo tail -f /var/log/ss22v2b.log"
    echo ""
    echo "Multi-Instance Mode (using symlinks):"
    echo ""
    echo "1. Create config file:"
    echo "   sudo cp ${CONFIG_DIR}/config.toml ${CONFIG_DIR}/instance1.toml"
    echo "   sudo nano ${CONFIG_DIR}/instance1.toml"
    echo ""
    echo "2. Create service symlink:"
    echo "   sudo ln -s /etc/init.d/ss22v2b /etc/init.d/ss22v2b.instance1"
    echo ""
    echo "3. Create config file for instance:"
    echo "   echo 'config_file=\"/usr/local/etc/ss22v2b/instance1.toml\"' | sudo tee /etc/conf.d/ss22v2b.instance1"
    echo ""
    echo "4. Start instance:"
    echo "   sudo rc-service ss22v2b.instance1 start"
    echo ""
    echo "5. Enable auto-start:"
    echo "   sudo rc-update add ss22v2b.instance1 default"
    echo ""
else
    echo "2. Start the service manually:"
    echo "   ${BIN_INSTALL_PATH} --config ${CONFIG_DIR}/config.toml"
    echo ""
    echo "   Note: No supported init system detected."
    echo "   You may need to configure service management manually."
    echo ""
fi
