#!/bin/bash
# Script to run tests on local validator

set -e

echo "Running tests..."

# Run anchor tests (this will start its own validator)
anchor test

echo ""
echo "All tests passed!"
