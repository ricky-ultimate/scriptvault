use anyhow::Result;
use colored::*;

fn health_url(api_endpoint: &str) -> String {
    if let Some(base) = api_endpoint.strip_suffix("/v1") {
        format!("{}/health", base)
    } else {
        format!("{}/health", api_endpoint)
    }
}

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

    println!();
    println!("  {}:", "SSH".bold());
    check_ssh_doctor();

    let config = crate::config::Config::load()?;
    let probe_url = health_url(&config.api_endpoint);

    println!();
    println!("  {}:", "Cloud sync".bold());

    print!("    API endpoint... ");
    match ureq::get(&probe_url)
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

fn check_ssh_doctor() {
    print!("    ssh binary... ");
    if which::which("ssh").is_ok() {
        println!("{}", "ok".green());
    } else {
        println!("{}", "not found".yellow());
        return;
    }

    print!("    ssh-agent socket... ");
    match std::env::var("SSH_AUTH_SOCK") {
        Err(_) => {
            println!(
                "{} (SSH_AUTH_SOCK not set)",
                "not running".yellow()
            );
            return;
        }
        Ok(sock) if sock.is_empty() => {
            println!(
                "{} (SSH_AUTH_SOCK is empty)",
                "not running".yellow()
            );
            return;
        }
        Ok(sock) => {
            if std::path::Path::new(&sock).exists() {
                println!("{}", "ok".green());
            } else {
                println!(
                    "{} (socket path does not exist: {})",
                    "stale".yellow(),
                    sock
                );
                return;
            }
        }
    }

    print!("    ssh-agent keys... ");
    match std::process::Command::new("ssh-add")
        .arg("-l")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
    {
        Err(_) => println!("{}", "ssh-add not found".yellow()),
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            match out.status.code() {
                Some(0) => {
                    let count = stdout.lines().count();
                    println!("{} ({} loaded)", "ok".green(), count);
                }
                Some(1) => {
                    let msg = stderr.trim();
                    if msg.contains("no identities") || stdout.contains("no identities") {
                        println!("{}", "no keys loaded".yellow());
                    } else {
                        println!("{} ({})", "agent error".yellow(), msg);
                    }
                }
                Some(2) => println!("{} (cannot connect to agent)", "error".red()),
                _ => println!("{}", "unknown".yellow()),
            }
        }
    }
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
