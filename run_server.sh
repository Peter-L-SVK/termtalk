#!/bin/bash

# Get the directory of the script
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"

# Define the server binary name
SERVER_BIN="server"

# Check if the server binary exists
if [ ! -f "$SCRIPT_DIR/target/release/$SERVER_BIN" ]; then
    echo "Server binary not found. Building the server..."
    (cd "$SCRIPT_DIR" && cargo build --release --bin $SERVER_BIN)
    if [ $? -ne 0 ]; then
        echo "Failed to build the server."
        exit 1
    fi
fi

# Run the server
echo "Starting the server..."
"$SCRIPT_DIR/target/release/$SERVER_BIN"
