# ScriptVault

A terminal script vault. Save scripts from any project, find them later from anywhere, track execution history.

## What It Does

ScriptVault stores your shell scripts, Python scripts, and other executables in a personal vault at `~/.scriptvault/`. When you save a script, it captures the directory and git context so you can find relevant scripts later.

Scripts are globally available — save a deployment script in one project, run it from any directory.

## Installation

Requires Rust (install via [rustup.rs](https://rustup.rs)).
```bash
git clone https://github.com/ricky-ultimate/scriptvault
cd scriptvault
cargo install --path .
```

Or build manually:
```bash
cargo build --release
cp target/release/sv ~/.local/bin/
```

## Quick Start
```bash
sv auth login
sv doctor

sv save ./deploy.sh --tags "deployment" --description "Deploy to staging"
sv list
sv run deploy
```

## Commands

### Saving Scripts
```bash
sv save ./script.sh
sv save ./script.sh --name deploy-staging
sv save ./script.sh --tags "deployment docker" --description "Deploy app" --yes
```

Re-saving a script after editing it on disk updates the vault and bumps the patch version:
```bash
sv save ./deploy.sh
```

Or use the explicit update command which errors if the script is not already in the vault:
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
```

Arguments after the script name are passed through to the script. `--verbose` prints the script content, interpreter, and arguments before execution. `--ci` skips all confirmation prompts.

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

### History
```bash
sv history
sv history deploy
sv history --failed
sv history --recent
```

### Exporting
```bash
sv export --format markdown --output SCRIPTS.md
sv export --format json --output scripts.json
```

## How Versioning Works

Scripts start at `v1.0.0`. The patch version increments automatically whenever content changes via `sv save`, `sv update`, or `sv edit`. Version is never decremented.

## Directory Structure
```
~/.scriptvault/
├── config.json
├── vault/
│   └── scripts.json
└── history.jsonl
```

## What Is Not Implemented Yet

These commands exist and respond cleanly but do not do anything useful:

- `sv versions`, `sv diff`, `sv checkout` — require a versioning model not yet built
- `sv share`, `sv team` — require a server
- `sv sync` — requires a remote backend
- `sv recommend` — not yet built
- `sv run --sandbox` — returns an explicit error

## Building from Source
```bash
./build.sh
./build.sh --release
./build.sh --test
./build.sh --release --install
```

## License

MIT
