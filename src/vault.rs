use crate::cli::ExportArgs;
use crate::cli::*;
use crate::config::Config;
use crate::context;
use crate::script::{Script, ScriptLanguage, Visibility};
use crate::storage::StorageBackend;
use anyhow::{Context as _, Result, anyhow};
use chrono::Utc;
use colored::*;
use dialoguer::{Confirm, Input};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

fn bump_patch_version(version: &str) -> String {
    let v = version.trim_start_matches('v');
    let parts: Vec<u64> = v.split('.').filter_map(|p| p.parse().ok()).collect();
    if parts.len() == 3 {
        format!("v{}.{}.{}", parts[0], parts[1], parts[2] + 1)
    } else {
        format!("{}.1", version)
    }
}

pub fn save_script(args: SaveArgs) -> Result<()> {
    let config = Config::load()?;
    let storage = config.get_storage_backend()?;

    let script_path = Path::new(&args.file);
    if !script_path.exists() {
        return Err(anyhow!("Script file not found: {}", args.file));
    }

    let content = fs::read_to_string(script_path).context("Failed to read script file")?;

    let derived_name = script_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("Invalid script filename"))?
        .to_string();

    let name = args.name.clone().unwrap_or(derived_name);

    let extension = script_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("sh");

    let language = ScriptLanguage::from_extension(extension);
    let mut script = Script::new(name, content, language);

    script.context = context::detect_context()?;

    let existing = storage.load_script_by_name(&script.name).ok();

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

    if let Some(ref ex) = existing {
        let content_changed = ex.metadata.hash != script.metadata.hash;
        let meta_changed = ex.tags != script.tags || ex.description != script.description;

        if !content_changed && !meta_changed {
            println!("{} No changes: {}", "i".cyan(), script.name.yellow());
            return Ok(());
        }

        if content_changed {
            script.version = bump_patch_version(&ex.version);
        } else {
            script.version = ex.version.clone();
        }

        script.id = ex.id.clone();
        script.created_at = ex.created_at;
        script.metadata.use_count = ex.metadata.use_count;
        script.metadata.success_count = ex.metadata.success_count;
        script.metadata.failure_count = ex.metadata.failure_count;
        script.metadata.last_run = ex.metadata.last_run;
        script.metadata.last_run_by = ex.metadata.last_run_by.clone();
        script.metadata.avg_runtime_ms = ex.metadata.avg_runtime_ms;
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

pub fn update_script_from_file(args: UpdateArgs) -> Result<()> {
    let config = Config::load()?;
    let storage = config.get_storage_backend()?;

    let script_path = Path::new(&args.file);
    if !script_path.exists() {
        return Err(anyhow!("File not found: {}", args.file));
    }

    let derived_name = script_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("Invalid filename"))?
        .to_string();

    let name = args.name.unwrap_or(derived_name);

    let mut existing = storage.load_script_by_name(&name).map_err(|_| {
        anyhow!(
            "Script '{}' not found in vault. Use 'sv save' to add it first.",
            name
        )
    })?;

    let new_content = fs::read_to_string(script_path).context("Failed to read file")?;

    let mut hasher = Sha256::new();
    hasher.update(new_content.as_bytes());
    let new_hash = hex::encode(hasher.finalize());

    if new_hash == existing.metadata.hash {
        println!("{} No changes: {}", "i".cyan(), existing.name.yellow());
        return Ok(());
    }

    let old_version = existing.version.clone();
    existing.version = bump_patch_version(&existing.version);
    existing.content = new_content.clone();
    existing.metadata.hash = new_hash;
    existing.metadata.size_bytes = new_content.len();
    existing.metadata.line_count = new_content.lines().count();
    existing.updated_at = Utc::now();

    storage.update_script(&existing)?;

    println!(
        "{} Updated: {} {} -> {}",
        "✓".green().bold(),
        existing.name.yellow(),
        old_version.dimmed(),
        existing.version.green()
    );

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

    let mut filtered: Vec<&Script> = scripts
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

    if args.recent {
        filtered.sort_by(|a, b| b.metadata.last_run.cmp(&a.metadata.last_run));
    } else {
        filtered.sort_by(|a, b| a.name.cmp(&b.name));
    }

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

pub fn list_scripts(args: ListArgs) -> Result<()> {
    let config = Config::load()?;
    let storage = config.get_storage_backend()?;
    let mut scripts = storage.list_scripts()?;

    if scripts.is_empty() {
        println!("No scripts saved yet.");
        return Ok(());
    }

    if args.mine {
        if let Some(ref username) = config.username {
            scripts.retain(|s| s.author == *username);
        }
    } else if args.team {
        scripts.retain(|s| {
            s.visibility == Visibility::Team || s.visibility == Visibility::Public
        });
    }

    if scripts.is_empty() {
        println!("No scripts found matching your criteria.");
        return Ok(());
    }

    if args.recent {
        scripts.sort_by(|a, b| b.metadata.last_run.cmp(&a.metadata.last_run));
    } else {
        scripts.sort_by(|a, b| a.name.cmp(&b.name));
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
    println!(
        "  {}: {}",
        "Language".bold(),
        script.language.to_string().green()
    );
    println!("  {}: {}", "Author".bold(), script.author);
    println!(
        "  {}: {}",
        "Created".bold(),
        script.created_at.format("%Y-%m-%d %H:%M:%S")
    );

    if let Some(desc) = &script.description {
        println!("  {}: {}", "Description".bold(), desc);
    }

    if !script.tags.is_empty() {
        println!("  {}: {}", "Tags".bold(), script.tags.join(", ").cyan());
    }

    println!();
    println!("  {}:", "Context".bold());
    if let Some(dir) = &script.context.directory {
        println!("    Directory: {}", dir.yellow());
    }
    if let Some(repo) = &script.context.git_repo {
        println!("    Git repo:  {}", repo.green());
    }
    if let Some(branch) = &script.context.git_branch {
        println!("    Branch:    {}", branch.blue());
    }

    println!();
    if script.metadata.use_count > 0 {
        println!(
            "  {} runs, {:.1}% success{}",
            script.metadata.use_count,
            script.success_rate(),
            script
                .metadata
                .last_run
                .map(|t| format!(", last run {}", t.format("%Y-%m-%d")))
                .unwrap_or_default()
                .dimmed()
        );
    } else {
        println!("  {}", "Never run".dimmed());
    }

    println!(
        "  Run {} for full execution breakdown",
        format!("sv stats {}", script.name).yellow()
    );

    Ok(())
}

pub fn show_stats(args: StatsArgs) -> Result<()> {
    let config = Config::load()?;
    let storage = config.get_storage_backend()?;
    let script = storage
        .load_script_by_name(&args.name)
        .map_err(|_| anyhow!("Script not found: {}", args.name))?;

    println!("{}", script.name.cyan().bold());
    println!();

    println!("  {}:", "Content".bold());
    println!("    Language:  {}", script.language.to_string().green());
    println!("    Size:      {} bytes", script.metadata.size_bytes);
    println!("    Lines:     {}", script.metadata.line_count);
    let hash_prefix = if script.metadata.hash.len() >= 16 {
        &script.metadata.hash[..16]
    } else {
        &script.metadata.hash
    };
    println!("    Hash:      {}", hash_prefix.dimmed());

    println!();
    println!("  {}:", "Execution".bold());
    println!("    Total runs:   {}", script.metadata.use_count);
    println!("    Successful:   {}", script.metadata.success_count);
    println!("    Failed:       {}", script.metadata.failure_count);

    let rate = script.success_rate();
    let rate_colored = if rate >= 90.0 {
        format!("{:.1}%", rate).green().to_string()
    } else if rate >= 70.0 {
        format!("{:.1}%", rate).yellow().to_string()
    } else {
        format!("{:.1}%", rate).red().to_string()
    };
    println!("    Success rate: {}", rate_colored);

    if let Some(avg_ms) = script.metadata.avg_runtime_ms {
        println!("    Avg runtime:  {:.2}s", avg_ms as f64 / 1000.0);
    }

    if let Some(last_run) = script.metadata.last_run {
        println!();
        println!("  {}:", "Last Run".bold());
        println!("    Time: {}", last_run.format("%Y-%m-%d %H:%M:%S UTC"));
        if let Some(ref by) = script.metadata.last_run_by {
            println!("    By:   {}", by);
        }
    }

    Ok(())
}

pub fn cat_script(args: CatArgs) -> Result<()> {
    let config = Config::load()?;
    let storage = config.get_storage_backend()?;
    let script = storage
        .load_script_by_name(&args.name)
        .map_err(|_| anyhow!("Script not found: {}", args.name))?;

    print!("{}", script.content);

    Ok(())
}

pub fn edit_script(args: EditArgs) -> Result<()> {
    let config = Config::load()?;
    let storage = config.get_storage_backend()?;

    let mut script = storage
        .load_script_by_name(&args.name)
        .map_err(|_| anyhow!("Script not found: {}", args.name))?;

    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| "vi".to_string());

    let mut parts = editor.split_whitespace();
    let editor_cmd = parts.next().unwrap_or("vi").to_string();
    let editor_args: Vec<String> = parts.map(|s| s.to_string()).collect();

    let temp_dir = std::env::temp_dir().join("scriptvault");
    fs::create_dir_all(&temp_dir)?;

    let temp_filename = format!("{}.{}", script.name, script.language.extension());
    let temp_path = temp_dir.join(&temp_filename);

    fs::write(&temp_path, &script.content).context("Failed to write temporary file")?;

    let status = std::process::Command::new(&editor_cmd)
        .args(&editor_args)
        .arg(&temp_path)
        .status()
        .map_err(|e| {
            let _ = fs::remove_file(&temp_path);
            anyhow!("Failed to open editor '{}': {}", editor_cmd, e)
        })?;

    let read_result = fs::read_to_string(&temp_path);
    let _ = fs::remove_file(&temp_path);
    let new_content = read_result.context("Failed to read edited file")?;

    if !status.success() {
        println!("Edit cancelled");
        return Ok(());
    }

    let mut hasher = Sha256::new();
    hasher.update(new_content.as_bytes());
    let new_hash = hex::encode(hasher.finalize());

    if new_hash == script.metadata.hash {
        println!("No changes made");
        return Ok(());
    }

    let old_version = script.version.clone();
    script.version = bump_patch_version(&script.version);
    script.content = new_content.clone();
    script.metadata.hash = new_hash;
    script.metadata.size_bytes = new_content.len();
    script.metadata.line_count = new_content.lines().count();
    script.updated_at = Utc::now();

    storage.update_script(&script)?;

    println!(
        "{} Updated: {} {} -> {}",
        "✓".green().bold(),
        script.name.yellow(),
        old_version.dimmed(),
        script.version.green()
    );

    Ok(())
}

pub fn rename_script(args: RenameArgs) -> Result<()> {
    let config = Config::load()?;
    let storage = config.get_storage_backend()?;

    let mut script = storage
        .load_script_by_name(&args.old_name)
        .map_err(|_| anyhow!("Script not found: {}", args.old_name))?;

    if storage.load_script_by_name(&args.new_name).is_ok() {
        return Err(anyhow!("A script named '{}' already exists", args.new_name));
    }

    let old_name = script.name.clone();
    script.name = args.new_name.clone();
    script.updated_at = Utc::now();

    storage.update_script(&script)?;

    println!(
        "{} Renamed: {} -> {}",
        "✓".green().bold(),
        old_name.yellow(),
        args.new_name.yellow()
    );

    Ok(())
}

pub fn copy_script(args: CopyArgs) -> Result<()> {
    let config = Config::load()?;
    let storage = config.get_storage_backend()?;

    let source = storage
        .load_script_by_name(&args.source)
        .map_err(|_| anyhow!("Script not found: {}", args.source))?;

    if storage.load_script_by_name(&args.dest).is_ok() {
        return Err(anyhow!("A script named '{}' already exists", args.dest));
    }

    let mut copy = source.clone();
    copy.id = uuid::Uuid::new_v4().to_string();
    copy.name = args.dest.clone();
    copy.version = "v1.0.0".to_string();
    copy.created_at = Utc::now();
    copy.updated_at = Utc::now();
    copy.metadata.use_count = 0;
    copy.metadata.success_count = 0;
    copy.metadata.failure_count = 0;
    copy.metadata.last_run = None;
    copy.metadata.last_run_by = None;
    copy.metadata.avg_runtime_ms = None;

    storage.save_script(&copy)?;

    println!(
        "{} Copied: {} -> {}",
        "✓".green().bold(),
        args.source.yellow(),
        args.dest.yellow()
    );

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
    purge_script_history(&script.id)?;

    println!("{} Deleted: {}", "✓".green().bold(), args.name.yellow());

    Ok(())
}

fn purge_script_history(script_id: &str) -> Result<()> {
    let history_path = Config::history_path()?;

    if !history_path.exists() {
        return Ok(());
    }

    let contents = fs::read_to_string(&history_path)?;
    let retained: Vec<&str> = contents
        .lines()
        .filter(|line| {
            if line.is_empty() {
                return false;
            }
            serde_json::from_str::<serde_json::Value>(line)
                .ok()
                .and_then(|v| {
                    v.get("script_id")
                        .and_then(|id| id.as_str())
                        .map(|id| id != script_id)
                })
                .unwrap_or(true)
        })
        .collect();

    if retained.is_empty() {
        fs::write(&history_path, "")?;
    } else {
        fs::write(&history_path, format!("{}\n", retained.join("\n")))?;
    }

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
