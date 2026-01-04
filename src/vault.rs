use crate::cli::*;
use crate::config::Config;
use crate::context;
use crate::script::{Script, ScriptLanguage, Visibility};
use anyhow::{Context as _, Result, anyhow};
use colored::*;
use dialoguer::Input;
use std::fs;
use std::path::Path;

pub fn save_script(args: SaveArgs) -> Result<()> {
    let config = Config::load()?;

    // Read the script file
    let script_path = Path::new(&args.file);
    if !script_path.exists() {
        return Err(anyhow!("Script file not found: {}", args.file));
    }

    let content = fs::read_to_string(script_path).context("Failed to read script file")?;

    let name = script_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("Invalid script filename"))?
        .to_string();

    let extension = script_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("sh");

    let language = ScriptLanguage::from_extension(extension);

    let mut script = Script::new(name.clone(), content, language);

    // Detect context
    let ctx = context::detect_context()?;
    script.context = ctx;

    // Interactive prompts (unless --yes)
    if !args.yes {
        println!("{}", "Saving script to vault...".cyan().bold());
        println!();

        if let Some(ref dir) = script.context.directory {
            println!("  {}: {}", "Directory".bold(), dir.yellow());
        }
        if let Some(ref repo) = script.context.git_repo {
            println!("  {}: {}", "Git Repo".bold(), repo.green());
        }
        println!();

        // Get tags
        let tags_input: String = if let Some(tags) = args.tags {
            tags
        } else {
            Input::new()
                .with_prompt("Tags (space-separated)")
                .allow_empty(true)
                .interact_text()?
        };
        script.tags = tags_input
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        // Get description
        let description = if let Some(desc) = args.description {
            Some(desc)
        } else {
            let desc: String = Input::new()
                .with_prompt("Description (optional)")
                .allow_empty(true)
                .interact_text()?;
            if desc.is_empty() { None } else { Some(desc) }
        };
        script.description = description;
    } else {
        if let Some(tags) = args.tags {
            script.tags = tags.split_whitespace().map(|s| s.to_string()).collect();
        }
        script.description = args.description;
    }

    // Set author from config
    if let Some(username) = &config.username {
        script.author = username.clone();
    }

    // Save locally
    save_script_local(&script)?;

    println!();
    println!(
        "{} Script saved: {} {}",
        "✓".green().bold(),
        script.name.yellow(),
        script.version.dimmed()
    );
    println!("  ID: {}", script.id.dimmed());
    if !script.tags.is_empty() {
        println!("  Tags: {}", script.tags.join(", ").cyan());
    }

    Ok(())
}

pub fn find_scripts(args: FindArgs) -> Result<()> {
    let scripts = load_scripts_local()?;

    let current_ctx = if args.here {
        Some(context::detect_context()?)
    } else {
        None
    };

    let filtered: Vec<&Script> = scripts
        .iter()
        .filter(|s| {
            // Filter by query
            if let Some(ref query) = args.query {
                let query_lower = query.to_lowercase();
                let matches_name = s.name.to_lowercase().contains(&query_lower);
                let matches_desc = s
                    .description
                    .as_ref()
                    .map(|d| d.to_lowercase().contains(&query_lower))
                    .unwrap_or(false);
                let matches_tags = s
                    .tags
                    .iter()
                    .any(|t| t.to_lowercase().contains(&query_lower));

                if !(matches_name || matches_desc || matches_tags) {
                    return false;
                }
            }

            // Filter by context
            if let Some(ref ctx) = current_ctx {
                if !context::contexts_match(&s.context, ctx) {
                    return false;
                }
            }

            // Filter by tag
            if let Some(ref tag) = args.tag {
                if !s.tags.iter().any(|t| t == tag) {
                    return false;
                }
            }

            // Filter by language
            if let Some(ref lang) = args.language {
                if s.language.to_string() != lang {
                    return false;
                }
            }

            // Filter by visibility
            if args.team && s.visibility != Visibility::Team {
                return false;
            }

            true
        })
        .collect();

    if filtered.is_empty() {
        println!("No scripts found matching your criteria.");
        return Ok(());
    }

    println!("{}", "Scripts matching your search:".cyan().bold());
    println!();

    // Table header
    println!(
        "{:<30} {:<10} {:<8} {:<20}",
        "NAME".bold(),
        "VERSION".bold(),
        "USES".bold(),
        "LAST RUN".bold()
    );
    println!("{}", "─".repeat(70).dimmed());

    for script in filtered.iter().take(20) {
        let last_run = if let Some(run) = script.metadata.last_run {
            let duration = chrono::Utc::now() - run;
            if duration.num_days() > 0 {
                format!("{} days ago", duration.num_days())
            } else if duration.num_hours() > 0 {
                format!("{} hours ago", duration.num_hours())
            } else {
                format!("{} minutes ago", duration.num_minutes())
            }
        } else {
            "Never".dimmed().to_string()
        };

        println!(
            "{:<30} {:<10} {:<8} {:<20}",
            script.name.yellow(),
            script.version.dimmed(),
            script.metadata.use_count.to_string().green(),
            last_run
        );
    }

    if filtered.len() > 20 {
        println!();
        println!("... and {} more", filtered.len() - 20);
    }

    Ok(())
}

pub fn list_scripts(_args: ListArgs) -> Result<()> {
    let scripts = load_scripts_local()?;

    println!("{}", "Your Scripts".cyan().bold());
    println!();

    for script in scripts {
        println!("  {} {}", script.name.yellow(), script.version.dimmed());
        if let Some(desc) = &script.description {
            println!("    {}", desc.dimmed());
        }
        if !script.tags.is_empty() {
            println!("    Tags: {}", script.tags.join(", ").cyan());
        }
        println!();
    }

    Ok(())
}

pub fn show_info(args: InfoArgs) -> Result<()> {
    let scripts = load_scripts_local()?;
    let script = scripts
        .iter()
        .find(|s| s.name == args.name)
        .ok_or_else(|| anyhow!("Script not found: {}", args.name))?;

    println!("{}", format!("Script: {}", script.name).cyan().bold());
    println!();
    println!("  {}: {}", "Version".bold(), script.version.yellow());
    println!(
        "  {}: {}",
        "Language".bold(),
        script.language.to_string().green()
    );
    println!("  {}: {}", "Author".bold(), script.author);

    if let Some(desc) = &script.description {
        println!("  {}: {}", "Description".bold(), desc);
    }

    if !script.tags.is_empty() {
        println!("  {}: {}", "Tags".bold(), script.tags.join(", ").cyan());
    }

    println!();
    println!("  {}:", "Statistics".bold());
    println!("    Uses: {}", script.metadata.use_count);
    println!("    Success Rate: {:.1}%", script.success_rate());

    if let Some(last_run) = script.metadata.last_run {
        println!("    Last Run: {}", last_run.format("%Y-%m-%d %H:%M:%S"));
    }

    println!();
    println!("  {}:", "Context".bold());
    if let Some(dir) = &script.context.directory {
        println!("    Directory: {}", dir.yellow());
    }
    if let Some(repo) = &script.context.git_repo {
        println!("    Git Repo: {}", repo.green());
    }

    Ok(())
}

pub(crate) fn update_script_metadata(updated_script: &Script) -> Result<()> {
    let mut scripts = load_scripts_local().unwrap_or_default();

    // Find and update the script
    if let Some(script) = scripts.iter_mut().find(|s| s.id == updated_script.id) {
        *script = updated_script.clone();
    } else {
        return Err(anyhow!("Script not found for metadata update"));
    }

    // Save back to file
    let scripts_path = Config::scripts_path()?;
    let json = serde_json::to_string_pretty(&scripts)?;
    fs::write(scripts_path, json)?;

    Ok(())
}

pub fn show_stats(_args: StatsArgs) -> Result<()> {
    println!("Stats feature coming soon...");
    Ok(())
}

pub fn show_versions(_args: VersionArgs) -> Result<()> {
    println!("Versions feature coming soon...");
    Ok(())
}

pub fn diff_versions(_args: DiffArgs) -> Result<()> {
    println!("Diff feature coming soon...");
    Ok(())
}

pub fn checkout_version(_args: CheckoutArgs) -> Result<()> {
    println!("Checkout feature coming soon...");
    Ok(())
}

pub fn share_script(_args: ShareArgs) -> Result<()> {
    println!("Share feature coming soon...");
    Ok(())
}

pub fn list_team_members() -> Result<()> {
    println!("Team members feature coming soon...");
    Ok(())
}

pub fn list_team_scripts() -> Result<()> {
    println!("Team scripts feature coming soon...");
    Ok(())
}

pub fn show_permissions() -> Result<()> {
    println!("Permissions feature coming soon...");
    Ok(())
}

pub fn recommend_scripts() -> Result<()> {
    println!("Recommendations feature coming soon...");
    Ok(())
}

pub fn export_scripts(_args: ExportArgs) -> Result<()> {
    println!("Export feature coming soon...");
    Ok(())
}

// Local storage helpers
fn save_script_local(script: &Script) -> Result<()> {
    let mut scripts = load_scripts_local().unwrap_or_default();

    // Remove existing script with same name
    scripts.retain(|s| s.name != script.name);

    scripts.push(script.clone());

    let scripts_path = Config::scripts_path()?;
    let json = serde_json::to_string_pretty(&scripts)?;
    fs::write(scripts_path, json)?;

    Ok(())
}

pub(crate) fn load_scripts_local() -> Result<Vec<Script>> {
    let scripts_path = Config::scripts_path()?;

    if !scripts_path.exists() {
        return Ok(Vec::new());
    }

    let contents = fs::read_to_string(scripts_path)?;
    let scripts: Vec<Script> = serde_json::from_str(&contents)?;

    Ok(scripts)
}
