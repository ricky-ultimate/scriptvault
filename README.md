# ScriptVault

> Your terminal script time-machine. Never lose a useful script again.

**ScriptVault** is a local-first, privacy-focused CLI tool that helps developers save, organize, find, and run their scripts with context awareness. Think of it as Git for your scripts, combined with a time-machine that remembers where and when you used them.

## Features

- **Save & Organize**: Store scripts with automatic context detection (directory, git repo, environment)
- **Smart Search**: Find scripts by name, tags, context, or content
- **Safe Execution**: Preview scripts before running, with safety checks for dangerous commands
- **Version Control**: Track script versions and compare changes over time
- **Context Aware**: Scripts remember where they were created and where they work best
- **Execution History**: Complete audit trail of every script run
- **Team Collaboration**: Share scripts with your team (coming soon)
- **Git Integration**: Automatically detect and tag scripts by git repository

## Quick Start

### Installation

```bash
# Via cargo
cargo install scriptvault

# Or clone and build
git clone https://github.com/ricky-ultimate/scriptvault
cd scriptvault
cargo build --release
```

### First Steps

```bash
# Authenticate (local mode for now)
sv auth login --token local

# Save your first script
sv save ./deploy.sh --tags "deployment docker" --description "Deploy to staging"

# Find it later
sv find deployment

# Run it with confidence
sv run deploy

# View execution history
sv history
```

## Usage

### Saving Scripts

```bash
# Save a script with automatic context detection
sv save ./script.sh

# Add tags and description
sv save ./backup.sh --tags "database backup" --description "Daily DB backup"

# Skip interactive prompts
sv save ./script.sh --yes --tags "tag1 tag2"
```

### Finding Scripts

```bash
# Search by name or content
sv find backup

# Find scripts for current directory
sv find --here

# Filter by tag
sv find --tag deployment

# Filter by language
sv find --language python
```

### Running Scripts

```bash
# Run a script
sv run deploy-staging

# Dry run (preview without executing)
sv run deploy-staging --dry-run

# Pass arguments to the script
sv run backup --args production today

# Verbose output
sv run script --verbose
```

### Script Information

```bash
# List all scripts
sv list

# Get detailed info
sv info deploy-staging

# View execution history
sv history
sv history deploy-staging
sv history --failed

# View statistics
sv stats deploy-staging
```

### Context & Organization

```bash
# Show current context
sv context

# Get recommendations for current project
sv recommend

# Export scripts as documentation
sv export --format markdown --output SCRIPTS.md
```

## Architecture

ScriptVault is built with Rust and uses:

- **Local Storage**: All scripts stored in `~/.scriptvault/`
- **JSONL Format**: Append-only execution history
- **Git Integration**: Automatic repository detection
- **Safe Execution**: Sandboxed script running with safety checks

### Directory Structure

```
~/.scriptvault/
├── config.json          # Configuration
├── vault/
│   └── scripts.json     # Your scripts
└── history.jsonl        # Execution history
```

## Security

- **Local First**: All data stored locally by default
- **Safety Checks**: Scans for dangerous commands before execution
- **Execution Preview**: Always shows what will run before running it
- **Audit Trail**: Complete history of who ran what, when, and where
- **Sandboxed Execution**: Option to run scripts in isolated environments (coming soon)

## Roadmap

### MVP (Current)
- [x] Save and organize scripts locally
- [x] Context detection (directory, git)
- [x] Search and find scripts
- [x] Safe execution with preview
- [x] Execution history
- [ ] Basic CLI polish

### Phase 2
- [ ] Cloud sync (optional)
- [ ] Team sharing
- [ ] Web dashboard
- [ ] OAuth authentication
- [ ] Script versioning
- [ ] Diff between versions

### Phase 3
- [ ] Public script library
- [ ] Script templates
- [ ] AI-powered recommendations
- [ ] CI/CD integration
- [ ] Browser extension
- [ ] VS Code extension

## Contributing

Contributions are welcome! This is an early-stage project and we'd love your help.

```bash
# Clone the repo
git clone https://github.com/ricky-ultimate/scriptvault
cd scriptvault

# Build and test
cargo build
cargo test

# Run locally
cargo run -- save ./test.sh
```


## Acknowledgments

Inspired by the daily frustration of losing track of useful scripts. Built with Rust and ❤️.
