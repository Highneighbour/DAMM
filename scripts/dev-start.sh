#!/bin/bash
# Script to start local validator for development and testing

set -e

echo "Starting local Solana validator..."

# Kill any existing validator
pkill -f solana-test-validator || true
sleep 2

# Start validator with necessary programs
solana-test-validator \
  --reset \
  --quiet \
  --bpf-program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA /usr/local/bin/spl_token.so &

echo "Waiting for validator to start..."
sleep 5

# Configure Solana CLI
solana config set --url http://localhost:8899

echo "Validator started successfully!"
echo "RPC URL: http://localhost:8899"
echo ""
echo "Building and deploying program..."

# Build the program
anchor build

# Deploy the program
anchor deploy --provider.cluster localnet

echo ""
echo "Development environment ready!"
echo "Run 'anchor test --skip-local-validator' to run tests against this validator"
