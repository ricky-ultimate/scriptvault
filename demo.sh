#!/usr/bin/env bash
# ScriptVault Demo Script
# Run this to see ScriptVault in action!

set -e

# Colors
GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
NC='\033[0m'

SV="./target/debug/sv"

# Check if sv binary exists
if [ ! -f "$SV" ]; then
    echo "Building ScriptVault first..."
    cargo build
fi

echo -e "${CYAN}╔════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║   ScriptVault Demo                     ║${NC}"
echo -e "${CYAN}╚════════════════════════════════════════╝${NC}"
echo ""

# Pause helper
pause() {
    echo ""
    echo -e "${YELLOW}Press ENTER to continue...${NC}"
    read -r
    echo ""
}

# Step 1: Setup
echo -e "${GREEN}Step 1: Initial Setup${NC}"
echo "First, let's authenticate (local mode):"
echo ""
echo "$ sv auth login --token local"
$SV auth login --token local
pause

# Step 2: Check health
echo -e "${GREEN}Step 2: Health Check${NC}"
echo "Make sure everything is working:"
echo ""
echo "$ sv doctor"
$SV doctor
pause

# Step 3: View context
echo -e "${GREEN}Step 3: View Current Context${NC}"
echo "ScriptVault captures context automatically:"
echo ""
echo "$ sv context"
$SV context
pause

# Step 4: Create demo scripts
echo -e "${GREEN}Step 4: Create Demo Scripts${NC}"
echo "Let's create some example scripts..."
echo ""

mkdir -p demo-scripts

# Script 1: Hello World
cat > demo-scripts/hello.sh << 'EOF'
#!/bin/bash
echo "Hello from ScriptVault!"
echo "Current time: $(date)"
EOF

# Script 2: Git status
cat > demo-scripts/git-status.sh << 'EOF'
#!/bin/bash
echo "Git Status Report"
echo "================="
git status --short
echo ""
echo "Recent commits:"
git log --oneline -5
EOF

# Script 3: System info
cat > demo-scripts/sysinfo.sh << 'EOF'
#!/bin/bash
echo "System Information"
echo "=================="
echo "OS: $(uname -s)"
echo "Kernel: $(uname -r)"
echo "Architecture: $(uname -m)"
echo "Hostname: $(hostname)"
echo "User: $(whoami)"
EOF

echo "Created 3 demo scripts in demo-scripts/"
pause

# Step 5: Save scripts
echo -e "${GREEN}Step 5: Save Scripts to Vault${NC}"
echo "Now let's save them with tags and descriptions:"
echo ""

echo "$ sv save demo-scripts/hello.sh --tags 'demo greeting' --description 'Simple hello world' --yes"
$SV save demo-scripts/hello.sh --tags "demo greeting" --description "Simple hello world" --yes
echo ""

echo "$ sv save demo-scripts/git-status.sh --tags 'demo git' --description 'Show git status' --yes"
$SV save demo-scripts/git-status.sh --tags "demo git" --description "Show git status" --yes
echo ""

echo "$ sv save demo-scripts/sysinfo.sh --tags 'demo system' --description 'Show system info' --yes"
$SV save demo-scripts/sysinfo.sh --tags "demo system" --description "Show system info" --yes
pause

# Step 6: List scripts
echo -e "${GREEN}Step 6: List Your Scripts${NC}"
echo "View all saved scripts:"
echo ""
echo "$ sv list"
$SV list
pause

# Step 7: Search
echo -e "${GREEN}Step 7: Search Scripts${NC}"
echo "Find scripts by tag:"
echo ""
echo "$ sv find --tag demo"
$SV find --tag demo
pause

# Step 8: Get info
echo -e "${GREEN}Step 8: Get Script Info${NC}"
echo "View detailed information about a script:"
echo ""
echo "$ sv info hello"
$SV info hello
pause

# Step 9: Run a script
echo -e "${GREEN}Step 9: Run a Script${NC}"
echo "First, let's do a dry run to see what would happen:"
echo ""
echo "$ sv run hello --dry-run"
$SV run hello --dry-run
echo ""

echo "Now let's actually run it (using --ci for non-interactive mode):"
echo ""
echo "$ sv run hello --ci"
$SV run hello --ci
pause

# Step 10: Run another
echo -e "${GREEN}Step 10: Run System Info Script${NC}"
echo "$ sv run sysinfo --ci"
$SV run sysinfo --ci
pause

# Step 11: View history
echo -e "${GREEN}Step 11: View Execution History${NC}"
echo "See what we've run:"
echo ""
echo "$ sv history"
$SV history
pause

# Step 12: Find scripts by context
echo -e "${GREEN}Step 12: Context-Aware Search${NC}"
echo "Find scripts relevant to current directory:"
echo ""
echo "$ sv find --here"
$SV find --here
pause

# Cleanup option
echo ""
echo -e "${GREEN}Demo Complete! 🎉${NC}"
echo ""
echo "Your scripts are saved in ~/.scriptvault/"
echo ""
echo "Try these commands:"
echo "  sv find demo           # Find all demo scripts"
echo "  sv run hello          # Run hello script"
echo "  sv history            # View execution history"
echo "  sv context            # Show current context"
echo ""
echo "Want to clean up the demo scripts? (y/N)"
read -r response

if [[ "$response" =~ ^[Yy]$ ]]; then
    echo "Cleaning up..."
    rm -rf demo-scripts
    echo -e "${GREEN}✓ Demo scripts removed${NC}"
    echo ""
    echo "Note: Scripts are still in your vault at ~/.scriptvault/"
    echo "To remove them completely, delete ~/.scriptvault/"
fi

echo ""
echo -e "${CYAN}Thanks for trying ScriptVault!${NC}"
echo ""
