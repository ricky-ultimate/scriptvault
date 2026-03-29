# ScriptVault

> Your terminal script vault — save, organize, version, run, and sync scripts across machines.

---

## What is ScriptVault?

ScriptVault (`sv`) is a developer CLI tool that solves a problem every engineer eventually runs into: you write a useful script, and then you lose it, forget what it does, or can't find it when you're on a different machine.

`sv` gives your scripts a home. Every script you save gets versioned, tagged, described, context-aware (knows which project it came from), and optionally synced to the cloud so it follows you everywhere.

```bash
# Save a script
sv save deploy.sh --tags "production deploy" --description "Deploys the app"

# Run it later, anywhere
sv run deploy

# See everything in your vault
sv list
```

---

## Features

- **Save & organize** — store scripts with tags, descriptions, and auto-detected language
- **Version history** — every save creates a version snapshot; diff, checkout, or restore any of them
- **Context-aware** — scripts remember the git repo and directory they were saved from
- **Run with safety** — dangerous pattern detection, dry-run mode, sandboxed execution
- **Remote SSH execution** — run a vault script on any remote host without manually copying it
- **Cloud sync** — push/pull scripts to a hosted server; full conflict detection and resolution
- **Script adaptation** — automatically substitutes paths and home directories when sharing scripts between users or machines
- **Export** — dump your entire vault to Markdown or JSON
- **Health checks** — `sv doctor` verifies your environment, tools, and cloud connectivity

---

## Installation

### Linux / macOS

```bash
curl -fsSL https://raw.githubusercontent.com/ricky-ultimate/scriptvault/main/install.sh | bash
```

Then add to your shell profile if prompted:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/ricky-ultimate/scriptvault/main/install.ps1 | iex
```

### Build from Source

Requires [Rust](https://rustup.rs/) (stable).

```bash
git clone https://github.com/ricky-ultimate/scriptvault
cd scriptvault
./build.sh --release --install
```

Build options:

```bash
./build.sh             # debug build
./build.sh --release   # optimized release build
./build.sh --test      # build + run tests
./build.sh --install   # build + install to ~/.local/bin
```

---

## Quick Start

```bash
# 1. Set a local username (no account needed to get started)
sv auth login --token myname

# 2. Save a script
sv save backup.sh --tags "server backup" --description "Daily backup"

# 3. List your vault
sv list

# 4. Run a script
sv run backup

# 5. Check health
sv doctor
```

To enable cloud sync across machines:

```bash
# Create an account
sv auth register

# On another machine, log in with the API key you received
sv auth login --token sv_4a2f...c91e

# Push your local scripts to the cloud
sv sync push

# Pull scripts on a new machine
sv sync pull
```

---

## Usage

```
sv <COMMAND> [OPTIONS]
```

### Command Overview

| Command | Description |
|---------|-------------|
| `sv auth` | Manage authentication (register, login, logout, status) |
| `sv save <file>` | Save a script to the vault |
| `sv update <file>` | Update an existing script from a file |
| `sv list` | List all scripts in your vault |
| `sv find / search` | Search scripts by name, tag, language, or context |
| `sv info <name>` | Show detailed info about a script |
| `sv cat <name>` | Print script content to stdout |
| `sv run <name>` | Run a script from the vault |
| `sv edit <name>` | Edit a script in your `$EDITOR` |
| `sv rename <old> <new>` | Rename a script |
| `sv copy <src> <dest>` | Copy a script under a new name |
| `sv delete <name>` | Delete a script from the vault |
| `sv history` | Show execution history |
| `sv stats <name>` | Show execution statistics for a script |
| `sv versions <name>` | List all versions of a script |
| `sv diff <name> <v1> <v2>` | Diff two versions of a script |
| `sv checkout <name>@<ver>` | Restore a script to a previous version |
| `sv context` | Show the current detected context (directory, git, env) |
| `sv adapt <name>` | Adapt a script's paths to the current environment |
| `sv sync` | Sync scripts with the cloud |
| `sv export` | Export vault to Markdown or JSON |
| `sv storage` | Manage storage configuration |
| `sv doctor` | Run a full environment health check |
| `sv status` | Quick vault status overview |

For detailed usage of every command with examples and expected output, see the [Command Reference](./COMMANDS.md).

---

## How It Works

### Local Storage

Scripts are stored as JSON files in `~/.scriptvault/vault/`. Each file contains the script content alongside all its metadata — tags, description, language, execution stats, version, sync state, and the context it was saved from. No database required.

```
~/.scriptvault/
├── config.json           # your configuration and credentials
├── history.jsonl         # execution log (append-only, rotates at 1000 entries)
└── vault/
    ├── index.json         # name → id lookup index
    ├── <script-id>.json   # one file per script
    └── history/
        └── <script-id>/
            ├── manifest.json       # version list
            ├── v1.0.0.json         # version snapshots (max 50)
            └── v1.0.1.json
```

### Cloud Sync

When authenticated, scripts can be pushed to and pulled from a hosted server backed by PostgreSQL (metadata) and Cloudflare R2 (script content). Sync uses hash-based conflict detection — if both local and remote have changed since the last sync, the script is flagged as a conflict for you to resolve manually.

```bash
sv sync push               # push all pending-push scripts
sv sync pull               # pull remote-only and pending-pull scripts
sv sync status             # view sync state of every script
sv sync resolve deploy --take-local    # resolve a conflict
```

### Context Awareness

When you save a script, ScriptVault captures your current directory and git repository. When you run `sv find --here`, it only returns scripts that match your current context — same git repo or same directory tree. When sharing scripts with `sv adapt`, path substitutions are applied automatically.

---

## Script Language Support

Language is detected automatically from the file extension:

| Extension | Language | Interpreter |
|-----------|----------|-------------|
| `.sh` | Shell | `sh` |
| `.bash` | Bash | `bash` |
| `.py` | Python | `python3` |
| `.js` | JavaScript | — |
| `.rb` | Ruby | `ruby` |
| `.pl` | Perl | `perl` |
| `.ps1` | PowerShell | `powershell -File` |
| `.bat` / `.cmd` | Batch | — |

---

## Safety

ScriptVault checks every script for dangerous patterns before execution and warns you if any are detected:

```
rm -rf /         rm -rf /*        mkfs
dd if=           > /dev/sda       :(){ :|:& };:
chmod -R 777 /   > /dev/sd        mkfs.ext
```

You can also use `--dry-run` to inspect what a script would do without executing it, or `--sandbox` to run in an isolated temp directory with a stripped-down environment.

> **Note:** `--sandbox` provides directory isolation and environment clearing — it does not provide kernel-level sandboxing or syscall filtering.

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `SCRIPTVAULT_HOME` | Override the default `~/.scriptvault` directory |
| `SCRIPTVAULT_CI` | Set to `1` to disable all interactive prompts (equivalent to `--ci`) |
| `SCRIPTVAULT_API_ENDPOINT` | Override the default API server URL |
| `EDITOR` / `VISUAL` | Editor used by `sv edit` |

---

## Configuration

Your config lives at `~/.scriptvault/config.json` and is managed automatically. Key fields:

| Field | Default | Description |
|-------|---------|-------------|
| `api_endpoint` | `https://scriptvault.fly.dev/v1` | Cloud API URL |
| `vault_path` | `~/.scriptvault/vault` | Local script storage path |
| `auto_sync` | `false` | Reserved for future automatic background sync |
| `confirm_before_run` | `true` | Whether `sv run` prompts for confirmation |
| `default_visibility` | `private` | Default visibility for new scripts |

You can relocate your vault with:

```bash
sv storage setup
```

---

## Development

### Prerequisites

- Rust stable (install via [rustup.rs](https://rustup.rs/))

### Build & Test

```bash
# Clone the repo
git clone https://github.com/ricky-ultimate/scriptvault
cd scriptvault

# Run tests
cargo test

# Debug build
cargo build

# Run the demo walkthrough (requires a debug build)
./demo.sh
```

### Project Structure

```
scriptvault/
├── src/
│   ├── main.rs          # CLI entrypoint and command dispatch
│   ├── lib.rs           # Library root (re-exports for integration tests)
│   ├── cli.rs           # Clap argument definitions
│   ├── script.rs        # Core data types (Script, SyncStatus, etc.)
│   ├── vault.rs         # Save, list, find, delete, export operations
│   ├── execution.rs     # Script running, history, sandboxing
│   ├── auth.rs          # Register, login, logout
│   ├── config.rs        # Config loading and saving
│   ├── context.rs       # Git and directory context detection
│   ├── adapt.rs         # Path substitution for script adaptation
│   ├── versions.rs      # Version snapshot storage and diffing
│   ├── utils.rs         # Doctor and status checks
│   ├── storage/
│   │   ├── mod.rs       # StorageBackend trait definition
│   │   ├── local.rs     # Local filesystem implementation
│   │   └── commands.rs  # `sv storage` subcommands
│   └── sync/
│       ├── mod.rs       # Push/pull/resolve entry points
│       ├── manager.rs   # Sync logic and conflict detection
│       └── remote.rs    # HTTP remote backend implementation
├── server/              # Cloud sync server (Axum + PostgreSQL + R2)
├── tests/
│   └── integration_test.rs
├── build.sh             # Dev build helper
├── demo.sh              # Interactive feature demo
├── install.sh           # Unix installer
└── install.ps1          # Windows installer
```

### Running the Server Locally

```bash
cd server
cp .env.example .env     # fill in DATABASE_URL, R2_*, etc.
cargo run
```

Or with Docker:

```bash
cd server
docker build -t scriptvault-server .
docker run -p 8080:8080 --env-file .env scriptvault-server
```

---

## Releases

Binaries are built automatically via GitHub Actions on every version tag push (`v*`). The following targets are supported:

| Platform | Architecture | File |
|----------|-------------|------|
| Linux | x86_64 (glibc) | `sv-linux-x86_64.tar.gz` |
| Linux | x86_64 (musl) | `sv-linux-x86_64-musl.tar.gz` |
| Linux | aarch64 | `sv-linux-aarch64.tar.gz` |
| macOS | x86_64 | `sv-macos-x86_64.tar.gz` |
| macOS | Apple Silicon | `sv-macos-aarch64.tar.gz` |
| Windows | x86_64 | `sv-windows-x86_64.zip` |

SHA256 checksums are included with every release.

To cut a release:

```bash
git tag v0.2.0
git push origin v0.2.0
```

---

## License

MIT — see [LICENSE](./LICENSE).

---

## Author

Built by [リッキー](https://github.com/ricky-ultimate).
