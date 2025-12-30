use crate::cli::LoginArgs;
use crate::config::Config;
use anyhow::Result;
use colored::*;

pub fn login(args: LoginArgs) -> Result<()> {
    let mut config = Config::load()?;

    if let Some(token) = args.token {
        // API token auth
        config.set_auth(token, "local_user".to_string(), "LocalUser".to_string());
        config.save()?;

        println!("{} Authenticated with API token", "✓".green().bold());
    } else {
        // OAuth flow (mock for now)
        println!("{}", "Opening browser for authentication...".cyan());
        println!(
            "{}",
            "OAuth flow not yet implemented. Use --token instead.".yellow()
        );
        println!();
        println!("Example: sv auth login --token YOUR_API_KEY");
    }

    Ok(())
}

pub fn logout() -> Result<()> {
    let mut config = Config::load()?;
    config.clear_auth();
    config.save()?;

    println!("{} Logged out successfully", "✓".green().bold());
    Ok(())
}

pub fn status() -> Result<()> {
    let config = Config::load()?;

    println!("{}", "Authentication Status".cyan().bold());
    println!();

    if config.is_authenticated() {
        println!("  {}: {}", "Status".bold(), "Authenticated".green());
        if let Some(username) = &config.username {
            println!("  {}: {}", "User".bold(), username.yellow());
        }
        if let Some(user_id) = &config.user_id {
            println!("  {}: {}", "User ID".bold(), user_id.dimmed());
        }
    } else {
        println!("  {}: {}", "Status".bold(), "Not authenticated".red());
        println!();
        println!("  Run 'sv auth login' to authenticate");
    }

    Ok(())
}
