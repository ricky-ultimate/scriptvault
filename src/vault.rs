use crate::cli::ExportArgs;
use crate::cli::*;
use crate::config::Config;
use crate::context;
use crate::script::{Script, ScriptLanguage, Visibility};
use crate::storage::StorageBackend;
use anyhow::{anyhow, Context as _, Result};
use colored::*;
use dialoguer::{Confirm, Input};
use std::fs;
use std::path::Path;

pub fn save_script(args: SaveArgs) -> Result<()> {
    let config = Config::load()?;
    let storage = config.get_storage_backend()?;

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
    let mut script = Script::new(name, content, language);

    script.context = context::detect_context()?;

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

        script.description = if let Some(desc) = args.description {
            Some(desc)
        } else {
            let desc: String = Input::new()
                .with_prompt("Description (optional)")
                .allow_empty(true)
                .interact_text()?;
            if desc.is_empty() { None } else { Some(desc) }
        };
    } else {
        if let Some(tags) = args.tags {
            script.tags = tags.split_whitespace().map(|s| s.to_string()).collect();
        }
        script.description = args.description;
    }

    if let Some(username) = &config.username {
        script.author = username.clone();
    }

    storage.save_script(&script)?;

    println!();
    println!(
        "{} Saved: {} {}",
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
    let config = Config::load()?;
    let storage = config.get_storage_backend()?;
    let scripts = storage.list_scripts()?;

    let current_ctx = if args.here {
        Some(context::detect_context()?)
    } else {
        None
    };

    let filtered: Vec<&Script> = scripts
        .iter()
        .filter(|s| {
            if let Some(ref query) = args.query {
                let q = query.to_lowercase();
                let matches = s.name.to_lowercase().contains(&q)
                    || s.description
                        .as_ref()
                        .map(|d| d.to_lowercase().contains(&q))
                        .unwrap_or(false)
                    || s.tags.iter().any(|t| t.to_lowercase().contains(&q));
                if !matches {
                    return false;
                }
            }

            if let Some(ref ctx) = current_ctx {
                if !context::contexts_match(&s.context, ctx) {
                    return false;
                }
            }

            if let Some(ref tag) = args.tag {
                if !s.tags.iter().any(|t| t == tag) {
                    return false;
                }
            }

            if let Some(ref lang) = args.language {
                if s.language.to_string() != *lang {
                    return false;
                }
            }

            if args.team && s.visibility != Visibility::Team {
                return false;
            }

            if let Some(ref repo) = args.git_repo {
                if s.context.git_repo.as_deref() != Some(repo.as_str()) {
                    return false;
                }
            }

            true
        })
        .collect();

    if filtered.is_empty() {
        println!("No scripts found matching your criteria.");
        return Ok(());
    }

    println!("{}", "Scripts".cyan().bold());
    println!();
    println!(
        "{:<30} {:<10} {:<8} {:<20}",
        "NAME".bold(),
        "VERSION".bold(),
        "USES".bold(),
        "LAST RUN".bold()
    );
    println!("{}", "─".repeat(70).dimmed());

    for script in filtered.iter().take(20) {
        let last_run = match script.metadata.last_run {
            Some(run) => {
                let delta = chrono::Utc::now() - run;
                if delta.num_days() > 0 {
                    format!("{} days ago", delta.num_days())
                } else if delta.num_hours() > 0 {
                    format!("{} hours ago", delta.num_hours())
                } else {
                    format!("{} minutes ago", delta.num_minutes())
                }
            }
            None => "Never".dimmed().to_string(),
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
    let config = Config::load()?;
    let storage = config.get_storage_backend()?;
    let scripts = storage.list_scripts()?;

    if scripts.is_empty() {
        println!("No scripts saved yet.");
        return Ok(());
    }

    println!("{}", "Scripts".cyan().bold());
    println!();

    for script in &scripts {
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
    let config = Config::load()?;
    let storage = config.get_storage_backend()?;
    let script = storage.load_script_by_name(&args.name)?;

    println!("{}", script.name.cyan().bold());
    println!();
    println!("  {}: {}", "Version".bold(), script.version.yellow());
    println!("  {}: {}", "Language".bold(), script.language.to_string().green());
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
    println!("    Success rate: {:.1}%", script.success_rate());

    if let Some(last_run) = script.metadata.last_run {
        println!("    Last run: {}", last_run.format("%Y-%m-%d %H:%M:%S"));
    }

    if let Some(avg_ms) = script.metadata.avg_runtime_ms {
        println!("    Avg runtime: {:.2}s", avg_ms as f64 / 1000.0);
    }

    println!();
    println!("  {}:", "Context".bold());
    if let Some(dir) = &script.context.directory {
        println!("    Directory: {}", dir.yellow());
    }
    if let Some(repo) = &script.context.git_repo {
        println!("    Git repo: {}", repo.green());
    }
    if let Some(branch) = &script.context.git_branch {
        println!("    Branch: {}", branch.blue());
    }

    Ok(())
}

pub fn delete_script(args: DeleteArgs) -> Result<()> {
    let config = Config::load()?;
    let storage = config.get_storage_backend()?;

    let script = storage
        .load_script_by_name(&args.name)
        .map_err(|_| anyhow!("Script not found: {}", args.name))?;

    if !args.yes {
        println!("{}", script.name.yellow().bold());

        if let Some(desc) = &script.description {
            println!("  {}", desc.dimmed());
        }
        if !script.tags.is_empty() {
            println!("  Tags: {}", script.tags.join(", ").cyan());
        }
        println!("  Uses: {}", script.metadata.use_count);
        println!();

        let confirmed = Confirm::new()
            .with_prompt("Delete this script?")
            .default(false)
            .interact()?;

        if !confirmed {
            println!("Cancelled");
            return Ok(());
        }
    }

    storage.delete_script(&script.id)?;

    println!("{} Deleted: {}", "✓".green().bold(), args.name.yellow());

    Ok(())
}

pub(crate) fn update_script_metadata(updated_script: &Script) -> Result<()> {
    let config = Config::load()?;
    let storage = config.get_storage_backend()?;
    storage.update_script(updated_script)
}

pub(crate) fn load_scripts_local() -> Result<Vec<Script>> {
    let config = Config::load()?;
    let storage = config.get_storage_backend()?;
    storage.list_scripts()
}

pub fn show_stats(_args: StatsArgs) -> Result<()> {
    println!("Stats command is not yet implemented.");
    Ok(())
}

pub fn show_versions(_args: VersionArgs) -> Result<()> {
    println!("Versions command is not yet implemented.");
    Ok(())
}

pub fn diff_versions(_args: DiffArgs) -> Result<()> {
    println!("Diff command is not yet implemented.");
    Ok(())
}

pub fn checkout_version(_args: CheckoutArgs) -> Result<()> {
    println!("Checkout command is not yet implemented.");
    Ok(())
}

pub fn share_script(_args: ShareArgs) -> Result<()> {
    println!("Share command is not yet implemented.");
    Ok(())
}

pub fn list_team_members() -> Result<()> {
    println!("Team command is not yet implemented.");
    Ok(())
}

pub fn list_team_scripts() -> Result<()> {
    println!("Team command is not yet implemented.");
    Ok(())
}

pub fn show_permissions() -> Result<()> {
    println!("Permissions command is not yet implemented.");
    Ok(())
}

pub fn recommend_scripts() -> Result<()> {
    println!("Recommend command is not yet implemented.");
    Ok(())
}

pub fn export_scripts(args: ExportArgs) -> Result<()> {
    let config = Config::load()?;
    let storage = config.get_storage_backend()?;
    let scripts = storage.list_scripts()?;

    if scripts.is_empty() {
        println!("No scripts to export.");
        return Ok(());
    }

    let output = match args.format.to_lowercase().as_str() {
        "json" => export_json(&scripts)?,
        "markdown" | "md" => export_markdown(&scripts)?,
        _ => {
            return Err(anyhow!(
                "Unknown format: '{}'. Supported: json, markdown",
                args.format
            ));
        }
    };

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
    struct ExportData<'a> {
        exported_at: String,
        export_version: &'a str,
        total_scripts: usize,
        scripts: &'a [Script],
    }

    let data = ExportData {
        exported_at: chrono::Utc::now().to_rfc3339(),
        export_version: "1.0",
        total_scripts: scripts.len(),
        scripts,
    };

    Ok(serde_json::to_string_pretty(&data)?)
}

fn export_markdown(scripts: &[Script]) -> Result<String> {
    let mut out = String::new();

    out.push_str("# ScriptVault Export\n\n");
    out.push_str(&format!(
        "Exported: {}\n\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    ));
    out.push_str(&format!("Total scripts: {}\n\n", scripts.len()));
    out.push_str("## Contents\n\n");

    for script in scripts {
        out.push_str(&format!(
            "- [{}](#{})\n",
            script.name,
            script.name.to_lowercase().replace(' ', "-")
        ));
    }

    out.push_str("\n---\n\n");

    for script in scripts {
        out.push_str(&format!("## {}\n\n", script.name));
        out.push_str("| Property | Value |\n");
        out.push_str("|----------|-------|\n");
        out.push_str(&format!("| Language | {} |\n", script.language));
        out.push_str(&format!("| Version | {} |\n", script.version));
        out.push_str(&format!("| Author | {} |\n", script.author));

        if !script.tags.is_empty() {
            out.push_str(&format!("| Tags | {} |\n", script.tags.join(", ")));
        }

        if let Some(desc) = &script.description {
            out.push_str(&format!("| Description | {} |\n", desc));
        }

        out.push_str(&format!(
            "| Created | {} |\n",
            script.created_at.format("%Y-%m-%d %H:%M:%S")
        ));

        if script.metadata.use_count > 0 {
            out.push_str(&format!("| Uses | {} |\n", script.metadata.use_count));
            out.push_str(&format!(
                "| Success rate | {:.1}% |\n",
                script.success_rate()
            ));
        }

        out.push_str("\n");

        if script.context.directory.is_some() || script.context.git_repo.is_some() {
            out.push_str("### Context\n\n");
            if let Some(dir) = &script.context.directory {
                out.push_str(&format!("- Directory: `{}`\n", dir));
            }
            if let Some(repo) = &script.context.git_repo {
                out.push_str(&format!("- Git repo: `{}`\n", repo));
            }
            if let Some(branch) = &script.context.git_branch {
                out.push_str(&format!("- Branch: `{}`\n", branch));
            }
            out.push_str("\n");
        }

        out.push_str("### Script\n\n");
        out.push_str(&format!(
            "```{}\n{}\n```\n\n",
            script.language, script.content
        ));
        out.push_str(&format!("Run: `sv run {}`\n\n", script.name));
        out.push_str("---\n\n");
    }

    Ok(out)
}
