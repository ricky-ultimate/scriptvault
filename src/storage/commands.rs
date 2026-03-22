use crate::cli::StorageAction;
use crate::config::Config;
use crate::storage::StorageConfig;
use anyhow::Result;
use colored::*;
use dialoguer::Input;
use std::path::PathBuf;

pub fn handle_storage_command(action: StorageAction) -> Result<()> {
    match action {
        StorageAction::Status => show_status(),
        StorageAction::Setup => setup_storage(),
        StorageAction::Test => test_connection(),
        StorageAction::Info => show_info(),
    }
}

fn show_status() -> Result<()> {
    let config = Config::load()?;

    println!("{}", "Storage Configuration".cyan().bold());
    println!();
    println!("  {}: {}", "Backend".bold(), "Local Filesystem".green());
    println!("  {}: {}", "Path".bold(), config.storage.path.display());
    println!();

    print!("  Health... ");
    match config.get_storage_backend()?.health_check() {
        Ok(true) => println!("{}", "healthy".green()),
        Ok(false) => println!("{}", "unhealthy".red()),
        Err(e) => println!("{}", format!("error: {}", e).red()),
    }

    Ok(())
}

fn setup_storage() -> Result<()> {
    println!("{}", "Storage Setup".cyan().bold());
    println!();

    let default_path = Config::vault_dir()?;
    let path: String = Input::new()
        .with_prompt("Vault path")
        .default(default_path.to_string_lossy().to_string())
        .interact_text()?;

    let mut config = Config::load()?;
    config.set_storage(StorageConfig {
        path: PathBuf::from(&path),
    })?;

    println!();
    println!("{} Storage configured: {}", "✓".green().bold(), path);

    Ok(())
}

fn test_connection() -> Result<()> {
    println!("{}", "Storage Test".cyan().bold());
    println!();

    let config = Config::load()?;

    print!("  Connecting... ");
    let storage = match config.get_storage_backend() {
        Ok(s) => {
            println!("{}", "✓".green());
            s
        }
        Err(e) => {
            println!("{}", "✗".red());
            return Err(e);
        }
    };

    print!("  Health check... ");
    match storage.health_check() {
        Ok(true) => println!("{}", "✓".green()),
        Ok(false) => {
            println!("{}", "failed".red());
            return Ok(());
        }
        Err(e) => {
            println!("{}", "✗".red());
            return Err(e);
        }
    }

    print!("  Read access... ");
    match storage.list_scripts() {
        Ok(_) => println!("{}", "✓".green()),
        Err(e) => {
            println!("{}", format!("✗ {}", e).red());
            return Ok(());
        }
    }

    println!();
    println!("{} Storage is working correctly", "✓".green().bold());

    Ok(())
}

fn show_info() -> Result<()> {
    let config = Config::load()?;
    let storage = config.get_storage_backend()?;

    println!("{}", "Storage Information".cyan().bold());
    println!();

    let metadata = storage.get_metadata()?;
    let size_mb = metadata.total_size_bytes as f64 / 1_048_576.0;

    println!("  {}: {}", "Backend".bold(), metadata.backend_type);
    println!("  {}: {}", "Scripts".bold(), metadata.total_scripts);
    println!("  {}: {:.2} MB", "Size".bold(), size_mb);

    Ok(())
}
