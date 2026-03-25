use anyhow::Result;
use colored::*;

pub fn run_doctor() -> Result<()> {
    println!("{}", "ScriptVault Health Check".cyan().bold());
    println!();

    print!("  Config file... ");
    let config_exists = crate::config::Config::config_path()?.exists();
    if config_exists {
        println!("{}", "ok".green());
    } else {
        println!("{}", "not found".red());
    }

    print!("  Vault directory... ");
    if crate::config::Config::vault_dir()?.exists() {
        println!("{}", "ok".green());
    } else {
        println!("{}", "not found".red());
    }

    for cmd in &["bash", "sh", "git"] {
        print!("  {}... ", cmd);
        if which::which(cmd).is_ok() {
            println!("{}", "ok".green());
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
            println!("{} ({})", "ok".green(), editor_bin);
        } else {
            println!("{} ({} not found in PATH)", "not found".red(), editor_bin);
        }
    }

    let config = crate::config::Config::load()?;

    println!();
    println!("  {}:", "Cloud sync".bold());

    print!("    API endpoint... ");
    match ureq::get(&format!("{}/health", config.api_endpoint))
        .timeout(std::time::Duration::from_secs(5))
        .call()
    {
        Ok(resp) if resp.status() == 200 => println!("{}", "reachable".green()),
        Ok(resp) => println!("{} (status {})", "degraded".yellow(), resp.status()),
        Err(e) => println!("{} ({})", "unreachable".red(), e),
    }

    print!("    Auth token... ");
    match &config.auth_mode {
        crate::config::AuthMode::Local => println!("{}", "not configured (local mode)".yellow()),
        crate::config::AuthMode::ApiKey | crate::config::AuthMode::OAuth => {
            match &config.auth_token {
                None => println!("{}", "missing".red()),
                Some(token) => {
                    match ureq::get(&format!("{}/auth/me", config.api_endpoint))
                        .set("Authorization", &format!("Bearer {}", token))
                        .timeout(std::time::Duration::from_secs(5))
                        .call()
                    {
                        Ok(resp) if resp.status() == 200 => println!("{}", "valid".green()),
                        Ok(resp) if resp.status() == 401 => {
                            println!("{}", "invalid or expired".red())
                        }
                        Ok(resp) => println!(
                            "{} (status {})",
                            "unexpected response".yellow(),
                            resp.status()
                        ),
                        Err(e) => println!("{} ({})", "check failed".red(), e),
                    }
                }
            }
        }
    }

    println!();
    println!("{}", "Health check complete.".green().bold());
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
