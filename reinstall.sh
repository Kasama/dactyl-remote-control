#!/bin/bash

CONFIG_DIR="$XDG_CONFIG_HOME/dactyl-remote-control/"
CONFIG_FILE="$CONFIG_DIR/config.json"

mkdir -p "$CONFIG_DIR"

if [ ! -e "$CONFIG_FILE" ]; then
    echo "Config file doesn't exist, creating one"
    cargo run -- watch-i3-focus --create-config
fi

cargo install --path .
