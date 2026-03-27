use crate::cli::{LoginArgs, RegisterArgs};
use crate::config::{AuthMode, Config};
use crate::constants::default_author;
use anyhow::{Result, anyhow};
use colored::*;
use dialoguer::Input;
use serde::Deserialize;

fn ureq_err(e: ureq::Error) -> anyhow::Error {
    match e {
        ureq::Error::Status(409, _) => anyhow!("That username is already taken"),
        ureq::Error::Status(401, _) => anyhow!("Invalid API key"),
        ureq::Error::Status(400, resp) => {
            let body = resp.into_string().unwrap_or_default();
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                if let Some(msg) = v.get("error").and_then(|m| m.as_str()) {
                    return anyhow!("{}", msg);
                }
            }
            anyhow!("Bad request")
        }
        ureq::Error::Status(code, _) => anyhow!("Server returned error {}", code),
        ureq::Error::Transport(t) => anyhow!("Network error: {}", t),
    }
}

pub fn register(args: RegisterArgs) -> Result<()> {
    let config = Config::load()?;
    let endpoint = config.api_endpoint.clone();

    let username: String = if let Some(name) = args.username {
        name
    } else {
        Input::new()
            .with_prompt("Choose a username")
            .default(default_author())
            .interact_text()?
    };

    println!("Registering...");

    #[derive(Deserialize)]
    struct RegisterResponse {
        api_key: String,
        user_id: String,
        username: String,
    }

    let response = ureq::post(&format!("{}/auth/register", endpoint))
        .set("Content-Type", "application/json")
        .send_json(serde_json::json!({ "username": username }))
        .map_err(ureq_err)?;

    let data: RegisterResponse = response
        .into_json()
        .map_err(|e| anyhow!("Failed to parse server response: {}", e))?;

    let mut config = Config::load()?;
    config.set_api_key(data.api_key.clone(), data.user_id, data.username.clone());
    config.save()?;

    println!("Registered as: {}", data.username.yellow());
    println!();
    println!("API key: {}", data.api_key.yellow().bold());
    println!();
    println!("Save this key. To authenticate on another machine:");
    println!("  sv auth login --token {}", data.api_key);

    Ok(())
}

pub fn login(args: LoginArgs) -> Result<()> {
    let mut config = Config::load()?;

    let input = if let Some(value) = args.token {
        value
    } else {
        Input::new()
            .with_prompt("Username or API key (sv_...)")
            .default(default_author())
            .interact_text()?
    };

    if input.starts_with("sv_") {
        let endpoint = config.api_endpoint.clone();

        #[derive(Deserialize)]
        struct MeResponse {
            user_id: String,
            username: String,
        }

        let response = ureq::get(&format!("{}/auth/me", endpoint))
            .set("Authorization", &format!("Bearer {}", input))
            .call()
            .map_err(ureq_err)?;

        let data: MeResponse = response
            .into_json()
            .map_err(|e| anyhow!("Failed to parse server response: {}", e))?;

        config.set_api_key(input, data.user_id, data.username.clone());
        config.save()?;

        println!("Logged in as: {}", data.username.yellow());
    } else {
        let username = if input == "local" {
            default_author()
        } else {
            input
        };
        config.set_local_user(username.clone());
        config.save()?;
        println!("Username set to: {}", username.yellow());
    }

    Ok(())
}

pub fn logout() -> Result<()> {
    let mut config = Config::load()?;
    config.clear_auth();
    config.save()?;
    println!("Logged out");
    Ok(())
}

pub fn status() -> Result<()> {
    let config = Config::load()?;

    println!("{}", "ScriptVault Auth Status".cyan().bold());
    println!();

    match &config.auth_mode {
        AuthMode::Local => {
            println!("  {}: {}", "Mode".bold(), "Local".yellow());
            match &config.username {
                Some(username) => println!("  {}: {}", "Username".bold(), username.yellow()),
                None => {
                    println!("  {}: {}", "Username".bold(), "not set".dimmed());
                    println!();
                    println!("  Run 'sv auth register' to create an account");
                    println!("  Or 'sv auth login' to set a local username");
                }
            }
        }
        AuthMode::ApiKey => {
            println!("  {}: {}", "Mode".bold(), "Cloud".green());
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

pub fn show_token() -> Result<()> {
    let config = Config::load()?;

    match &config.auth_token {
        Some(token) => {
            println!("{}", "API Key".cyan().bold());
            println!();
            println!("  {}", token.yellow());
            println!();
            println!("To log in on another machine:");
            println!("  sv auth login --token {}", token);
            println!();
            println!("To generate a new key (this one will be revoked):");
            println!("  sv auth rotate-token");
        }
        None => {
            println!("{}", "No API key configured.".yellow());
            println!();
            println!("Run 'sv auth register' to create an account.");
            println!("Run 'sv auth login --token <KEY>' to authenticate with an existing key.");
        }
    }

    Ok(())
}

pub fn rotate_token() -> Result<()> {
    let config = Config::load()?;

    let token = config
        .auth_token
        .as_deref()
        .ok_or_else(|| anyhow!("No API key configured. Run 'sv auth register' first."))?;

    let endpoint = config.api_endpoint.clone();

    println!("{}", "This will revoke your current API key.".yellow());
    println!();

    let confirmed = dialoguer::Confirm::new()
        .with_prompt("Proceed?")
        .default(false)
        .interact()?;

    if !confirmed {
        println!("Cancelled.");
        return Ok(());
    }

    #[derive(Deserialize)]
    struct RotateResponse {
        api_key: String,
    }

    let response = ureq::post(&format!("{}/auth/rotate", endpoint))
        .set("Authorization", &format!("Bearer {}", token))
        .set("Content-Type", "application/json")
        .send_json(serde_json::json!({}))
        .map_err(ureq_err)?;

    let data: RotateResponse = response
        .into_json()
        .map_err(|e| anyhow!("Failed to parse server response: {}", e))?;

    let mut updated_config = Config::load()?;
    let user_id = updated_config
        .user_id
        .clone()
        .ok_or_else(|| anyhow!("Missing user_id in config"))?;
    let username = updated_config
        .username
        .clone()
        .ok_or_else(|| anyhow!("Missing username in config"))?;

    updated_config.set_api_key(data.api_key.clone(), user_id, username);
    updated_config.save()?;

    println!();
    println!("{} New API key:", "✓".green().bold());
    println!();
    println!("  {}", data.api_key.yellow().bold());
    println!();
    println!("Save this key. The old key is now invalid.");
    println!("To log in on another machine:");
    println!("  sv auth login --token {}", data.api_key);

    Ok(())
}
