use anyhow::Result;
use colored::*;

pub fn sync_vault() -> Result<()> {
    println!("{}", "Syncing vault...".cyan());
    println!("{}", "Sync feature not yet implemented.".yellow());
    println!();
    println!("For now, all scripts are stored locally at:");
    println!("  ~/.scriptvault/");
    Ok(())
}
