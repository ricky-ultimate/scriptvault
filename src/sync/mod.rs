pub mod manager;
pub mod remote;

pub use manager::{ConflictResolution, SyncManager, SyncReport};
pub use remote::RemoteBackend;

use crate::config::Config;
use crate::sync::remote::HttpRemoteBackend;
use anyhow::{Result, anyhow};
use colored::*;

fn build_manager() -> Result<SyncManager> {
    let config = Config::load()?;
    if !config.is_authenticated() {
        return Err(anyhow!(
            "Cloud sync requires authentication. Run 'sv auth login --token <API_KEY>'"
        ));
    }
    let token = config
        .auth_token
        .clone()
        .ok_or_else(|| anyhow!("No auth token found"))?;
    let local = config.get_storage_backend()?;
    let remote = HttpRemoteBackend::new(config.api_endpoint.clone(), token);
    Ok(SyncManager::new(local, Box::new(remote)))
}

pub fn push_all(dry_run: bool) -> Result<()> {
    if dry_run {
        return show_dry_run_push();
    }
    let manager = build_manager()?;
    let report = manager.push_pending()?;
    print_report(&report);
    Ok(())
}

pub fn pull_all(dry_run: bool) -> Result<()> {
    if dry_run {
        return show_dry_run_pull();
    }
    let manager = build_manager()?;
    let report = manager.full_sync()?;
    print_report(&report);
    Ok(())
}

fn show_dry_run_push() -> Result<()> {
    let config = Config::load()?;
    if !config.is_authenticated() {
        return Err(anyhow!(
            "Cloud sync requires authentication. Run 'sv auth login --token <API_KEY>'"
        ));
    }
    let local = config.get_storage_backend()?;
    let pending = local.list_pending_push()?;

    if pending.is_empty() {
        println!("{}", "Nothing to push.".green());
        return Ok(());
    }

    println!(
        "{}",
        "Dry run — scripts that would be pushed:".yellow().bold()
    );
    println!();
    for script in &pending {
        println!("  {} {}", script.name.yellow(), script.version.dimmed());
    }
    println!();
    println!("Run 'sv sync push' without --dry-run to apply.");
    Ok(())
}

fn show_dry_run_pull() -> Result<()> {
    let manager = build_manager()?;
    let config = Config::load()?;
    let local = config.get_storage_backend()?;

    let local_scripts = local.list_scripts()?;
    let remote_metas = manager.remote_list()?;

    let local_names: std::collections::HashSet<String> =
        local_scripts.iter().map(|s| s.name.clone()).collect();

    let remote_only: Vec<_> = remote_metas
        .iter()
        .filter(|m| !local_names.contains(&m.name))
        .collect();

    if remote_only.is_empty() {
        println!("{}", "Nothing to pull.".green());
        return Ok(());
    }

    println!(
        "{}",
        "Dry run — scripts that would be pulled:".yellow().bold()
    );
    println!();
    for meta in &remote_only {
        println!("  {} {}", meta.name.yellow(), meta.version.dimmed());
    }
    println!();
    println!("Run 'sv sync pull' without --dry-run to apply.");
    Ok(())
}

pub fn resolve_conflict(script_name: &str, resolution: ConflictResolution) -> Result<()> {
    let manager = build_manager()?;
    manager.resolve_conflict(script_name, resolution)?;
    println!("Conflict resolved for: {}", script_name.yellow());
    Ok(())
}

pub fn show_status() -> Result<()> {
    let config = Config::load()?;
    let local = config.get_storage_backend()?;
    let scripts = local.list_scripts()?;

    if scripts.is_empty() {
        println!("No scripts in vault.");
        return Ok(());
    }

    println!("{}", "Sync Status".cyan().bold());
    println!();
    println!(
        "{:<30} {:<10} {:<15} {:<20}",
        "NAME".bold(),
        "VERSION".bold(),
        "STATUS".bold(),
        "LAST SYNCED".bold()
    );
    println!("{}", "─".repeat(78).dimmed());

    for script in &scripts {
        let status_display = match script.sync_state.status {
            crate::script::SyncStatus::Synced => "synced".green().to_string(),
            crate::script::SyncStatus::LocalOnly => "local-only".yellow().to_string(),
            crate::script::SyncStatus::RemoteOnly => "remote-only".cyan().to_string(),
            crate::script::SyncStatus::PendingPush => "pending-push".yellow().to_string(),
            crate::script::SyncStatus::PendingPull => "pending-pull".cyan().to_string(),
            crate::script::SyncStatus::Conflict => "conflict".red().bold().to_string(),
        };

        let last_synced = match script.sync_state.last_synced_at {
            Some(t) => t.format("%Y-%m-%d %H:%M").to_string(),
            None => "never".dimmed().to_string(),
        };

        println!(
            "{:<30} {:<10} {:<15} {:<20}",
            script.name.yellow(),
            script.version.dimmed(),
            status_display,
            last_synced
        );
    }

    let conflicts: Vec<_> = scripts
        .iter()
        .filter(|s| s.sync_state.status == crate::script::SyncStatus::Conflict)
        .collect();

    if !conflicts.is_empty() {
        println!();
        println!(
            "{} conflict(s) detected. Resolve with:",
            conflicts.len().to_string().red().bold()
        );
        for s in &conflicts {
            println!(
                "  sv sync resolve {} --take-local  (or --take-remote)",
                s.name
            );
        }
    }

    Ok(())
}

fn print_report(report: &SyncReport) {
    if !report.pushed.is_empty() {
        println!("Pushed ({}):", report.pushed.len());
        for name in &report.pushed {
            println!("  {}", name.yellow());
        }
    }

    if !report.pulled.is_empty() {
        println!("Pulled ({}):", report.pulled.len());
        for name in &report.pulled {
            println!("  {}", name.yellow());
        }
    }

    if !report.conflicts.is_empty() {
        println!("Conflicts ({}):", report.conflicts.len());
        for name in &report.conflicts {
            println!(
                "  {} - resolve with 'sv sync resolve {} --take-local|--take-remote'",
                name.red(),
                name
            );
        }
    }

    if !report.errors.is_empty() {
        println!("Errors ({}):", report.errors.len());
        for (name, err) in &report.errors {
            println!("  {}: {}", name.red(), err);
        }
    }

    if report.pushed.is_empty()
        && report.pulled.is_empty()
        && report.conflicts.is_empty()
        && report.errors.is_empty()
    {
        println!("{}", "Everything is up to date.".green());
    }
}
