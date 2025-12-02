#!/bin/bash

# RR3 Staking - Solana Verify Script
# This script verifies that the deployed program matches the source code

set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

PROGRAM_ID="8HVDihB8NrYpqzRNrFuiSPUC7B4rRqa8HFRJNWoxH4JK"
LIBRARY_NAME="staker"
NETWORK="devnet"

echo -e "${YELLOW}RR3 Staking - Solana Program Verification${NC}"
echo "==========================================="
echo ""

# Check if solana-verify is installed
if ! command -v solana-verify &> /dev/null; then
    echo -e "${RED}Error: solana-verify is not installed${NC}"
    echo "Install it with: cargo install solana-verify"
    exit 1
fi

# Check if git is initialized
if [ ! -d .git ]; then
    echo -e "${RED}Error: Git repository not initialized${NC}"
    echo "Run: git init && git add . && git commit -m 'Initial commit'"
    exit 1
fi

# Get current commit hash
COMMIT_HASH=$(git rev-parse HEAD)

echo -e "${GREEN}Configuration:${NC}"
echo "  Program ID: $PROGRAM_ID"
echo "  Network: $NETWORK"
echo "  Library: $LIBRARY_NAME"
echo "  Commit: $COMMIT_HASH"
echo ""

# Get repository URL
REPO_URL=$(git config --get remote.origin.url || echo "NOT_SET")

if [ "$REPO_URL" = "NOT_SET" ]; then
    echo -e "${YELLOW}Warning: Git remote origin not set${NC}"
    echo "For verification, you need to:"
    echo "  1. Push this code to GitHub"
    echo "  2. Set the remote: git remote add origin <YOUR_REPO_URL>"
    echo ""
    echo -e "${YELLOW}Verification command (run after pushing to GitHub):${NC}"
    echo ""
    echo "solana-verify verify-from-repo \\"
    echo "  --program-id $PROGRAM_ID \\"
    echo "  --repo <YOUR_GITHUB_REPO_URL> \\"
    echo "  --commit-hash $COMMIT_HASH \\"
    echo "  --library-name $LIBRARY_NAME \\"
    echo "  --cluster $NETWORK"
    echo ""
else
    echo -e "${GREEN}Repository: $REPO_URL${NC}"
    echo ""
    echo -e "${YELLOW}Running verification...${NC}"
    echo ""
    
    solana-verify verify-from-repo \
      --program-id "$PROGRAM_ID" \
      --repo "$REPO_URL" \
      --commit-hash "$COMMIT_HASH" \
      --library-name "$LIBRARY_NAME" \
      --cluster "$NETWORK"
    
    if [ $? -eq 0 ]; then
        echo ""
        echo -e "${GREEN}✓ Verification successful!${NC}"
        echo "The deployed program matches the source code."
    else
        echo ""
        echo -e "${RED}✗ Verification failed${NC}"
        echo "The deployed program does not match the source code."
        exit 1
    fi
fi
