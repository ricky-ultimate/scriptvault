use crate::cli::{StorageAction, StorageSetupArgs};
use crate::config::Config;
use crate::storage::{StorageBackend, StorageConfig};
use anyhow::{Result, anyhow};
use colored::*;
use dialoguer::{Input, Select};
use std::path::PathBuf;

pub fn handle_storage_command(action: StorageAction) -> Result<()> {
    match action {
        StorageAction::Status => show_storage_status(),
        StorageAction::Setup(args) => setup_storage_backend(args),
        StorageAction::Test => test_storage_connection(),
        StorageAction::Info => show_storage_info(),
    }
}

fn show_storage_status() -> Result<()> {
    let config = Config::load()?;

    println!("{}", "Storage Configuration".cyan().bold());
    println!();

    match &config.storage {
        StorageConfig::Local { path } => {
            println!("  {}: {}", "Backend".bold(), "Local Filesystem".green());
            println!("  {}: {}", "Path".bold(), path.display());
        }
        StorageConfig::Backblaze {
            bucket_name,
            endpoint,
            ..
        } => {
            println!("  {}: {}", "Backend".bold(), "Backblaze B2".green());
            println!("  {}: {}", "Bucket".bold(), bucket_name);
            if let Some(ep) = endpoint {
                println!("  {}: {}", "Endpoint".bold(), ep);
            }
            println!("  {}: {}", "Status".bold(), "✓ Configured".green());
        }
        StorageConfig::S3 { bucket, region, .. } => {
            println!("  {}: {}", "Backend".bold(), "AWS S3".green());
            println!("  {}: {}", "Bucket".bold(), bucket);
            println!("  {}: {}", "Region".bold(), region);
        }
        StorageConfig::Gcs { bucket, .. } => {
            println!("  {}: {}", "Backend".bold(), "Google Cloud Storage".green());
            println!("  {}: {}", "Bucket".bold(), bucket);
        }
        StorageConfig::Azure { container, .. } => {
            println!("  {}: {}", "Backend".bold(), "Azure Blob Storage".green());
            println!("  {}: {}", "Container".bold(), container);
        }
    }

    // Show health status
    println!();
    print!("  Checking storage health... ");
    match config.get_storage_backend() {
        Ok(storage) => {
            if storage.health_check()? {
                println!("{}", "✓ Healthy".green());
            } else {
                println!("{}", "✗ Unhealthy".red());
            }
        }
        Err(e) => {
            println!("{}", format!("✗ Error: {}", e).red());
        }
    }

    Ok(())
}

fn setup_storage_backend(args: StorageSetupArgs) -> Result<()> {
    let backend_type = args.backend.to_lowercase();

    match backend_type.as_str() {
        "local" => setup_local_storage(),
        "backblaze" | "b2" => setup_backblaze_storage(),
        "s3" | "aws" => setup_s3_storage(),
        "gcs" | "google" => setup_gcs_storage(),
        "azure" => setup_azure_storage(),
        _ => {
            println!("{}", "Unknown storage backend.".red());
            println!();
            println!("Available backends:");
            println!("  • local      - Local filesystem (default)");
            println!("  • backblaze  - Backblaze B2 (recommended)");
            println!("  • s3         - AWS S3");
            println!("  • gcs        - Google Cloud Storage");
            println!("  • azure      - Azure Blob Storage");
            Err(anyhow!("Unknown storage backend: {}", backend_type))
        }
    }
}

fn setup_local_storage() -> Result<()> {
    println!("{}", "Setting up Local Storage".cyan().bold());
    println!();

    let default_path = Config::vault_dir()?;
    let path: String = Input::new()
        .with_prompt("Vault path")
        .default(default_path.to_string_lossy().to_string())
        .interact_text()?;

    let storage_config = StorageConfig::Local {
        path: PathBuf::from(path),
    };

    let mut config = Config::load()?;
    config.set_storage(storage_config)?;

    println!();
    println!("{} Local storage configured!", "✓".green().bold());
    println!("  Path: {}", config.vault_path.display());

    Ok(())
}

fn setup_backblaze_storage() -> Result<()> {
    println!("{}", "Setting up Backblaze B2 Storage".cyan().bold());
    println!();
    println!("📋 Prerequisites:");
    println!("  1. Create a Backblaze account: https://www.backblaze.com/b2/sign-up.html");
    println!("  2. Create a bucket: https://secure.backblaze.com/b2_buckets.htm");
    println!("  3. Generate Application Keys: https://secure.backblaze.com/app_keys.htm");
    println!();

    let key_id: String = Input::new()
        .with_prompt("Application Key ID")
        .interact_text()?;

    let app_key: String = Input::new()
        .with_prompt("Application Key")
        .interact_text()?;

    let bucket: String = Input::new()
        .with_prompt("Bucket Name")
        .default("scriptvault".to_string())
        .interact_text()?;

    let use_custom_endpoint = Select::new()
        .with_prompt("Use custom endpoint?")
        .items(&["No (use default)", "Yes (specify endpoint)"])
        .default(0)
        .interact()?;

    let endpoint = if use_custom_endpoint == 1 {
        Some(
            Input::new()
                .with_prompt("Custom endpoint URL")
                .interact_text()?,
        )
    } else {
        None
    };

    let storage_config = StorageConfig::Backblaze {
        key_id,
        application_key: app_key,
        bucket_name: bucket.clone(),
        endpoint,
    };

    let mut config = Config::load()?;
    config.set_storage(storage_config)?;

    println!();
    println!("{} Backblaze B2 storage configured!", "✓".green().bold());
    println!("  Bucket: {}", bucket);
    println!();
    println!("⚠️  Note: Backblaze B2 backend implementation coming in Phase 3!");
    println!("   For now, your config is saved but sync won't work yet.");

    Ok(())
}

fn setup_s3_storage() -> Result<()> {
    println!("{}", "Setting up AWS S3 Storage".cyan().bold());
    println!();

    let access_key: String = Input::new()
        .with_prompt("AWS Access Key ID")
        .interact_text()?;

    let secret_key: String = Input::new()
        .with_prompt("AWS Secret Access Key")
        .interact_text()?;

    let bucket: String = Input::new().with_prompt("S3 Bucket Name").interact_text()?;

    let region: String = Input::new()
        .with_prompt("AWS Region")
        .default("us-east-1".to_string())
        .interact_text()?;

    let storage_config = StorageConfig::S3 {
        access_key,
        secret_key,
        bucket: bucket.clone(),
        region: region.clone(),
    };

    let mut config = Config::load()?;
    config.set_storage(storage_config)?;

    println!();
    println!("{} AWS S3 storage configured!", "✓".green().bold());
    println!("  Bucket: {}", bucket);
    println!("  Region: {}", region);
    println!();
    println!("⚠️  Note: S3 backend implementation coming in Phase 7!");

    Ok(())
}

fn setup_gcs_storage() -> Result<()> {
    println!("{}", "Setting up Google Cloud Storage".cyan().bold());
    println!();

    let project_id: String = Input::new().with_prompt("GCP Project ID").interact_text()?;

    let bucket: String = Input::new()
        .with_prompt("GCS Bucket Name")
        .interact_text()?;

    let creds_path: String = Input::new()
        .with_prompt("Service Account JSON Path")
        .default("~/.gcp/credentials.json".to_string())
        .interact_text()?;

    let storage_config = StorageConfig::Gcs {
        project_id: project_id.clone(),
        bucket: bucket.clone(),
        credentials_path: PathBuf::from(shellexpand::tilde(&creds_path).to_string()),
    };

    let mut config = Config::load()?;
    config.set_storage(storage_config)?;

    println!();
    println!("{} Google Cloud Storage configured!", "✓".green().bold());
    println!("  Project: {}", project_id);
    println!("  Bucket: {}", bucket);
    println!();
    println!("⚠️  Note: GCS backend implementation coming in Phase 7!");

    Ok(())
}

fn setup_azure_storage() -> Result<()> {
    println!("{}", "Setting up Azure Blob Storage".cyan().bold());
    println!();

    let account_name: String = Input::new()
        .with_prompt("Storage Account Name")
        .interact_text()?;

    let account_key: String = Input::new()
        .with_prompt("Storage Account Key")
        .interact_text()?;

    let container: String = Input::new()
        .with_prompt("Container Name")
        .default("scriptvault".to_string())
        .interact_text()?;

    let storage_config = StorageConfig::Azure {
        account_name: account_name.clone(),
        account_key,
        container: container.clone(),
    };

    let mut config = Config::load()?;
    config.set_storage(storage_config)?;

    println!();
    println!("{} Azure Blob Storage configured!", "✓".green().bold());
    println!("  Account: {}", account_name);
    println!("  Container: {}", container);
    println!();
    println!("⚠️  Note: Azure backend implementation coming in Phase 7!");

    Ok(())
}

fn test_storage_connection() -> Result<()> {
    println!("{}", "Testing Storage Connection".cyan().bold());
    println!();

    let config = Config::load()?;

    print!("  Connecting to storage... ");
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

    print!("  Running health check... ");
    match storage.health_check() {
        Ok(true) => {
            println!("{}", "✓".green());
        }
        Ok(false) => {
            println!("{}", "✗ Failed".red());
            return Ok(());
        }
        Err(e) => {
            println!("{}", "✗".red());
            return Err(e);
        }
    }

    print!("  Checking read access... ");
    match storage.list_scripts() {
        Ok(_) => {
            println!("{}", "✓".green());
        }
        Err(e) => {
            println!("{}", format!("✗ {}", e).red());
            return Ok(());
        }
    }

    println!();
    println!("{} Storage is working correctly!", "✓".green().bold());

    Ok(())
}

fn show_storage_info() -> Result<()> {
    let config = Config::load()?;
    let storage = config.get_storage_backend()?;

    println!("{}", "Storage Information".cyan().bold());
    println!();

    let metadata = storage.get_metadata()?;

    println!("  {}: {}", "Backend Type".bold(), metadata.backend_type);
    println!("  {}: {}", "Total Scripts".bold(), metadata.total_scripts);

    let size_mb = metadata.total_size_bytes as f64 / 1_048_576.0;
    println!("  {}: {:.2} MB", "Total Size".bold(), size_mb);

    if let Some(last_sync) = metadata.last_sync {
        println!(
            "  {}: {}",
            "Last Sync".bold(),
            last_sync.format("%Y-%m-%d %H:%M:%S")
        );
    }

    Ok(())
}
