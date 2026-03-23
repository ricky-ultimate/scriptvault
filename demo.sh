#!/usr/bin/env bash

set -e

GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
NC='\033[0m'

SV="./target/debug/sv"

if [ ! -f "$SV" ]; then
    echo "Building ScriptVault..."
    cargo build
fi

echo -e "${CYAN}ScriptVault Demo${NC}"
echo ""

pause() {
    echo ""
    echo -e "${YELLOW}Press ENTER to continue...${NC}"
    read -r
    echo ""
}

echo -e "${GREEN}Step 1: Set username${NC}"
echo "$ sv auth login --token demo-user"
$SV auth login --token demo-user
pause

echo -e "${GREEN}Step 2: Health check${NC}"
echo "$ sv doctor"
$SV doctor
pause

echo -e "${GREEN}Step 3: Current context${NC}"
echo "$ sv context"
$SV context
pause

echo -e "${GREEN}Step 4: Create demo scripts${NC}"
mkdir -p demo-scripts

cat > demo-scripts/hello.sh << 'EOF'
#!/bin/bash
echo "Hello from ScriptVault!"
echo "Current time: $(date)"
EOF

cat > demo-scripts/sysinfo.sh << 'EOF'
#!/bin/bash
echo "System Information"
echo "=================="
echo "OS: $(uname -s)"
echo "Kernel: $(uname -r)"
echo "Hostname: $(hostname)"
echo "User: $(whoami)"
EOF

cat > demo-scripts/git-status.sh << 'EOF'
#!/bin/bash
echo "Git Status"
echo "=========="
git status --short 2>/dev/null || echo "Not a git repository"
EOF

echo "Created 3 demo scripts"
pause

echo -e "${GREEN}Step 5: Save scripts${NC}"
echo "$ sv save demo-scripts/hello.sh --tags 'demo greeting' --description 'Hello world' --yes"
$SV save demo-scripts/hello.sh --tags "demo greeting" --description "Hello world" --yes

echo "$ sv save demo-scripts/sysinfo.sh --tags 'demo system' --description 'System info' --yes"
$SV save demo-scripts/sysinfo.sh --tags "demo system" --description "System info" --yes

echo "$ sv save demo-scripts/git-status.sh --tags 'demo git' --description 'Git status' --yes"
$SV save demo-scripts/git-status.sh --tags "demo git" --description "Git status" --yes
pause

echo -e "${GREEN}Step 6: List scripts${NC}"
echo "$ sv list"
$SV list
pause

echo -e "${GREEN}Step 7: Find by tag${NC}"
echo "$ sv find --tag demo"
$SV find --tag demo
pause

echo -e "${GREEN}Step 8: Script info${NC}"
echo "$ sv info hello"
$SV info hello
pause

echo -e "${GREEN}Step 9: View script content${NC}"
echo "$ sv cat hello"
$SV cat hello
pause

echo -e "${GREEN}Step 10: Dry run${NC}"
echo "$ sv run hello --dry-run"
$SV run hello --dry-run
pause

echo -e "${GREEN}Step 11: Run a script${NC}"
echo "$ sv run hello --ci"
$SV run hello --ci
pause

echo -e "${GREEN}Step 12: Run with arguments${NC}"
echo "$ sv run sysinfo --ci"
$SV run sysinfo --ci
pause

echo -e "${GREEN}Step 13: Verbose run (shows script content before execution)${NC}"
echo "$ sv run hello --ci --verbose"
$SV run hello --ci --verbose
pause

echo -e "${GREEN}Step 14: Execution history${NC}"
echo "$ sv history"
$SV history
pause

echo -e "${GREEN}Step 15: Stats for a script${NC}"
echo "$ sv stats hello"
$SV stats hello
pause

echo -e "${GREEN}Step 16: Update a script${NC}"
cat > demo-scripts/hello.sh << 'EOF'
#!/bin/bash
echo "Hello from ScriptVault!"
echo "Current time: $(date)"
echo "Version 2 - updated content"
EOF

echo "$ sv update demo-scripts/hello.sh"
$SV update demo-scripts/hello.sh
echo ""
echo "$ sv info hello"
$SV info hello
pause

echo -e "${GREEN}Step 17: Context-aware find${NC}"
echo "$ sv find --here"
$SV find --here
pause

echo -e "${GREEN}Step 18: Export to markdown${NC}"
echo "$ sv export --format markdown --output demo-export.md"
$SV export --format markdown --output demo-export.md
echo "Exported. First few lines:"
head -20 demo-export.md
rm -f demo-export.md
pause

echo ""
echo -e "${GREEN}Demo complete.${NC}"
echo ""
echo "Scripts are saved in ~/.scriptvault/"
echo ""
echo "Clean up demo scripts? (y/N)"
read -r response

if [[ "$response" =~ ^[Yy]$ ]]; then
    rm -rf demo-scripts
    echo "Demo scripts removed."
    echo "Vault scripts remain at ~/.scriptvault/"
    echo "To remove them: sv delete hello --yes && sv delete sysinfo --yes && sv delete git-status --yes"
fi
