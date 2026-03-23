use crate::cli::LoginArgs;
use crate::config::Config;
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

    config.set_auth("local".to_string(), "local_user".to_string(), username.clone());
    config.save()?;

    println!("{} Username set to: {}", "✓".green().bold(), username.yellow());

    Ok(())
}

pub fn logout() -> Result<()> {
    let mut config = Config::load()?;
    config.clear_auth();
    config.save()?;
    println!("{} Username cleared", "✓".green().bold());
    Ok(())
}

pub fn status() -> Result<()> {
    let config = Config::load()?;

    println!("{}", "ScriptVault Local Mode".cyan().bold());
    println!();

    if let Some(username) = &config.username {
        println!("  {}: {}", "Username".bold(), username.yellow());
    } else {
        println!(
            "  {}: {}",
            "Username".bold(),
            "not set".dimmed()
        );
        println!();
        println!("  Run 'sv auth login' to set a username");
    }

    Ok(())
}
