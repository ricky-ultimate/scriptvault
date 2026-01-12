use crate::cli::{HistoryArgs, RunArgs};
use crate::config::Config;
use crate::constants::*;
use crate::context;
use crate::script::{ExecutionRecord, Script, ScriptLanguage};
use crate::vault::{load_scripts_local, update_script_metadata};
use anyhow::{Result, anyhow};
use colored::*;
use dialoguer::Confirm;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Instant;

pub fn run_script(args: RunArgs) -> Result<()> {
    let config = Config::load()?;

    // Check for CI mode from environment variable
    let ci_mode = args.ci || std::env::var(ENV_SCRIPTVAULT_CI).is_ok();

    // Load script from vault
    let scripts = load_scripts_local()?;
    let mut script = scripts
        .iter()
        .find(|s| s.name == args.script)
        .ok_or_else(|| anyhow!("Script not found: {}", args.script))?
        .clone();

    // Safety check
    if !script.is_safe() {
        println!(
            "{}",
            "⚠ Warning: This script contains potentially dangerous commands!"
                .red()
                .bold()
        );
        if !ci_mode && !args.dry_run {
            let proceed = Confirm::new()
                .with_prompt("Are you sure you want to run this script?")
                .default(false)
                .interact()?;

            if !proceed {
                println!("Execution cancelled.");
                return Ok(());
            }
        }
    }

    // Show preview
    show_script_preview(&script, &args)?;

    // Confirm execution
    if config.confirm_before_run && !ci_mode && !args.dry_run {
        println!();
        let proceed = Confirm::new()
            .with_prompt("Run this script?")
            .default(true)
            .interact()?;

        if !proceed {
            println!("Execution cancelled.");
            return Ok(());
        }
    }

    if args.dry_run {
        println!();
        println!(
            "{}",
            "Dry run - script would execute with these settings".yellow()
        );
        return Ok(());
    }

    // Execute the script
    println!();
    println!("{}", "Executing script...".cyan().bold());
    println!();

    let start = Instant::now();
    let result = execute_script(&script, &args.args)?;
    let duration = start.elapsed();

    // Record execution
    let ctx = context::detect_context()?;
    let execution = ExecutionRecord {
        id: uuid::Uuid::new_v4().to_string(),
        script_id: script.id.clone(),
        script_version: script.version.clone(),
        executed_by: config.username.clone().unwrap_or_else(|| default_author()),
        executed_at: chrono::Utc::now(),
        exit_code: result.exit_code,
        duration_ms: duration.as_millis() as u64,
        output: Some(result.output),
        error: result.error,
        context: ctx,
    };

    save_execution_record(&execution)?;

    // update script metadata
    script.metadata.use_count += 1;
    script.metadata.last_run = Some(execution.executed_at);
    script.metadata.last_run_by = Some(execution.executed_by.clone());

    if result.exit_code == 0 {
        script.metadata.success_count += 1;
    } else {
        script.metadata.failure_count += 1;
    }

    // update average runtime
    if let Some(avg) = script.metadata.avg_runtime_ms {
        script.metadata.avg_runtime_ms = Some(
            (avg * (script.metadata.use_count - 1) + duration.as_millis() as u64)
                / script.metadata.use_count,
        );
    } else {
        script.metadata.avg_runtime_ms = Some(duration.as_millis() as u64);
    }

    // Save updated script metadata back to vault
    update_script_metadata(&script)?;

    // Show result
    println!();
    if result.exit_code == 0 {
        println!(
            "{} Script completed successfully in {:.2}s",
            "✓".green().bold(),
            duration.as_secs_f64()
        );
    } else {
        println!(
            "{} Script failed with exit code {} in {:.2}s",
            "✗".red().bold(),
            result.exit_code,
            duration.as_secs_f64()
        );
    }

    Ok(())
}

fn show_script_preview(script: &Script, _args: &RunArgs) -> Result<()> {
    println!("╭{}╮", "─".repeat(60));
    println!(
        "│ {} {} │",
        script.name.yellow().bold(),
        script.version.dimmed()
    );
    println!("├{}┤", "─".repeat(60));
    println!("│ │");

    if !script.tags.is_empty() {
        println!("│ Tags: {} │", script.tags.join(", ").cyan());
    }

    if let Some(desc) = &script.description {
        println!("│ Description: {} │", desc);
    }

    println!("│ │");
    println!("│ This script will: │");

    if let Some(dir) = &script.context.directory {
        println!("│  • Execute in: {} │", dir.yellow());
    }

    println!("│  • Language: {} │", script.language.to_string().green());

    let success_rate = script.success_rate();
    if script.metadata.use_count > 0 {
        let _ate_color = if success_rate > 90.0 {
            "green"
        } else if success_rate > 70.0 {
            "yellow"
        } else {
            "red"
        };
        println!(
            "│  • Success Rate: {:.1}% ({}/{} runs) │",
            success_rate, script.metadata.success_count, script.metadata.use_count
        );
    }

    println!("│ │");
    println!("╰{}╯", "─".repeat(60));

    Ok(())
}

struct ExecutionResult {
    exit_code: i32,
    output: String,
    error: Option<String>,
}

fn execute_script(script: &Script, args: &[String]) -> Result<ExecutionResult> {
    // Create a ScriptVault-specific temp directory
    let temp_dir = std::env::temp_dir().join("scriptvault");
    fs::create_dir_all(&temp_dir)?;

    let script_path = temp_dir.join(format!(
        "{}.{}",
        script.name,
        get_extension(&script.language)
    ));

    fs::write(&script_path, &script.content)?;

    // Make it executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&script_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms)?;
    }

    // Get interpreter and args
    let (interpreter, mut interpreter_args) = get_interpreter_command(&script.language);

    // Execute
    let output = Command::new(interpreter)
        .args(&interpreter_args)
        .arg(&script_path)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;

    // Clean up
    fs::remove_file(script_path).ok();

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    // Print output
    if !stdout.is_empty() {
        print!("{}", stdout);
    }
    if !stderr.is_empty() {
        eprint!("{}", stderr);
    }

    Ok(ExecutionResult {
        exit_code: output.status.code().unwrap_or(1),
        output: stdout,
        error: if stderr.is_empty() {
            None
        } else {
            Some(stderr)
        },
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
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();

    // Load scripts to map IDs to names
    let scripts = load_scripts_local()?;
    let script_map: HashMap<String, String> = scripts
        .iter()
        .map(|s| (s.id.clone(), s.name.clone()))
        .collect();

    // Filter records
    let filtered: Vec<&ExecutionRecord> = records
        .iter()
        .filter(|r| {
            // Filter by script name if provided
            if let Some(ref script_name) = args.script {
                // Try to find the script ID from the name
                let script_id = scripts
                    .iter()
                    .find(|s| s.name == *script_name)
                    .map(|s| s.id.as_str());

                if let Some(id) = script_id {
                    if r.script_id != id {
                        return false;
                    }
                } else {
                    return false;
                }
            }

            // Filter by failed status
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

    // Table header
    println!(
        "{:<20} {:<20} {:<15} {:<10} {:<10}",
        "TIME".bold(),
        "SCRIPT".bold(),
        "USER".bold(),
        "EXIT CODE".bold(),
        "DURATION".bold()
    );
    println!("{}", "─".repeat(80).dimmed());

    let limit = if args.recent {
        10
    } else {
        DEFAULT_HISTORY_LIMIT
    };

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

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(history_path)?;

    let json = serde_json::to_string(record)?;
    writeln!(file, "{}", json)?;

    Ok(())
}
