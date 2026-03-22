use crate::cli::{HistoryArgs, RunArgs};
use crate::config::Config;
use crate::constants::*;
use crate::context;
use crate::script::{ExecutionRecord, Script, ScriptLanguage};
use crate::vault::{load_scripts_local, update_script_metadata};
use anyhow::{anyhow, Result};
use colored::*;
use dialoguer::Confirm;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Instant;

pub fn run_script(args: RunArgs) -> Result<()> {
    if args.sandbox {
        return Err(anyhow!(
            "Sandbox execution is not yet available in this version"
        ));
    }

    let config = Config::load()?;
    let ci_mode = args.ci || std::env::var(ENV_SCRIPTVAULT_CI).is_ok();

    let scripts = load_scripts_local()?;
    let mut script = scripts
        .iter()
        .find(|s| s.name == args.script)
        .ok_or_else(|| anyhow!("Script not found: {}", args.script))?
        .clone();

    if !script.is_safe() {
        println!(
            "{}",
            "Warning: This script contains potentially dangerous commands"
                .red()
                .bold()
        );
        if !ci_mode && !args.dry_run {
            let proceed = Confirm::new()
                .with_prompt("Run this script?")
                .default(false)
                .interact()?;
            if !proceed {
                println!("Execution cancelled");
                return Ok(());
            }
        }
    }

    show_script_preview(&script)?;

    let needs_confirm = args.confirm || (config.confirm_before_run && !ci_mode);
    if needs_confirm && !args.dry_run {
        println!();
        let proceed = Confirm::new()
            .with_prompt("Run this script?")
            .default(true)
            .interact()?;
        if !proceed {
            println!("Execution cancelled");
            return Ok(());
        }
    }

    if args.dry_run {
        println!();
        println!("{}", "Dry run complete. Script was not executed.".yellow());
        return Ok(());
    }

    println!();
    println!("{}", "Executing...".cyan().bold());
    println!();

    let start = Instant::now();
    let result = execute_script(&script, &args.args, args.verbose)?;
    let duration = start.elapsed();

    let exit_code = result.exit_code;
    let ctx = context::detect_context()?;

    let execution = ExecutionRecord {
        id: uuid::Uuid::new_v4().to_string(),
        script_id: script.id.clone(),
        script_version: script.version.clone(),
        executed_by: config.username.clone().unwrap_or_else(|| default_author()),
        executed_at: chrono::Utc::now(),
        exit_code,
        duration_ms: duration.as_millis() as u64,
        output: result.output,
        error: result.error,
        context: ctx,
    };

    save_execution_record(&execution)?;

    let prev_count = script.metadata.use_count;
    script.metadata.use_count += 1;
    script.metadata.last_run = Some(execution.executed_at);
    script.metadata.last_run_by = Some(execution.executed_by.clone());

    if exit_code == 0 {
        script.metadata.success_count += 1;
    } else {
        script.metadata.failure_count += 1;
    }

    script.metadata.avg_runtime_ms = Some(match script.metadata.avg_runtime_ms {
        Some(avg) => {
            (avg * prev_count + duration.as_millis() as u64) / script.metadata.use_count
        }
        None => duration.as_millis() as u64,
    });

    update_script_metadata(&script)?;

    println!();
    if exit_code == 0 {
        println!(
            "{} Completed in {:.2}s",
            "✓".green().bold(),
            duration.as_secs_f64()
        );
    } else {
        println!(
            "{} Failed with exit code {} in {:.2}s",
            "✗".red().bold(),
            exit_code,
            duration.as_secs_f64()
        );
    }

    Ok(())
}

fn show_script_preview(script: &Script) -> Result<()> {
    println!("╭{}╮", "─".repeat(60));
    println!(
        "│  {} {}",
        script.name.yellow().bold(),
        script.version.dimmed()
    );
    println!("├{}┤", "─".repeat(60));

    if !script.tags.is_empty() {
        println!("│  Tags: {}", script.tags.join(", ").cyan());
    }

    if let Some(desc) = &script.description {
        println!("│  Description: {}", desc);
    }

    println!("│");
    println!("│  Language: {}", script.language.to_string().green());

    if let Some(dir) = &script.context.directory {
        println!("│  Directory: {}", dir.yellow());
    }

    if script.metadata.use_count > 0 {
        println!(
            "│  Success rate: {:.1}% ({}/{})",
            script.success_rate(),
            script.metadata.success_count,
            script.metadata.use_count
        );
    }

    println!("╰{}╯", "─".repeat(60));
    Ok(())
}

struct ExecutionResult {
    exit_code: i32,
    output: Option<String>,
    error: Option<String>,
}

fn execute_script(script: &Script, args: &[String], verbose: bool) -> Result<ExecutionResult> {
    let temp_dir = std::env::temp_dir().join("scriptvault");
    fs::create_dir_all(&temp_dir)?;

    let temp_filename = format!(
        "{}.{}",
        uuid::Uuid::new_v4(),
        get_extension(&script.language)
    );
    let script_path = temp_dir.join(temp_filename);

    fs::write(&script_path, &script.content)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&script_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms)?;
    }

    let (interpreter, interpreter_args) = get_interpreter_command(&script.language);

    if verbose {
        println!("  Interpreter: {}", interpreter);
        println!("  Script: {}", script_path.display());
        if !args.is_empty() {
            println!("  Arguments: {}", args.join(" "));
        }
        println!();
    }

    let output = Command::new(interpreter)
        .args(&interpreter_args)
        .arg(&script_path)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    if let Err(e) = fs::remove_file(&script_path) {
        eprintln!("Warning: failed to remove temporary file: {}", e);
    }

    let output = output?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !stdout.is_empty() {
        print!("{}", stdout);
    }
    if !stderr.is_empty() {
        eprint!("{}", stderr);
    }

    Ok(ExecutionResult {
        exit_code: output.status.code().unwrap_or(1),
        output: if stdout.is_empty() { None } else { Some(stdout) },
        error: if stderr.is_empty() { None } else { Some(stderr) },
    })
}

fn get_interpreter_command(language: &ScriptLanguage) -> (&'static str, Vec<&'static str>) {
    match language {
        ScriptLanguage::Bash => (BASH_INTERPRETER, vec![]),
        ScriptLanguage::Shell => (SHELL_INTERPRETER, vec![]),
        ScriptLanguage::Python => (PYTHON_INTERPRETER, vec![]),
        ScriptLanguage::Ruby => (RUBY_INTERPRETER, vec![]),
        ScriptLanguage::Perl => (PERL_INTERPRETER, vec![]),
        ScriptLanguage::PowerShell => (POWERSHELL_INTERPRETER, vec!["-File"]),
        _ => (BASH_INTERPRETER, vec![]),
    }
}

fn get_extension(language: &ScriptLanguage) -> &'static str {
    match language {
        ScriptLanguage::Bash | ScriptLanguage::Shell => "sh",
        ScriptLanguage::Python => "py",
        ScriptLanguage::JavaScript => "js",
        ScriptLanguage::Ruby => "rb",
        ScriptLanguage::Perl => "pl",
        ScriptLanguage::PowerShell => "ps1",
        ScriptLanguage::Batch => "bat",
        _ => "sh",
    }
}

pub fn show_history(args: HistoryArgs) -> Result<()> {
    let history_path = Config::history_path()?;

    if !history_path.exists() {
        println!("No execution history found.");
        return Ok(());
    }

    let contents = fs::read_to_string(history_path)?;
    let records: Vec<ExecutionRecord> = contents
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();

    let scripts = load_scripts_local()?;
    let script_map: HashMap<String, String> = scripts
        .iter()
        .map(|s| (s.id.clone(), s.name.clone()))
        .collect();

    let filtered: Vec<&ExecutionRecord> = records
        .iter()
        .filter(|r| {
            if let Some(ref script_name) = args.script {
                let matched_id = scripts
                    .iter()
                    .find(|s| s.name == *script_name)
                    .map(|s| s.id.as_str());
                if let Some(id) = matched_id {
                    if r.script_id != id {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            if args.failed && r.exit_code == 0 {
                return false;
            }
            true
        })
        .collect();

    if filtered.is_empty() {
        println!("No execution history found.");
        return Ok(());
    }

    println!("{}", "Execution History".cyan().bold());
    println!();
    println!(
        "{:<20} {:<20} {:<15} {:<10} {:<10}",
        "TIME".bold(),
        "SCRIPT".bold(),
        "USER".bold(),
        "EXIT CODE".bold(),
        "DURATION".bold()
    );
    println!("{}", "─".repeat(78).dimmed());

    let limit = if args.recent { 10 } else { DEFAULT_HISTORY_LIMIT };

    for record in filtered.iter().rev().take(limit) {
        let time = record.executed_at.format("%Y-%m-%d %H:%M:%S");
        let script_name = script_map
            .get(&record.script_id)
            .map(|s| s.as_str())
            .unwrap_or(&record.script_id);
        let exit_status = if record.exit_code == 0 {
            record.exit_code.to_string().green()
        } else {
            record.exit_code.to_string().red()
        };
        let duration = format!("{:.2}s", record.duration_ms as f64 / 1000.0);

        println!(
            "{:<20} {:<20} {:<15} {:<10} {:<10}",
            time.to_string().dimmed(),
            script_name.yellow(),
            record.executed_by,
            exit_status,
            duration
        );
    }

    Ok(())
}

fn save_execution_record(record: &ExecutionRecord) -> Result<()> {
    let history_path = Config::history_path()?;

    {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&history_path)?;
        let json = serde_json::to_string(record)?;
        writeln!(file, "{}", json)?;
    }

    rotate_history(&history_path)?;

    Ok(())
}

fn rotate_history(path: &Path) -> Result<()> {
    let contents = fs::read_to_string(path)?;
    let lines: Vec<&str> = contents.lines().filter(|l| !l.is_empty()).collect();

    if lines.len() > MAX_HISTORY_ENTRIES {
        let trimmed = lines[lines.len() - MAX_HISTORY_ENTRIES..].join("\n");
        fs::write(path, format!("{}\n", trimmed))?;
    }

    Ok(())
}
