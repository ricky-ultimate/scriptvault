# ScriptVault — Command Reference

A complete reference for every `sv` command, including flags, examples, and expected output.

---

## Table of Contents

- [Authentication](#authentication)
  - [sv auth register](#sv-auth-register)
  - [sv auth login](#sv-auth-login)
  - [sv auth logout](#sv-auth-logout)
  - [sv auth status](#sv-auth-status)
- [Saving & Managing Scripts](#saving--managing-scripts)
  - [sv save](#sv-save-file)
  - [sv update](#sv-update-file)
  - [sv list](#sv-list)
  - [sv find / sv search](#sv-find--sv-search)
  - [sv info](#sv-info-name)
  - [sv cat](#sv-cat-name)
  - [sv edit](#sv-edit-name)
  - [sv rename](#sv-rename-old-name-new-name)
  - [sv copy](#sv-copy-source-dest)
  - [sv delete](#sv-delete-name)
- [Running Scripts](#running-scripts)
  - [sv run](#sv-run-name-args)
- [History & Statistics](#history--statistics)
  - [sv history](#sv-history)
  - [sv stats](#sv-stats-name)
- [Version Control](#version-control)
  - [sv versions](#sv-versions-name)
  - [sv diff](#sv-diff-name-version1-version2)
  - [sv checkout](#sv-checkout-nameversion)
- [Context & Adaptation](#context--adaptation)
  - [sv context](#sv-context)
  - [sv adapt](#sv-adapt-name)
- [Cloud Sync](#cloud-sync)
  - [sv sync / sv sync pull](#sv-sync--sv-sync-pull)
  - [sv sync push](#sv-sync-push)
  - [sv sync status](#sv-sync-status)
  - [sv sync resolve](#sv-sync-resolve-name)
- [Export](#export)
  - [sv export](#sv-export)
- [Storage](#storage)
  - [sv storage status](#sv-storage-status)
  - [sv storage setup](#sv-storage-setup)
  - [sv storage test](#sv-storage-test)
  - [sv storage info](#sv-storage-info)
- [Diagnostics](#diagnostics)
  - [sv doctor](#sv-doctor)
  - [sv status](#sv-status)

---

## Authentication

ScriptVault works in two modes. **Local mode** requires no account — you just set a username and all scripts stay on your machine. **Cloud mode** requires registering for an account, which gives you an API key you can use to sync scripts across machines.

---

### `sv auth register`

Creates a new cloud account and stores your API key locally. You only need to do this once. After registering, copy your API key somewhere safe — it is shown only once.

```bash
sv auth register
sv auth register --username yourname
```

**Flags:**

| Flag | Description |
|------|-------------|
| `--username <NAME>` | Pre-fill the username instead of being prompted |

**Example — interactive:**
```
$ sv auth register

Registering...
Registered as: yourname

API key: sv_4a2f9c1e3d7b8f2a...

Save this key. To authenticate on another machine:
  sv auth login --token sv_4a2f9c1e3d7b8f2a...
```

**Example — with flag:**
```
$ sv auth register --username yourname

Registering...
Registered as: yourname

API key: sv_4a2f9c1e3d7b8f2a...
...
```

**Error — username taken:**
```
Error: That username is already taken
```

---

### `sv auth login`

Logs in with an existing API key (switches to cloud mode) or sets a local username (stays in local mode). ScriptVault detects which mode to use based on whether the value you pass starts with `sv_`.

```bash
# Cloud login with an API key
sv auth login --token sv_4a2f9c1e3d7b8f2a...

# Set a local username (no account or network needed)
sv auth login --token myusername
```

**Flags:**

| Flag | Description |
|------|-------------|
| `--token <VALUE>` | API key (`sv_...`) for cloud mode, or a plain name for local mode |

**Example — cloud login:**
```
$ sv auth login --token sv_4a2f9c1e3d7b8f2a...

Logged in as: yourname
```

**Example — local username:**
```
$ sv auth login --token myname

Username set to: myname
```

**Error — invalid API key:**
```
Error: Invalid API key
```

---

### `sv auth logout`

Clears your stored credentials and returns to an unauthenticated local state.

```bash
sv auth logout
```

**Example:**
```
$ sv auth logout

Logged out
```

---

### `sv auth status`

Shows your current authentication mode, username, and whether you are set up for cloud sync.

```bash
sv auth status
```

**Example — local mode:**
```
$ sv auth status

ScriptVault Auth Status

  Mode: Local
  Username: myname
```

**Example — not configured:**
```
$ sv auth status

ScriptVault Auth Status

  Mode: Local
  Username: not set

  Run 'sv auth register' to create an account
  Or 'sv auth login' to set a local username
```

**Example — cloud mode:**
```
$ sv auth status

ScriptVault Auth Status

  Mode: Cloud
  Username: yourname
```

---

## Saving & Managing Scripts

---

### `sv save <file>`

Saves a script file into your vault. ScriptVault reads the file, detects the language from its extension, captures your current directory and git context, and stores it all together. If a script with the same name already exists, the content is compared — if it changed, the version is bumped; if nothing changed, the save is skipped.

```bash
sv save deploy.sh
sv save backup.sh --name my-backup
sv save cleanup.py --tags "maintenance cron" --description "Weekly cleanup"
sv save build.sh --yes    # skip all interactive prompts
```

**Flags:**

| Flag | Description |
|------|-------------|
| `--name <NAME>` | Override the vault name (defaults to the filename without extension) |
| `--tags <TAGS>` | Space-separated list of tags |
| `--description <DESC>` | Short description of what the script does |
| `--yes` | Skip all interactive prompts and use provided values as-is |

**Example — interactive:**
```
$ sv save deploy.sh

Saving script to vault...

  Directory: /home/user/myproject
  Git Repo: github.com/user/myproject

Tags (space-separated): deploy production
Description (optional): Deploys the app to production

✓ Saved: deploy v1.0.0
  ID: 3f8a1c2d-4b5e-6f7a-8b9c-0d1e2f3a4b5c
  Tags: deploy, production
```

**Example — non-interactive:**
```
$ sv save deploy.sh --tags "deploy production" --description "Deploys the app" --yes

✓ Saved: deploy v1.0.0
  ID: 3f8a1c2d-...
  Tags: deploy, production
```

**Example — re-saving with changes (version bumped):**
```
$ sv save deploy.sh --yes

✓ Saved: deploy v1.0.1
  ID: 3f8a1c2d-...
```

**Example — re-saving with no changes:**
```
$ sv save deploy.sh --yes

i No changes: deploy
```

**Error — file not found:**
```
Error: Script file not found: deploy.sh
```

---

### `sv update <file>`

Updates an already-saved script with new content from a file on disk. The script must already exist in the vault — use `sv save` to add new scripts. The version is bumped automatically if the content changed.

```bash
sv update deploy.sh
sv update deploy.sh --name my-deploy    # if the vault name differs from the filename
```

**Flags:**

| Flag | Description |
|------|-------------|
| `--name <NAME>` | Specify the vault script name if it differs from the filename |

**Example:**
```
$ sv update deploy.sh

✓ Updated: deploy v1.0.1 -> v1.0.2
```

**Example — no changes detected:**
```
$ sv update deploy.sh

i No changes: deploy
```

**Error — script not in vault:**
```
Error: Script 'deploy' not found in vault. Use 'sv save' to add it first.
```

---

### `sv list`

Lists all scripts in your vault with their version, description, and tags.

```bash
sv list
sv list --recent              # sort by most recently run
sv list --mine                # only scripts you authored
sv list --limit 20 --offset 0 # pagination
```

**Flags:**

| Flag | Default | Description |
|------|---------|-------------|
| `--recent` | — | Sort by last run time instead of name |
| `--mine` | — | Filter to only scripts you authored |
| `--limit <N>` | 50 | Maximum number of scripts to show |
| `--offset <N>` | 0 | Number of scripts to skip (for pagination) |

**Example:**
```
$ sv list

Scripts

  deploy v1.0.2
    Deploys the app to production
    Tags: deploy, production

  backup v1.0.0
    Daily database backup
    Tags: server, backup

  cleanup v1.0.1
    Tags: maintenance
```

**Example — empty vault:**
```
$ sv list

No scripts saved yet.
```

---

### `sv find` / `sv search`

Searches your vault by name, description, tag, language, or context. `sv find` and `sv search` are identical — use whichever feels natural.

```bash
sv find deploy
sv find --tag production
sv find --language python
sv find --here              # only scripts from the current project
sv find --recent            # sort by most recently run
sv search backup            # same as sv find
```

**Flags:**

| Flag | Description |
|------|-------------|
| `<QUERY>` | Free-text search across name, description, and tags |
| `--tag <TAG>` | Filter to scripts with this exact tag |
| `--language <LANG>` | Filter by language (e.g. `bash`, `python`) |
| `--here` | Only show scripts saved from the current directory or git repo |
| `--recent` | Sort results by most recently run |

**Example:**
```
$ sv find deploy

Scripts

NAME                           VERSION    USES     LAST RUN
──────────────────────────────────────────────────────────────────────
deploy                         v1.0.2     5        2 hours ago
deploy-staging                 v1.0.0     1        3 days ago
```

**Example — context-aware search:**
```
$ sv find --here

Scripts

NAME                           VERSION    USES     LAST RUN
──────────────────────────────────────────────────────────────────────
deploy                         v1.0.2     5        2 hours ago
build                          v1.0.0     12       1 hour ago
```

**Example — no results:**
```
$ sv find nonexistent

No scripts found matching your criteria.
```

---

### `sv info <name>`

Shows all metadata for a script — version, language, author, tags, description, context it was saved from, and execution summary.

```bash
sv info deploy
```

**Example:**
```
$ sv info deploy

deploy

  Version:     v1.0.2
  Language:    bash
  Author:      yourname
  Created:     2026-03-20 10:00:00
  Description: Deploys the app to production
  Tags:        deploy, production

  Context:
    Directory: /home/user/myproject
    Git repo:  github.com/user/myproject
    Branch:    main

  5 runs, 100.0% success, last run 2026-03-27
  Run sv stats deploy for full execution breakdown
```

**Example — never run:**
```
$ sv info backup

backup

  Version:     v1.0.0
  Language:    bash
  Author:      yourname
  Created:     2026-03-25 08:30:00
  Description: Daily database backup
  Tags:        server, backup

  Context:
    Directory: /home/user/infra

  Never run
  Run sv stats backup for full execution breakdown
```

---

### `sv cat <name>`

Prints the raw content of a script to stdout. Useful for inspecting a script or piping it elsewhere.

```bash
sv cat deploy
sv cat deploy > restored-deploy.sh    # restore content to a file
sv cat deploy | grep "echo"           # pipe to other tools
```

**Example:**
```
$ sv cat deploy

#!/usr/bin/env bash
set -e
echo "Deploying..."
git pull origin main
docker compose up -d --build
echo "Done."
```

---

### `sv edit <name>`

Opens a script in your `$EDITOR` (falling back to `$VISUAL`, then `vi`). If you save and close the editor with changes, the script is updated in the vault with a bumped version. If you close without changes, nothing happens.

```bash
sv edit deploy
```

**Example — after saving changes:**
```
$ sv edit deploy

✓ Updated: deploy v1.0.2 -> v1.0.3
```

**Example — closed without changes:**
```
$ sv edit deploy

No changes made
```

**Example — editor cancelled or exited with error:**
```
$ sv edit deploy

Edit cancelled
```

> Set your preferred editor with `export EDITOR=nvim` (or `nano`, `vim`, `code --wait`, etc.) in your shell profile.

---

### `sv rename <old-name> <new-name>`

Renames a script in the vault. The script ID, content, and all history are preserved under the new name.

```bash
sv rename deploy deploy-production
```

**Example:**
```
$ sv rename deploy deploy-production

✓ Renamed: deploy -> deploy-production
```

**Error — new name already taken:**
```
Error: A script named 'deploy-production' already exists
```

**Error — source not found:**
```
Error: Script not found: deploy
```

---

### `sv copy <source> <dest>`

Creates a copy of a script under a new name. The copy has its own ID and starts with a fresh execution history and zero run count. The content and version string are preserved from the source.

```bash
sv copy deploy deploy-staging
```

**Example:**
```
$ sv copy deploy deploy-staging

✓ Copied: deploy -> deploy-staging
```

**Error — destination already exists:**
```
Error: A script named 'deploy-staging' already exists
```

---

### `sv delete <name>`

Permanently deletes a script from the vault, along with its entire version history and execution records. Prompts for confirmation unless `--yes` is passed.

```bash
sv delete deploy
sv delete deploy --yes
```

**Flags:**

| Flag | Description |
|------|-------------|
| `--yes` | Skip confirmation prompt |

**Example — interactive:**
```
$ sv delete deploy

deploy
  Deploys the app to production
  Tags: deploy, production
  Uses: 5

Delete this script? [y/N]: y
✓ Deleted: deploy
```

**Example — skipping confirmation:**
```
$ sv delete deploy --yes

✓ Deleted: deploy
```

**Error — not found:**
```
Error: Script not found: deploy
```

---

## Running Scripts

---

### `sv run <name> [args...]`

Runs a script from the vault. The script is written to a temp file and executed with the appropriate interpreter. A minimal, safe set of environment variables is passed to the process. If the script contains dangerous patterns, a warning is shown before execution.

```bash
sv run deploy
sv run backup --verbose
sv run myscript --dry-run
sv run myscript --ci
sv run greet -- --name Alice         # pass arguments to the script itself
sv run deploy --update               # pull latest from cloud before running
sv run deploy --ssh user@prod-server # run on a remote host over SSH
```

**Flags:**

| Flag | Description |
|------|-------------|
| `--dry-run` | Show the script preview without executing |
| `--verbose` | Print the script content before executing |
| `--ci` | Skip all interactive prompts (also triggered by `SCRIPTVAULT_CI=1`) |
| `--confirm` | Always prompt for confirmation before running, regardless of config |
| `--sandbox` | Run in an isolated temp directory with a stripped environment |
| `--update` | Pull the latest cloud version before running (requires auth) |
| `--ssh <USER@HOST>` | Execute the script on a remote host over SSH |
| `--ssh-port <PORT>` | SSH port to use with `--ssh` (default: `22`) |
| `--ssh-identity <PATH>` | Path to an SSH identity file (private key) |
| `--ssh-agent` | Forward the local SSH agent to the remote host |

**Script preview (shown before every run):**
```
╭────────────────────────────────────────────────────────────╮
│  deploy v1.0.2
├────────────────────────────────────────────────────────────┤
│  Tags: deploy, production
│  Description: Deploys the app to production
│
│  Language: bash
│  Directory: /home/user/myproject
│  Success rate: 100.0% (5/5)
╰────────────────────────────────────────────────────────────╯

Run this script? [Y/n]:
```

**Example — successful run:**
```
$ sv run deploy --ci

╭────────────────────────────────────────────────────────────╮
│  deploy v1.0.2
...
╰────────────────────────────────────────────────────────────╯

Executing...

Deploying...
Already up to date.
Recreating app ... done

Completed in 3.42s
```

**Example — dry run:**
```
$ sv run deploy --dry-run

╭────────────────────────────────────────────────────────────╮
│  deploy v1.0.2
...
╰────────────────────────────────────────────────────────────╯

Dry run complete. Script was not executed.
```

**Example — verbose:**
```
$ sv run deploy --verbose --ci

  Content:
    #!/usr/bin/env bash
    set -e
    echo "Deploying..."
    ...

Executing...

Deploying...

Completed in 3.42s
```

**Example — dangerous script warning:**
```
$ sv run risky --ci

Warning: This script contains potentially dangerous commands.

Executing...
```

**Example — passing arguments to the script:**
```
$ sv run greet --ci -- --name Alice

Hello, Alice!

Completed in 0.05s
```

**Example — failed run:**
```
$ sv run deploy --ci

Executing...

./deploy.sh: line 4: docker: command not found

Failed with exit code 127 in 0.11s
```

**Example — remote SSH execution:**
```
$ sv run deploy --ssh user@prod-server --ssh-port 2222 --ci

Executing...

Deploying...
Already up to date.

Remote execution completed successfully.
```

**Example — SSH dry run:**
```
$ sv run deploy --ssh user@prod-server --dry-run

Dry run — remote execution plan:
  Target:        user@prod-server
  Port:          2222
  Remote path:   /tmp/sv_3f8a1c2d....sh
  Script:        deploy v1.0.2

Dry run complete. Script was not executed.
```

**Error — script not found:**
```
Error: Script not found: deploy
```

**Error — interpreter not found:**
```
Error: Required interpreter 'python3' not found in PATH.
Install it before running this script.
```

**Error — `--update` without auth:**
```
Error: sv run --update requires cloud sync.
Run 'sv auth login --token <API_KEY>' first.
```

> **Note on `--sandbox`:** This mode uses a private temp directory and strips the environment down to a minimal set. It does **not** provide kernel-level sandboxing or syscall filtering.

---

## History & Statistics

---

### `sv history`

Shows your recent script execution history — time, script name, user, exit code, and duration. Displays the 20 most recent entries by default, or 10 with `--recent`. Scripts that have been deleted appear as `[deleted]`.

```bash
sv history
sv history deploy         # history for a specific script
sv history --failed       # only failed runs (exit code != 0)
sv history --recent       # limit to last 10 entries
```

**Flags:**

| Flag | Description |
|------|-------------|
| `<SCRIPT>` | Filter history to a specific script name |
| `--failed` | Only show runs that exited with a non-zero code |
| `--recent` | Show only the last 10 entries |

**Example:**
```
$ sv history

Execution History

TIME                 SCRIPT                 USER            EXIT CODE  DURATION
────────────────────────────────────────────────────────────────────────────────
2026-03-27 14:22:01  deploy                 yourname        0          3.42s
2026-03-27 13:10:45  backup                 yourname        0          0.83s
2026-03-26 09:05:11  deploy                 yourname        1          0.11s
2026-03-25 08:00:00  [deleted]              yourname        0          1.20s
```

**Example — filtered by script:**
```
$ sv history deploy

Execution History

TIME                 SCRIPT                 USER            EXIT CODE  DURATION
────────────────────────────────────────────────────────────────────────────────
2026-03-27 14:22:01  deploy                 yourname        0          3.42s
2026-03-26 09:05:11  deploy                 yourname        1          0.11s
```

**Example — no history:**
```
$ sv history

No execution history found.
```

**Note — filtering by deleted script:**
```
$ sv history old-script

Note: 'old-script' is not in your vault (it may have been deleted).
History for deleted scripts cannot be filtered by name.
Run 'sv history' to see all records including those marked [deleted].
```

---

### `sv stats <name>`

Shows detailed execution statistics for a specific script — content info, run counts, success rate, average runtime, and last run details.

```bash
sv stats deploy
```

**Example:**
```
$ sv stats deploy

deploy

  Content:
    Language:  bash
    Size:      412 bytes
    Lines:     18
    Hash:      4a2f1c8e9d3b7f1a

  Execution:
    Total runs:   6
    Successful:   5
    Failed:       1
    Success rate: 83.3%
    Avg runtime:  1.94s

  Last Run:
    Time: 2026-03-27 14:22:01 UTC
    By:   yourname
```

**Example — never run:**
```
$ sv stats backup

backup

  Content:
    Language:  bash
    Size:      280 bytes
    Lines:     12
    Hash:      9c3e7a2f1d4b8e0f

  Execution:
    Total runs:   0
    Successful:   0
    Failed:       0
    Success rate: 0.0%
```

---

## Version Control

ScriptVault automatically saves a version snapshot every time a script is created or updated. Up to 50 snapshots are kept per script; older ones are pruned automatically.

---

### `sv versions <name>`

Lists all saved version snapshots for a script, showing when each was saved, who saved it, line count, and size.

```bash
sv versions deploy
```

**Example:**
```
$ sv versions deploy

deploy

VERSION      SAVED AT               AUTHOR          LINES    SIZE
──────────────────────────────────────────────────────────────────────
v1.0.0       2026-03-20 10:00:00    yourname        12       280b
v1.0.1       2026-03-22 15:30:00    yourname        15       340b
v1.0.2       2026-03-27 14:00:00    yourname        18       412b
```

**Example — no version history:**
```
$ sv versions deploy

No version history for: deploy
```

---

### `sv diff <name> <version1> <version2>`

Shows a line-by-line diff between two saved versions of a script. Lines only in `version1` are shown in red with `-`, lines only in `version2` in green with `+`, and unchanged lines are shown as-is.

```bash
sv diff deploy v1.0.1 v1.0.2
```

**Example:**
```
$ sv diff deploy v1.0.1 v1.0.2

deploy v1.0.1 vs v1.0.2

  #!/usr/bin/env bash
  set -e
- echo "Deploying..."
+ echo "Starting deployment..."
+ echo "Target: production"
  git pull origin main
  docker compose up -d --build
- echo "Done."
+ echo "Deployment complete."

3 line(s) changed
```

**Example — no differences:**
```
$ sv diff deploy v1.0.1 v1.0.1

deploy v1.0.1 vs v1.0.1

  #!/usr/bin/env bash
  ...

0 line(s) changed
```

---

### `sv checkout <name>@<version>`

Restores a script to the content of a previous version. The restored content is saved as a **new version** (patch bump on top of the current latest), so your full history is preserved. You can always diff or roll back further.

```bash
sv checkout deploy@v1.0.1
```

**Example:**
```
$ sv checkout deploy@v1.0.1

✓ Restored: deploy from v1.0.1 as v1.0.3
```

**Error — wrong format:**
```
Error: Invalid format. Use: sv checkout <script>@<version>
```

**Error — version not found:**
```
Error: version v1.0.9 not found for script deploy
```

---

## Context & Adaptation

---

### `sv context`

Displays what ScriptVault currently detects about your environment — working directory, git repository, branch, and relevant shell variables. This is the same context captured when you run `sv save`.

```bash
sv context
```

**Example — inside a git repo:**
```
$ sv context

Current Context

  Directory: /home/user/myproject
  Git Repo:  github.com/user/myproject
  Branch:    main

  Environment:
    SHELL: /bin/zsh
    USER:  yourname
```

**Example — outside a git repo:**
```
$ sv context

Current Context

  Directory: /home/user/scripts
  Git Repo:  Not in a git repository

  Environment:
    SHELL: /bin/bash
    USER:  yourname
```

---

### `sv adapt <name>`

Adapts a script to your current environment by detecting differences between the context it was saved in and your current context, then substituting directory paths and home directories accordingly. This is particularly useful when sharing scripts between users or running scripts saved on a different machine.

```bash
sv adapt deploy
sv adapt deploy --dry-run           # preview substitutions without applying
sv adapt deploy --output out.sh     # write adapted content to a file instead
sv adapt deploy --yes               # skip confirmation prompt
```

**Flags:**

| Flag | Description |
|------|-------------|
| `--dry-run` | Show what would change without applying anything |
| `--output <PATH>` | Write the adapted script to a file instead of updating the vault |
| `--yes` | Skip the confirmation prompt |

**Example — adaptation needed:**
```
$ sv adapt deploy

Adapt Preview

  Script:  deploy
  Context: /home/alice/myproject -> /home/bob/myproject

  Substitutions:
    [directory]      /home/alice/myproject -> /home/bob/myproject
    [home directory] /home/alice           -> /home/bob

  Diff:
  - cd /home/alice/myproject && ./run.sh
  + cd /home/bob/myproject && ./run.sh
  - source /home/alice/.env
  + source /home/bob/.env

Apply adaptations and save to vault? [Y/n]: y
✓ Adapted: deploy v1.0.2 -> v1.0.3
```

**Example — dry run:**
```
$ sv adapt deploy --dry-run

Adapt Preview

  Script:  deploy
  Context: /home/alice/myproject -> /home/bob/myproject

  Substitutions:
    [directory] /home/alice/myproject -> /home/bob/myproject

  Diff:
  - cd /home/alice/myproject && ./run.sh
  + cd /home/bob/myproject && ./run.sh

Dry run complete. No changes applied.
```

**Example — write to file:**
```
$ sv adapt deploy --output adapted-deploy.sh --yes

✓ Written to: adapted-deploy.sh
```

**Example — no adaptation needed:**
```
$ sv adapt deploy

i No adaptations needed for deploy

  Script context:  /home/bob/myproject
  Current context: /home/bob/myproject
```

**Example — paths unchanged after substitution:**
```
$ sv adapt deploy

i Script content unchanged after applying context substitutions.
  The script may not reference any context-specific paths.
```

---

## Cloud Sync

Cloud sync requires an account and a valid API key. Run `sv auth register` to create one, or `sv auth login --token sv_...` to authenticate with an existing key.

---

### `sv sync` / `sv sync pull`

Runs a full two-way sync. Pushes scripts with pending local changes, pulls scripts that only exist on the remote, and flags any scripts where both sides have changed since the last sync as conflicts. Running `sv sync` with no subcommand is equivalent to `sv sync pull`.

```bash
sv sync
sv sync pull
sv sync pull --dry-run    # preview what would be pulled without making changes
```

**Flags:**

| Flag | Description |
|------|-------------|
| `--dry-run` | Show what would be pulled without actually pulling |

**Example:**
```
$ sv sync

Pulled (2):
  backup
  deploy-staging
Pushed (1):
  build
Everything is up to date.
```

**Example — already in sync:**
```
$ sv sync

Everything is up to date.
```

**Example — dry run:**
```
$ sv sync pull --dry-run

Dry run — scripts that would be pulled:

  backup         v1.0.0
  deploy-staging v1.0.2

Run 'sv sync pull' without --dry-run to apply.
```

**Example — nothing to pull:**
```
$ sv sync pull --dry-run

Nothing to pull.
```

**Error — not authenticated:**
```
Error: Cloud sync requires authentication.
Run 'sv auth login --token <API_KEY>'
```

---

### `sv sync push`

Pushes all scripts in `pending-push` state to the cloud. These are scripts that have been modified locally since their last sync.

```bash
sv sync push
sv sync push --dry-run
```

**Flags:**

| Flag | Description |
|------|-------------|
| `--dry-run` | Show what would be pushed without actually pushing |

**Example:**
```
$ sv sync push

Pushed (2):
  deploy
  backup
```

**Example — nothing pending:**
```
$ sv sync push

Nothing to push.
```

**Example — dry run:**
```
$ sv sync push --dry-run

Dry run — scripts that would be pushed:

  deploy    v1.0.3
  backup    v1.0.1

Run 'sv sync push' without --dry-run to apply.
```

---

### `sv sync status`

Shows the sync state of every script in your vault in a table. Also lists any conflicts with instructions for resolving them.

```bash
sv sync status
```

**Example:**
```
$ sv sync status

Sync Status

NAME                           VERSION    STATUS          LAST SYNCED
──────────────────────────────────────────────────────────────────────────────
backup                         v1.0.0     pending-push    never
build                          v1.0.1     synced          2026-03-27 14:00
deploy                         v1.0.3     synced          2026-03-27 14:22
old-script                     v1.0.2     conflict        2026-03-25 09:00
utils                          v1.0.0     local-only      never

1 conflict(s) detected. Resolve with:
  sv sync resolve old-script --take-local  (or --take-remote)
```

**Sync state reference:**

| Status | Meaning |
|--------|---------|
| `synced` | Local and remote are identical |
| `local-only` | Script has never been pushed to the cloud |
| `pending-push` | Local changes exist that have not been pushed |
| `pending-pull` | Remote has newer changes not yet pulled |
| `conflict` | Both local and remote changed since last sync |
| `remote-only` | Script exists on the remote but not locally |

---

### `sv sync resolve <name>`

Resolves a sync conflict by choosing which version to keep. `--take-local` overwrites the remote with your local version. `--take-remote` overwrites your local copy with the remote version.

```bash
sv sync resolve deploy --take-local    # keep your local version, push it
sv sync resolve deploy --take-remote   # discard local changes, pull remote
```

**Flags:**

| Flag | Description |
|------|-------------|
| `--take-local` | Keep the local version and push it to the cloud |
| `--take-remote` | Keep the remote version and overwrite local |

These two flags are mutually exclusive — you must specify exactly one.

**Example:**
```
$ sv sync resolve deploy --take-local

Conflict resolved for: deploy
```

**Error — script is not in conflict:**
```
Error: Script 'deploy' is not in a conflict state (current status: synced)
```

**Error — no flag provided:**
```
Error: Specify --take-local or --take-remote to resolve the conflict.
```

---

## Export

---

### `sv export`

Exports all scripts from your vault to a file or stdout. Supports Markdown and JSON formats.

```bash
sv export                                           # Markdown to stdout
sv export --format markdown --output scripts.md
sv export --format json --output scripts.json
```

**Flags:**

| Flag | Default | Description |
|------|---------|-------------|
| `--format <FORMAT>` | `markdown` | Output format: `markdown` (or `md`) or `json` |
| `--output <PATH>` | — | Write to a file instead of stdout |

**Example — export to file:**
```
$ sv export --format markdown --output scripts.md

✓ Exported 5 scripts to: scripts.md
```

**Example — Markdown output structure (stdout):**
```markdown
# ScriptVault Export

Exported: 2026-03-27 14:22:01 UTC

Total scripts: 2

## Contents

- [deploy](#deploy)
- [backup](#backup)

---

## deploy

| Property | Value |
|----------|-------|
| Language | bash |
| Version  | v1.0.3 |
| Author   | yourname |
| Tags     | deploy, production |
| Uses     | 6 |
| Success rate | 83.3% |

### Context

- Directory: `/home/user/myproject`
- Git repo: `github.com/user/myproject`
- Branch: `main`

### Script

\`\`\`bash
#!/usr/bin/env bash
set -e
echo "Deploying..."
\`\`\`

Run: `sv run deploy`

---
```

**Example — JSON output structure:**
```json
{
  "exported_at": "2026-03-27T14:22:01Z",
  "export_version": "1.0",
  "total_scripts": 2,
  "scripts": [
    {
      "id": "3f8a1c2d-...",
      "name": "deploy",
      "content": "#!/usr/bin/env bash\n...",
      "version": "v1.0.3",
      "language": "Bash",
      "tags": ["deploy", "production"],
      ...
    }
  ]
}
```

**Example — empty vault:**
```
$ sv export

No scripts to export.
```

---

## Storage

---

### `sv storage status`

Shows the active storage backend type, vault path, and health.

```bash
sv storage status
```

**Example:**
```
$ sv storage status

Storage Configuration

  Backend: Local Filesystem
  Path: /home/user/.scriptvault/vault

  Health... healthy
```

**Example — unhealthy (vault directory missing):**
```
$ sv storage status

Storage Configuration

  Backend: Local Filesystem
  Path: /home/user/.scriptvault/vault

  Health... unhealthy
```

---

### `sv storage setup`

Interactively reconfigures where your vault is stored. Useful if you want to move your vault to a different directory or a synced location (e.g. a Dropbox or NFS mount).

```bash
sv storage setup
```

**Example:**
```
$ sv storage setup

Storage Setup

Vault path [/home/user/.scriptvault/vault]: /data/my-vault

✓ Storage configured: /data/my-vault
```

---

### `sv storage test`

Tests the storage backend by running a connection check, a health check, and a read access check.

```bash
sv storage test
```

**Example — all passing:**
```
$ sv storage test

Storage Test

  Connecting...    ✓
  Health check...  ✓
  Read access...   ✓

✓ Storage is working correctly
```

**Example — health check failure:**
```
$ sv storage test

Storage Test

  Connecting...    ✓
  Health check...  failed
```

---

### `sv storage info`

Shows the total number of scripts in the vault and the combined size of all script content.

```bash
sv storage info
```

**Example:**
```
$ sv storage info

Storage Information

  Backend: local
  Scripts: 12
  Size:    0.18 MB
```

---

## Diagnostics

---

### `sv doctor`

Runs a comprehensive health check of your entire ScriptVault setup: config file, vault directory, required system tools, editor configuration, SSH agent, and cloud API connectivity and authentication.

```bash
sv doctor
```

**Example — fully healthy:**
```
$ sv doctor

ScriptVault Health Check

  Config file...       ok
  Vault directory...   ok
  bash...              ok
  sh...                ok
  git...               ok
  editor ($EDITOR)...  ok (nvim)

  SSH:
    ssh binary...        ok
    ssh-agent socket...  ok
    ssh-agent keys...    ok (2 loaded)

  Cloud sync:
    API endpoint...  reachable
    Auth token...    valid

Health check complete.
```

**Example — partial issues:**
```
$ sv doctor

ScriptVault Health Check

  Config file...       ok
  Vault directory...   ok
  bash...              ok
  sh...                ok
  git...               not found
  editor ($EDITOR)...  not set, checking fallback... vi available

  SSH:
    ssh binary...        ok
    ssh-agent socket...  not running (SSH_AUTH_SOCK not set)

  Cloud sync:
    API endpoint...  unreachable (connection refused)
    Auth token...    not configured (local mode)

Health check complete.
```

**Doctor checks reference:**

| Check | What it verifies |
|-------|-----------------|
| Config file | `~/.scriptvault/config.json` exists |
| Vault directory | `~/.scriptvault/vault/` exists |
| `bash` / `sh` / `git` | Binaries are available in `PATH` |
| Editor | `$EDITOR` or `$VISUAL` is set and resolvable |
| SSH binary | `ssh` is in `PATH` |
| SSH agent socket | `SSH_AUTH_SOCK` is set and the socket exists |
| SSH agent keys | `ssh-add -l` reports loaded keys |
| API endpoint | `/health` route returns 200 |
| Auth token | `GET /auth/me` returns a valid user |

---

### `sv status`

A quick one-liner overview of your current mode and vault path. Lighter than `sv doctor` — no network calls, no tool checks.

```bash
sv status
```

**Example:**
```
$ sv status

ScriptVault Status

  Mode:  Local
  Vault: /home/user/.scriptvault/vault
```

---

## Global Notes

**Dangerous pattern detection.** Before any script is run, ScriptVault scans its content for patterns known to cause irreversible system damage. If one is found, a warning is printed. In non-CI mode, you will be prompted to confirm before execution proceeds.

Detected patterns include: `rm -rf /`, `rm -rf /*`, `mkfs`, `dd if=`, `> /dev/sda`, `:(){ :|:& };:`, `chmod -R 777 /`, and variants.

**Version bumping.** Versions follow `vMAJOR.MINOR.PATCH`. Every time script content changes (via `sv save`, `sv update`, `sv edit`, `sv adapt`, or `sv checkout`), the patch number is incremented automatically. You do not manage version numbers manually.

**Execution environment.** By default, scripts run with a minimal safe set of environment variables: `PATH`, `TERM`, `LANG`, `LC_ALL`, `LC_CTYPE`, `HOME`, `USER`, `LOGNAME`, `SHELL`, `TZ`, `TMPDIR`, `TEMP`, `TMP`. Use `--sandbox` to further isolate to a private temp directory with only `PATH`, `HOME`, `TMPDIR`, `TERM`, `LANG`, and `ISOLATED=1`.

**History rotation.** The execution log at `~/.scriptvault/history.jsonl` is capped at 1000 entries and trimmed automatically.

**Version snapshots.** Up to 50 version snapshots are stored per script. When the limit is reached, the oldest snapshot is pruned automatically.
