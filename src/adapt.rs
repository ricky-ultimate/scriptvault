use crate::cli::AdaptArgs;
use crate::config::Config;
use crate::context;
use crate::script::SyncStatus;
use crate::vault::load_scripts_local;
use anyhow::{Result, anyhow};
use colored::*;
use dialoguer::Confirm;
use sha2::{Digest, Sha256};
use std::fs;

#[derive(Debug)]
pub struct Substitution {
    from: String,
    to: String,
    pub(crate) kind: &'static str,
}

pub(crate) fn build_substitutions(
    script_dir: Option<&str>,
    current_dir: Option<&str>,
) -> Vec<Substitution> {
    let mut subs: Vec<Substitution> = Vec::new();

    match (script_dir, current_dir) {
        (Some(s), Some(c)) if s != c => {
            subs.push(Substitution {
                from: s.to_string(),
                to: c.to_string(),
                kind: "directory",
            });

            if let (Some(s_home), Some(c_home)) = (extract_home(s), extract_home(c)) {
                if s_home != c_home {
                    subs.push(Substitution {
                        from: s_home,
                        to: c_home,
                        kind: "home directory",
                    });
                }
            }
        }
        _ => {}
    }

    subs
}

fn extract_home(path: &str) -> Option<String> {
    if path.starts_with("/home/") || path.starts_with("/Users/") {
        let parts: Vec<&str> = path.splitn(4, '/').collect();
        if parts.len() >= 3 {
            return Some(format!("/{}/{}", parts[1], parts[2]));
        }
    }
    None
}

pub(crate) fn apply_substitutions(content: &str, subs: &[Substitution]) -> String {
    let mut result = content.to_string();
    for sub in subs {
        result = result.replace(&sub.from, &sub.to);
    }
    result
}

fn show_diff(original: &str, adapted: &str) {
    let orig_lines: Vec<&str> = original.lines().collect();
    let new_lines: Vec<&str> = adapted.lines().collect();
    let max = orig_lines.len().max(new_lines.len());
    let mut has_changes = false;

    for i in 0..max {
        match (orig_lines.get(i).copied(), new_lines.get(i).copied()) {
            (Some(a), Some(b)) if a == b => {
                println!("  {}", a);
            }
            (Some(a), Some(b)) => {
                println!("{} {}", "-".red(), a.red());
                println!("{} {}", "+".green(), b.green());
                has_changes = true;
            }
            (Some(a), None) => {
                println!("{} {}", "-".red(), a.red());
                has_changes = true;
            }
            (None, Some(b)) => {
                println!("{} {}", "+".green(), b.green());
                has_changes = true;
            }
            (None, None) => {}
        }
    }

    if !has_changes {
        println!("  (no line-level changes)");
    }
}

pub fn adapt_script(args: AdaptArgs) -> Result<()> {
    let config = Config::load()?;
    let storage = config.get_storage_backend()?;

    let script = load_scripts_local()?
        .into_iter()
        .find(|s| s.name == args.script)
        .ok_or_else(|| anyhow!("Script not found: {}", args.script))?;

    let current_ctx = context::detect_context()?;

    let subs = build_substitutions(
        script.context.directory.as_deref(),
        current_ctx.directory.as_deref(),
    );

    if subs.is_empty() {
        println!(
            "{} No adaptations needed for {}",
            "i".cyan(),
            args.script.yellow()
        );
        println!();
        println!(
            "  Script context:  {}",
            script
                .context
                .directory
                .as_deref()
                .unwrap_or("unknown")
                .dimmed()
        );
        println!(
            "  Current context: {}",
            current_ctx
                .directory
                .as_deref()
                .unwrap_or("unknown")
                .dimmed()
        );
        return Ok(());
    }

    let adapted_content = apply_substitutions(&script.content, &subs);

    if adapted_content == script.content {
        println!(
            "{} Script content unchanged after applying context substitutions.",
            "i".cyan()
        );
        println!("  The script may not reference any context-specific paths.");
        return Ok(());
    }

    println!("{}", "Adapt Preview".cyan().bold());
    println!();
    println!("  Script:  {}", args.script.yellow());
    println!(
        "  Context: {} -> {}",
        script
            .context
            .directory
            .as_deref()
            .unwrap_or("unknown")
            .dimmed(),
        current_ctx
            .directory
            .as_deref()
            .unwrap_or("unknown")
            .green()
    );
    println!();
    println!("  {}:", "Substitutions".bold());
    for sub in &subs {
        println!(
            "    [{}] {} -> {}",
            sub.kind.cyan(),
            sub.from.red(),
            sub.to.green()
        );
    }
    println!();
    println!("  {}:", "Diff".bold());
    show_diff(&script.content, &adapted_content);
    println!();

    if args.dry_run {
        println!("{}", "Dry run complete. No changes applied.".yellow());
        return Ok(());
    }

    if let Some(ref output_path) = args.output {
        if !args.yes {
            let proceed = Confirm::new()
                .with_prompt(format!("Write adapted script to {}?", output_path))
                .default(true)
                .interact()?;
            if !proceed {
                println!("Cancelled.");
                return Ok(());
            }
        }
        fs::write(output_path, &adapted_content)?;
        println!(
            "{} Written to: {}",
            "✓".green().bold(),
            output_path.yellow()
        );
        return Ok(());
    }

    if !args.yes {
        let proceed = Confirm::new()
            .with_prompt("Apply adaptations and save to vault?")
            .default(true)
            .interact()?;
        if !proceed {
            println!("Cancelled.");
            return Ok(());
        }
    }

    let mut updated = script.clone();
    updated.content = adapted_content;
    updated.context.directory = current_ctx.directory;
    updated.context.git_repo = current_ctx.git_repo;
    updated.context.git_branch = current_ctx.git_branch;

    let mut hasher = Sha256::new();
    hasher.update(updated.content.as_bytes());
    updated.metadata.hash = hex::encode(hasher.finalize());
    updated.metadata.size_bytes = updated.content.len();
    updated.metadata.line_count = updated.content.lines().count();
    updated.updated_at = chrono::Utc::now();

    let old_version = updated.version.clone();
    let v = old_version.trim_start_matches('v');
    let parts: Vec<u64> = v.split('.').filter_map(|p| p.parse().ok()).collect();
    updated.version = if parts.len() == 3 {
        format!("v{}.{}.{}", parts[0], parts[1], parts[2] + 1)
    } else {
        format!("{}.1", old_version)
    };

    match updated.sync_state.status {
        SyncStatus::Synced => updated.sync_state.status = SyncStatus::PendingPush,
        SyncStatus::PendingPull | SyncStatus::RemoteOnly => {
            updated.sync_state.status = SyncStatus::Conflict
        }
        SyncStatus::PendingPush | SyncStatus::LocalOnly | SyncStatus::Conflict => {}
    }

    storage.update_script(&updated)?;

    let store = crate::versions::VersionStore::new(&Config::vault_dir()?);
    store.save_version(&updated)?;

    println!(
        "{} Adapted: {} {} -> {}",
        "✓".green().bold(),
        args.script.yellow(),
        old_version.dimmed(),
        updated.version.green()
    );

    Ok(())
}
