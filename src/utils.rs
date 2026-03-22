use anyhow::Result;
use colored::*;

pub fn run_doctor() -> Result<()> {
    println!("{}", "ScriptVault Health Check".cyan().bold());
    println!();

    print!("  Config file... ");
    if crate::config::Config::config_path()?.exists() {
        println!("{}", "✓".green());
    } else {
        println!("{}", "not found".red());
    }

    print!("  Vault directory... ");
    if crate::config::Config::vault_dir()?.exists() {
        println!("{}", "✓".green());
    } else {
        println!("{}", "not found".red());
    }

    for cmd in &["bash", "sh", "git"] {
        print!("  {}... ", cmd);
        if which::which(cmd).is_ok() {
            println!("{}", "✓".green());
        } else {
            println!("{}", "not found".yellow());
        }
    }

    println!();
    println!("{}", "Health check complete".green().bold());
    Ok(())
}

pub fn check_status() -> Result<()> {
    println!("{}", "ScriptVault Status".cyan().bold());
    println!();
    println!("  {}: {}", "Mode".bold(), "Local".green());
    println!(
        "  {}: {}",
        "Vault".bold(),
        crate::config::Config::vault_dir()?.display()
    );
    Ok(())
}
