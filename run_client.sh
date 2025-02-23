#!/bin/bash

# Get the directory of the script
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"

# Define the client binary name
CLIENT_BIN="client"

# Check if the client binary exists
if [ ! -f "$SCRIPT_DIR/target/debug/$CLIENT_BIN" ]; then
    echo "Client binary not found. Building the client..."
    (cd "$SCRIPT_DIR" && cargo build --release --bin $CLIENT_BIN)
    if [ $? -ne 0 ]; then
        echo "Failed to build the client."
        exit 1
    fi
fi

# Run the client
echo "Starting the client..."
"$SCRIPT_DIR/target/debug/$CLIENT_BIN"
