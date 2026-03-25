# ScriptVault

A terminal script vault for developers. Save scripts from any project, run them from anywhere, sync across machines.

## What It Does

ScriptVault stores your shell scripts, Python scripts, and other executables in a personal vault at `~/.scriptvault/`. When you save a script, it captures the directory and git context so you can find relevant scripts later.

Scripts are globally available — save a deployment script in one project, run it from any directory. Optionally sync to the cloud and access your vault from any machine.

## Installation


**Linux / macOS:**
```bash
curl -fsSL https://raw.githubusercontent.com/ricky-ultimate/scriptvault/main/install.sh | bash
```

**Windows (PowerShell):**
```powershell
iwr https://raw.githubusercontent.com/ricky-ultimate/scriptvault/main/install.ps1 | iex
```

**To release a new version**, just tag and push:
```bash
git tag v0.1.0
git push origin v0.1.0
```

**Build from source**
Requires Rust (install via [rustup.rs](https://rustup.rs)).
```bash
git clone https://github.com/ricky-ultimate/scriptvault
cd scriptvault
./build.sh --release --install
```

Or with Cargo directly:
```bash
cargo install --path .
```

Ensure `~/.local/bin` is in your `PATH`:
```bash
export PATH="$HOME/.local/bin:$PATH"
```

## Quick Start
```bash
sv doctor
sv save ./deploy.sh --tags "deployment" --description "Deploy to staging"
sv list
sv run deploy
```

## Commands

### Auth
```bash
sv auth register
sv auth login --token <API_KEY>
sv auth logout
sv auth status
```

Register creates an account on the ScriptVault cloud and returns an API key. Use `sv auth login --token <key>` to authenticate on additional machines. Without an account, everything works locally — cloud sync is optional.

### Saving Scripts
```bash
sv save ./script.sh
sv save ./script.sh --name deploy-staging
sv save ./script.sh --tags "deployment docker" --description "Deploy app" --yes
```

Re-saving a script after editing it on disk updates the vault and bumps the patch version. Use `sv update` if you want an explicit error when the script is not already in the vault:
```bash
sv update ./deploy.sh
```

### Finding Scripts
```bash
sv list
sv list --recent

sv find deploy
sv find --tag deployment
sv find --language bash
sv find --here
sv find --recent
```

`--here` filters to scripts saved from the current directory or git repo.

### Running Scripts
```bash
sv run deploy
sv run deploy --dry-run
sv run deploy arg1 arg2
sv run deploy --verbose
sv run deploy --ci
sv run deploy --update
```

Arguments after the script name are passed through to the script. `--verbose` prints the script content, interpreter, and arguments before execution. `--ci` skips all confirmation prompts. `--update` pulls the latest version from the cloud before running.

#### Remote Execution
```bash
sv run deploy --ssh user@host
sv run deploy --ssh user@host --ssh-port 2222
sv run deploy --ssh user@host --ssh-identity ~/.ssh/id_ed25519
sv run deploy --ssh user@host --ssh-agent
```

Copies the script to the remote host over SSH, executes it, and cleans up.

### Managing Scripts
```bash
sv info deploy
sv stats deploy
sv cat deploy
sv edit deploy
sv rename deploy deploy-staging
sv copy deploy deploy-prod
sv delete deploy
```

`sv edit` opens the script in `$EDITOR`. `sv info` shows identity and context. `sv stats` shows execution statistics.

### Version History
```bash
sv versions deploy
sv diff deploy v1.0.0 v1.0.2
sv checkout deploy@v1.0.0
```

Scripts start at `v1.0.0`. The patch version increments automatically whenever content changes. Up to 50 versions are retained per script.

### History
```bash
sv history
sv history deploy
sv history --failed
sv history --recent
```

### Adapt
```bash
sv adapt deploy
sv adapt deploy --dry-run
sv adapt deploy --output ./deploy-local.sh
```

Rewrites a script's hardcoded paths to match the current directory context. Useful when running a script saved on another machine.

### Sync
```bash
sv sync
sv sync status
sv sync push
sv sync push --dry-run
sv sync pull
sv sync pull --dry-run
sv sync resolve deploy --take-local
sv sync resolve deploy --take-remote
```

Requires a registered account. Syncs your local vault with the cloud. Conflict resolution is explicit — you choose which version wins.

### Export
```bash
sv export --format markdown --output SCRIPTS.md
sv export --format json --output scripts.json
```

### Storage
```bash
sv storage status
sv storage setup
sv storage test
sv storage info
```

### Diagnostics
```bash
sv doctor
sv status
sv context
```

`sv doctor` checks your environment: config, vault directory, required interpreters, editor, SSH agent, and cloud connectivity. `sv context` shows the current directory and git context.

## How Versioning Works

Scripts start at `v1.0.0`. The patch version increments automatically on any content change via `sv save`, `sv update`, `sv edit`, or `sv adapt`. Version is never decremented. `sv checkout` restores content from a previous version as a new version.

## Cloud Sync

ScriptVault has an optional cloud backend. Register at the CLI with `sv auth register`, then use `sv sync` to push and pull scripts. Conflicts are flagged explicitly and resolved with `sv sync resolve`.

The server is open source and self-hostable. See `server/` in the repository.

## Directory Structure
```
~/.scriptvault/
├── config.json
├── history.jsonl
└── vault/
    ├── index.json
    ├── <script-id>.json
    └── history/
        └── <script-id>/
            ├── manifest.json
            └── <version>.json
```

## Building from Source
```bash
./build.sh
./build.sh --release
./build.sh --test
./build.sh --release --install
```
