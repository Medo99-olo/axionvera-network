#!/bin/bash
set -e

# Build script for Axionvera Vault Contract WASM target
# This script builds the vault contract as a wasm32-unknown-unknown target

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CONTRACT_DIR="$PROJECT_ROOT/contracts/vault-contract"

echo "Building Axionvera Vault Contract for WASM..."
echo "Project root: $PROJECT_ROOT"
echo "Contract directory: $CONTRACT_DIR"
echo ""

# Check if contract directory exists
if [ ! -d "$CONTRACT_DIR" ]; then
    echo "Error: Contract directory not found at $CONTRACT_DIR"
    exit 1
fi

# Add wasm32 target if not already installed
echo "Checking for wasm32-unknown-unknown target..."
if ! rustup target list --installed | grep -q "wasm32-unknown-unknown"; then
    echo "Adding wasm32-unknown-unknown target..."
    rustup target add wasm32-unknown-unknown
else
    echo "wasm32-unknown-unknown target already installed"
fi
echo ""

# Build the contract for wasm32 target
echo "Building vault contract for wasm32-unknown-unknown..."
cd "$CONTRACT_DIR"
cargo build --target wasm32-unknown-unknown --release

# Check if build succeeded
if [ $? -eq 0 ]; then
    echo ""
    echo "Build successful!"
    echo "WASM file location: $CONTRACT_DIR/target/wasm32-unknown-unknown/release/axionvera_vault_contract.wasm"
else
    echo ""
    echo "Build failed!"
    exit 1
fi
