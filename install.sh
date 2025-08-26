#!/bin/bash

set -e

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

echo "Installation complete!"
echo "Service status:"
sudo systemctl status shetaye-me.service --no-pager -l