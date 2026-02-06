#!/bin/bash

set -e

load_env_files() {
    if [[ -f ".env" ]]; then
        # Export values from .env into the current shell.
        set -a
        # shellcheck disable=SC1091
        source ".env"
        set +a
    fi

    if [[ -f ".env.local" ]]; then
        set -a
        # shellcheck disable=SC1091
        source ".env.local"
        set +a
    fi
}

flush_cloudflare_cache() {
    if [[ -z "${CF_API_TOKEN:-}" || -z "${CF_ZONE_ID:-}" ]]; then
        echo "Skipping Cloudflare cache flush (set CF_API_TOKEN and CF_ZONE_ID to enable)"
        return 0
    fi

    echo "Flushing Cloudflare cache..."
    local response
    response="$(curl -sS -X POST "https://api.cloudflare.com/client/v4/zones/${CF_ZONE_ID}/purge_cache" \
        -H "Authorization: Bearer ${CF_API_TOKEN}" \
        -H "Content-Type: application/json" \
        --data '{"purge_everything":true}')"

    if command -v jq >/dev/null 2>&1; then
        if echo "${response}" | jq -e '.success == true' >/dev/null 2>&1; then
            echo "Cloudflare cache flushed"
            return 0
        fi
    elif echo "${response}" | grep -Eq '"success"[[:space:]]*:[[:space:]]*true'; then
        echo "Cloudflare cache flushed"
        return 0
    fi

    echo "Cloudflare cache flush failed:"
    echo "${response}"
    return 1
}

load_env_files

echo "Stopping existing shetaye-me service if running..."
if sudo systemctl is-active --quiet shetaye-me.service 2>/dev/null; then
    sudo systemctl stop shetaye-me.service
    echo "Stopped shetaye-me service"
else
    echo "Service not running or not installed"
fi

echo "Cleaning previous build..."
cargo clean

echo "Building shetaye.me website in release mode..."
cargo build --release

echo "Installing binary to /usr/local/bin..."
sudo cp target/release/shetaye_me /usr/local/bin/

echo "Installing systemd service..."
sudo cp unit/shetaye-me.service /etc/systemd/system/

echo "Reloading systemd daemon..."
sudo systemctl daemon-reload

echo "Enabling and starting shetaye-me service..."
sudo systemctl enable shetaye-me.service
sudo systemctl start shetaye-me.service

flush_cloudflare_cache

echo "Installation complete!"
echo "Service status:"
sudo systemctl status shetaye-me.service --no-pager -l
