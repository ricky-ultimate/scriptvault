# Getting Started with ScriptVault

## Building from Source

### Prerequisites
- Rust 1.70+ (install via [rustup](https://rustup.rs/))
- Git

### Build Steps

```bash
# Clone the repository
git clone https://github.com/ricky-ultimate/scriptvault
cd scriptvault

# Build the project
cargo build --release

# The binary will be at target/release/sv
# Optionally, install it globally
cargo install --path .

# Or copy to your bin directory
cp target/release/sv ~/.local/bin/
```

## First Time Setup

```bash
# Initialize (creates ~/.scriptvault/)
sv auth login --token local

# Check everything is working
sv doctor
```

## Basic Workflow

### 1. Save Your First Script

Let's say you have a deployment script:

```bash
# Create a simple test script
cat > deploy-test.sh << 'EOF'
#!/bin/bash
echo "Deploying application..."
git pull origin main
echo "Deployment complete!"
EOF

# Save it to your vault
sv save ./deploy-test.sh --tags "deployment git test"
```

**What happened:**
- ScriptVault analyzed your script
- Detected you're in a git repository (if applicable)
- Captured the current directory context
- Stored it in `~/.scriptvault/vault/scripts.json`

### 2. Find Your Scripts

```bash
# Search by name
sv find deploy

# Search by tag
sv find --tag deployment

# Find scripts relevant to current directory
sv find --here

# List all your scripts
sv list
```

### 3. Run Your Scripts

```bash
# Preview what will happen (dry run)
sv run deploy-test --dry-run

# Run the script
sv run deploy-test

# Run with verbose output
sv run deploy-test --verbose
```

### 4. Check Execution History

```bash
# See all executions
sv history

# See only failed runs
sv history --failed

# Get detailed stats about a script
sv info deploy-test
```

## Advanced Usage

### Context-Aware Scripts

ScriptVault automatically captures:
- **Directory**: Where the script was created
- **Git Repository**: Which repo you were in
- **Git Branch**: Current branch
- **Environment**: Relevant env variables

```bash
# View current context
sv context

# Scripts will show you if they were created in a different context
sv run deploy-test  # Will warn if you're not in the same directory
```

### Organizing with Tags

```bash
# Save with multiple tags
sv save ./backup.sh --tags "database backup production"

# Find all database-related scripts
sv find --tag database

# Find all backup scripts
sv find --tag backup
```

### Safety Features

ScriptVault checks for dangerous commands:

```bash
# This will trigger a warning
echo "rm -rf /" > danger.sh
sv save danger.sh
sv run danger  #  Will ask for confirmation
```

### Exporting Documentation

```bash
# Export all scripts as markdown
sv export --format markdown --output SCRIPTS.md

# Create a cheatsheet
sv export --format cheatsheet
```

## Example: Real-World Workflow

### Scenario: Managing Multiple Projects

```bash
# In project A
cd ~/projects/web-app
sv save ./deploy-staging.sh --tags "webapp deployment staging"
sv save ./db-backup.sh --tags "webapp database backup"

# In project B
cd ~/projects/api-server
sv save ./deploy.sh --tags "api deployment"
sv save ./test-integration.sh --tags "api testing"

# Later, find all deployment scripts
sv find --tag deployment

# Or find scripts for current project
cd ~/projects/web-app
sv find --here  # Shows only web-app scripts
```

### Scenario: Sharing Commands with Team

```bash
# You found a useful command
sv save ./fix-permissions.sh --tags "ops permissions fix"

# Add a helpful description
sv info fix-permissions
# (Shows usage stats, success rate, etc.)

# Team member can search and use it
sv find permissions
sv run fix-permissions
```

## Tips & Tricks

### 1. Quick Save Workflow

```bash
# Skip interactive prompts
sv save script.sh --yes --tags "quick temp"
```

### 2. Version Control

```bash
# Save with git context
sv save deploy.sh --git

# Later, find scripts by repo
sv find --git-repo "yourusername/yourrepo"
```

### 3. Execution Confidence

Before running a script you found:

```bash
# Check the details
sv info script-name

# Look at success rate
# View last execution time
# See who ran it last

# Run with confidence
sv run script-name
```

### 4. Keyboard Shortcuts

Add to your shell rc file:

```bash
# Quick save
alias svs='sv save'

# Quick find
alias svf='sv find'

# Quick run
alias svr='sv run'
```

## Troubleshooting

### "Script not found"

```bash
# List all scripts
sv list

# Check if it's saved with a different name
sv find <partial-name>

# Sync from cloud (when implemented)
sv sync
```

### "Permission denied"

```bash
# Make sure the script is executable
# ScriptVault handles this automatically, but if issues persist:
chmod +x ~/.scriptvault/vault/*
```

### "Git context not detected"

```bash
# Make sure you're in a git repository
git status

# Or save without git context
sv save script.sh  # Will still work, just without git metadata
```

### Health Check

```bash
# Run diagnostics
sv doctor

# Check service status
sv status

# View authentication
sv auth status
```

## Next Steps

- Report issues on [GitHub](https://github.com/ricky-ultimate/scriptvault/issues)
- Star the repo if you find it useful!

## Configuration

Edit `~/.scriptvault/config.json`:

```json
{
  "api_endpoint": "https://api.scriptvault.dev",
  "vault_path": "/home/user/.scriptvault/vault",
  "auto_sync": true,
  "confirm_before_run": true,
  "default_visibility": "private"
}
```

## Directory Structure

```
~/.scriptvault/
├── config.json          # Configuration
├── vault/
│   └── scripts.json     # Your saved scripts
└── history.jsonl        # Execution history (append-only log)
```

---

Happy scripting!
