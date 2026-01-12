use crate::cli::ExportArgs;
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

pub fn export_scripts(args: ExportArgs) -> Result<()> {
    let scripts = load_scripts_local()?;

    if scripts.is_empty() {
        println!("No scripts to export.");
        return Ok(());
    }

    let output = match args.format.to_lowercase().as_str() {
        "json" => export_json(&scripts)?,
        "markdown" | "md" => export_markdown(&scripts)?,
        _ => {
            return Err(anyhow!(
                "Unknown export format: '{}'. Supported formats: json, markdown",
                args.format
            ));
        }
    };

    // Write to file or stdout
    if let Some(output_file) = args.output {
        fs::write(&output_file, output)?;
        println!(
            "{} Exported {} scripts to: {}",
            "✓".green().bold(),
            scripts.len(),
            output_file.yellow()
        );
    } else {
        println!("{}", output);
    }

    Ok(())
}

fn export_json(scripts: &[Script]) -> Result<String> {
    #[derive(serde::Serialize)]
    struct ExportData {
        exported_at: String,
        export_version: String,
        total_scripts: usize,
        scripts: Vec<Script>,
    }

    let data = ExportData {
        exported_at: chrono::Utc::now().to_rfc3339(),
        export_version: "1.0".to_string(),
        total_scripts: scripts.len(),
        scripts: scripts.to_vec(),
    };

    Ok(serde_json::to_string_pretty(&data)?)
}

fn export_markdown(scripts: &[Script]) -> Result<String> {
    let mut output = String::new();

    // Header
    output.push_str("# ScriptVault Export\n\n");
    output.push_str(&format!(
        "**Exported:** {}\n\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    ));
    output.push_str(&format!("**Total Scripts:** {}\n\n", scripts.len()));

    // Table of contents
    output.push_str("## Table of Contents\n\n");
    for script in scripts {
        output.push_str(&format!(
            "- [{}](#{})\n",
            script.name,
            script.name.to_lowercase().replace(' ', "-")
        ));
    }
    output.push_str("\n---\n\n");

    // Individual scripts
    for script in scripts {
        output.push_str(&format!("## {}\n\n", script.name));

        // Metadata table
        output.push_str("| Property | Value |\n");
        output.push_str("|----------|-------|\n");
        output.push_str(&format!(
            "| **Language** | {} |\n",
            script.language.to_string()
        ));
        output.push_str(&format!("| **Version** | {} |\n", script.version));
        output.push_str(&format!("| **Author** | {} |\n", script.author));

        if !script.tags.is_empty() {
            output.push_str(&format!("| **Tags** | {} |\n", script.tags.join(", ")));
        }

        if let Some(desc) = &script.description {
            output.push_str(&format!("| **Description** | {} |\n", desc));
        }

        output.push_str(&format!(
            "| **Created** | {} |\n",
            script.created_at.format("%Y-%m-%d %H:%M:%S")
        ));

        // Statistics (if script has been used)
        if script.metadata.use_count > 0 {
            output.push_str(&format!("| **Uses** | {} |\n", script.metadata.use_count));
            output.push_str(&format!(
                "| **Success Rate** | {:.1}% ({}/{} runs) |\n",
                script.success_rate(),
                script.metadata.success_count,
                script.metadata.use_count
            ));

            if let Some(avg) = script.metadata.avg_runtime_ms {
                output.push_str(&format!(
                    "| **Avg Runtime** | {:.2}s |\n",
                    avg as f64 / 1000.0
                ));
            }

            if let Some(last_run) = script.metadata.last_run {
                output.push_str(&format!(
                    "| **Last Run** | {} |\n",
                    last_run.format("%Y-%m-%d %H:%M:%S")
                ));
            }
        }

        output.push_str("\n");

        // Context (if available)
        if script.context.directory.is_some()
            || script.context.git_repo.is_some()
            || script.context.git_branch.is_some()
        {
            output.push_str("### Context\n\n");

            if let Some(dir) = &script.context.directory {
                output.push_str(&format!("- **Directory:** `{}`\n", dir));
            }
            if let Some(repo) = &script.context.git_repo {
                output.push_str(&format!("- **Git Repo:** `{}`\n", repo));
            }
            if let Some(branch) = &script.context.git_branch {
                output.push_str(&format!("- **Branch:** `{}`\n", branch));
            }
            output.push_str("\n");
        }

        // Script content
        output.push_str("### Script\n\n");
        output.push_str(&format!(
            "```{}\n{}\n```\n\n",
            script.language.to_string(),
            script.content
        ));

        // Command to run
        output.push_str(&format!("**Run:** `sv run {}`\n\n", script.name));

        output.push_str("---\n\n");
    }

    // Footer with helpful commands
    output.push_str("## ScriptVault Commands\n\n");
    output.push_str("```bash\n");
    output.push_str("# Find scripts\n");
    output.push_str("sv find <query>\n");
    output.push_str("sv find --tag <tag>\n");
    output.push_str("sv find --here\n\n");
    output.push_str("# Run scripts\n");
    output.push_str("sv run <script-name>\n");
    output.push_str("sv run <script-name> --dry-run\n");
    output.push_str("sv run <script-name> --verbose\n\n");
    output.push_str("# View information\n");
    output.push_str("sv info <script-name>\n");
    output.push_str("sv history [<script-name>]\n");
    output.push_str("sv list\n");
    output.push_str("```\n\n");

    output.push_str(&format!(
        "*Exported from ScriptVault on {}*\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    ));

    Ok(output)
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
