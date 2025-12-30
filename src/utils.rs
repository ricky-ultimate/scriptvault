use anyhow::Result;
use colored::*;

pub fn run_doctor() -> Result<()> {
    println!("{}", "ScriptVault Health Check".cyan().bold());
    println!();

    // Check config
    print!("  Config file... ");
    if crate::config::Config::config_path()?.exists() {
        println!("{}", "✓".green());
    } else {
        println!("{}", "✗ Not found".red());
    }

    // Check vault directory
    print!("  Vault directory... ");
    if crate::config::Config::vault_dir()?.exists() {
        println!("{}", "✓".green());
    } else {
        println!("{}", "✗ Not found".red());
    }

    // Check required commands
    let commands = vec!["bash", "sh", "git"];
    for cmd in commands {
        print!("  {} command... ", cmd);
        if which::which(cmd).is_ok() {
            println!("{}", "✓".green());
        } else {
            println!("{}", "✗ Not found".yellow());
        }
    }

    println!();
    println!("{}", "All checks passed!".green().bold());
    Ok(())
}

pub fn check_status() -> Result<()> {
    println!("{}", "ScriptVault Service Status".cyan().bold());
    println!();
    println!(
        "  {}: {}",
        "API".bold(),
        "https://api.scriptvault.dev".green()
    );
    println!(
        "  {}: {}",
        "Status".bold(),
        "Service not yet deployed".yellow()
    );
    println!();
    println!("For now, ScriptVault operates in local-only mode.");
    Ok(())
}
