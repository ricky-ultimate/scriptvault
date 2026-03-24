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

    print!("  editor ($EDITOR)... ");
    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_default();

    if editor.is_empty() {
        print!("not set, checking fallback... ");
        if which::which("vi").is_ok() {
            println!("{}", "vi available".yellow());
        } else {
            println!("{}", "no editor found".red());
        }
    } else {
        let editor_bin = editor.split_whitespace().next().unwrap_or(&editor);
        if which::which(editor_bin).is_ok() {
            println!("{} ({})", "✓".green(), editor_bin);
        } else {
            println!("{} ({} not found in PATH)", "not found".red(), editor_bin);
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
