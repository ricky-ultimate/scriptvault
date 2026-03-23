use crate::cli::LoginArgs;
use crate::config::{AuthMode, Config};
use crate::constants::default_author;
use anyhow::Result;
use colored::*;
use dialoguer::Input;

pub fn login(args: LoginArgs) -> Result<()> {
    let mut config = Config::load()?;

    let username = if let Some(name) = args.token {
        if name == "local" {
            default_author()
        } else {
            name
        }
    } else {
        Input::new()
            .with_prompt("Username")
            .default(default_author())
            .interact_text()?
    };

    config.set_local_user(username.clone());
    config.save()?;

    println!("Username set to: {}", username.yellow());

    Ok(())
}

pub fn logout() -> Result<()> {
    let mut config = Config::load()?;
    config.clear_auth();
    config.save()?;
    println!("Username cleared");
    Ok(())
}

pub fn status() -> Result<()> {
    let config = Config::load()?;

    println!("{}", "ScriptVault Auth Status".cyan().bold());
    println!();

    match &config.auth_mode {
        AuthMode::Local => {
            println!("  {}: {}", "Mode".bold(), "Local".yellow());
            if let Some(username) = &config.username {
                println!("  {}: {}", "Username".bold(), username.yellow());
            } else {
                println!("  {}: {}", "Username".bold(), "not set".dimmed());
                println!();
                println!("  Run 'sv auth login' to set a username");
            }
        }
        AuthMode::ApiKey => {
            println!("  {}: {}", "Mode".bold(), "API Key".green());
            if let Some(username) = &config.username {
                println!("  {}: {}", "Username".bold(), username.yellow());
            }
        }
        AuthMode::OAuth => {
            println!("  {}: {}", "Mode".bold(), "OAuth".green());
            if let Some(username) = &config.username {
                println!("  {}: {}", "Username".bold(), username.yellow());
            }
        }
    }

    Ok(())
}
