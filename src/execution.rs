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
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Instant;

const SAFE_ENV_VARS: &[&str] = &[
    "PATH", "TERM", "LANG", "LC_ALL", "LC_CTYPE", "HOME", "USER", "LOGNAME", "SHELL", "TZ",
    "TMPDIR", "TEMP", "TMP",
];

fn build_safe_env() -> HashMap<String, String> {
    let mut env = HashMap::new();
    for key in SAFE_ENV_VARS {
        if let Ok(val) = std::env::var(key) {
            env.insert(key.to_string(), val);
        }
    }
    env
}

fn check_interpreter_available(language: &ScriptLanguage) -> Result<()> {
    let (interpreter, _) = get_interpreter_command(language);
    which::which(interpreter).map_err(|_| {
        anyhow!(
            "Required interpreter '{}' not found in PATH. Install it before running this script.",
            interpreter
        )
    })?;
    Ok(())
}

pub fn run_script(args: RunArgs) -> Result<()> {
    let config = Config::load()?;
    let ci_mode = args.ci || std::env::var(ENV_SCRIPTVAULT_CI).is_ok();

    if args.update {
        if !config.is_authenticated() {
            return Err(anyhow!(
                "sv run --update requires cloud sync. Run 'sv auth login --token <API_KEY>' first."
            ));
        }
        pull_script_update(&args.script, &config)?;
    }

    let scripts = load_scripts_local()?;
    let mut script = scripts
        .iter()
        .find(|s| s.name == args.script)
        .ok_or_else(|| anyhow!("Script not found: {}", args.script))?
        .clone();

    if let Some(ref target) = args.ssh {
        return run_script_remote(
            &script,
            &args.args,
            target,
            args.ssh_port,
            args.ssh_identity.as_deref(),
            args.ssh_agent,
            args.dry_run,
            args.verbose,
        );
    }

    check_interpreter_available(&script.language)?;

    if !script.is_safe() {
        println!(
            "{}",
            "Warning: This script contains potentially dangerous commands."
                .red()
                .bold()
        );
        if !ci_mode && !args.dry_run {
            let proceed = Confirm::new()
                .with_prompt("Run this script?")
                .default(false)
                .interact()?;
            if !proceed {
                println!("Execution cancelled.");
                return Ok(());
            }
        }
    }

    show_script_preview(&script, &args.args)?;

    let needs_confirm = args.confirm || (config.confirm_before_run && !ci_mode);
    if needs_confirm && !args.dry_run {
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
        println!("{}", "Dry run complete. Script was not executed.".yellow());
        return Ok(());
    }

    println!();
    println!("{}", "Executing...".cyan().bold());
    println!();

    let start = Instant::now();
    let result = if args.sandbox {
        println!(
            "{}",
            "Note: --sandbox uses a private temp directory and clears environment variables. \
         It does not provide kernel-level sandboxing, syscall filtering, or filesystem isolation."
                .yellow()
        );
        execute_script_isolated(&script, &args.args, args.verbose)?
    } else {
        execute_script_safe_env(&script, &args.args, args.verbose)?
    };
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

    let prev_recorded = script.metadata.success_count + script.metadata.failure_count;
    script.metadata.use_count += 1;

    if exit_code == 0 {
        script.metadata.success_count += 1;
    } else {
        script.metadata.failure_count += 1;
    }

    let new_recorded = script.metadata.success_count + script.metadata.failure_count;

    script.metadata.avg_runtime_ms = Some(match script.metadata.avg_runtime_ms {
        Some(avg) => (avg * prev_recorded + duration.as_millis() as u64) / new_recorded,
        None => duration.as_millis() as u64,
    });

    script.metadata.last_run = Some(execution.executed_at);
    script.metadata.last_run_by = Some(execution.executed_by.clone());

    update_script_metadata(&script)?;

    println!();
    if exit_code == 0 {
        println!("Completed in {:.2}s", duration.as_secs_f64());
    } else {
        println!(
            "Failed with exit code {} in {:.2}s",
            exit_code,
            duration.as_secs_f64()
        );
    }

    Ok(())
}

fn run_script_remote(
    script: &Script,
    run_args: &[String],
    target: &str,
    port: u16,
    identity: Option<&str>,
    forward_agent: bool,
    dry_run: bool,
    verbose: bool,
) -> Result<()> {
    which::which("ssh")
        .map_err(|_| anyhow!("'ssh' not found in PATH. Install OpenSSH to use --ssh."))?;

    let remote_path = format!(
        "/tmp/sv_{}.{}",
        uuid::Uuid::new_v4(),
        script.language.extension()
    );

    if dry_run {
        println!("{}", "Dry run — remote execution plan:".yellow().bold());
        println!("  Target:        {}", target.cyan());
        println!("  Port:          {}", port);
        println!("  Remote path:   {}", remote_path.dimmed());
        println!(
            "  Script:        {} {}",
            script.name.yellow(),
            script.version.dimmed()
        );
        if forward_agent {
            println!("  Agent forward: yes");
        }
        if !run_args.is_empty() {
            println!("  Arguments:     {}", run_args.join(" ").cyan());
        }
        println!();
        println!("{}", "Dry run complete. Script was not executed.".yellow());
        return Ok(());
    }

    let mut base_ssh_args: Vec<String> = vec![
        "-p".into(),
        port.to_string(),
        "-o".into(),
        "StrictHostKeyChecking=accept-new".into(),
        "-o".into(),
        "BatchMode=yes".into(),
    ];

    if forward_agent {
        base_ssh_args.push("-A".into());
    }

    if let Some(key) = identity {
        base_ssh_args.push("-i".into());
        base_ssh_args.push(key.to_string());
    }

    if verbose {
        println!("  Target:      {}", target);
        println!("  Remote path: {}", remote_path);
        if forward_agent {
            println!("  Agent forward: enabled");
        }
        println!();
        println!("  {}:", "Content".dimmed());
        for line in script.content.lines() {
            println!("    {}", line.dimmed());
        }
        println!();
    }

    let mut copy_cmd = std::process::Command::new("ssh");
    copy_cmd.args(&base_ssh_args);
    copy_cmd
        .arg(target)
        .arg(format!(
            "cat > {} && chmod 700 {}",
            remote_path, remote_path
        ))
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());

    let mut copy_child = copy_cmd
        .spawn()
        .map_err(|e| anyhow!("Failed to open SSH connection to {}: {}", target, e))?;

    if let Some(mut stdin) = copy_child.stdin.take() {
        stdin
            .write_all(script.content.as_bytes())
            .map_err(|e| anyhow!("Failed to write script content over SSH: {}", e))?;
    }

    let copy_status = copy_child.wait()?;
    if !copy_status.success() {
        return Err(anyhow!("Failed to copy script to remote host {}", target));
    }

    let mut exec_cmd = std::process::Command::new("ssh");
    exec_cmd.args(&base_ssh_args);

    let script_call = if run_args.is_empty() {
        remote_path.clone()
    } else {
        format!("{} {}", remote_path, run_args.join(" "))
    };

    let cleanup = format!(
        "{}; _exit=$?; rm -f {}; exit $_exit",
        script_call, remote_path
    );

    exec_cmd
        .arg(target)
        .arg(&cleanup)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let status = exec_cmd
        .spawn()
        .map_err(|e| anyhow!("Failed to execute script on remote host {}: {}", target, e))?
        .wait()?;

    let exit_code = status.code().unwrap_or(1);
    println!();
    if exit_code == 0 {
        println!("Remote execution completed successfully.");
    } else {
        println!("Remote execution failed with exit code {}.", exit_code);
    }

    Ok(())
}
fn pull_script_update(script_name: &str, config: &Config) -> Result<()> {
    use crate::storage::StorageBackend;
    use crate::sync::remote::{HttpRemoteBackend, RemoteBackend};

    let token = config
        .auth_token
        .clone()
        .ok_or_else(|| anyhow!("No auth token found"))?;

    let remote = HttpRemoteBackend::new(config.api_endpoint.clone(), token);
    let local = config.get_storage_backend()?;

    let remote_metas = remote.list_scripts()?;
    let meta = remote_metas
        .iter()
        .find(|m| m.name == script_name)
        .ok_or_else(|| anyhow!("Script '{}' not found on remote", script_name))?;

    let local_script = local.load_script_by_name(script_name);
    let needs_update = match &local_script {
        Ok(s) => s.metadata.hash != meta.hash,
        Err(_) => true,
    };

    if !needs_update {
        return Ok(());
    }

    println!("Pulling latest version of '{}'...", script_name.yellow());

    let mut remote_script = remote.fetch_script(&meta.id)?;
    let now = chrono::Utc::now();
    let hash = remote_script.metadata.hash.clone();
    let version = remote_script.version.clone();

    remote_script.sync_state = crate::script::SyncState {
        status: crate::script::SyncStatus::Synced,
        last_synced_at: Some(now),
        remote_version: Some(version),
        conflict_base_hash: Some(hash),
    };

    if local.script_exists(&remote_script.id)? {
        local.update_script(&remote_script)?;
    } else {
        local.save_script(&remote_script)?;
    }

    println!(
        "Updated '{}' to {}",
        script_name.yellow(),
        remote_script.version.green()
    );

    Ok(())
}

fn show_script_preview(script: &Script, run_args: &[String]) -> Result<()> {
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

    if !run_args.is_empty() {
        println!("│  Arguments: {}", run_args.join(" ").cyan());
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

fn write_temp_script(script: &Script) -> Result<std::path::PathBuf> {
    let temp_dir = std::env::temp_dir().join("scriptvault");
    fs::create_dir_all(&temp_dir)?;

    let temp_filename = format!("{}.{}", uuid::Uuid::new_v4(), script.language.extension());
    let script_path = temp_dir.join(temp_filename);

    fs::write(&script_path, &script.content)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&script_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms)?;
    }

    Ok(script_path)
}

fn spawn_and_collect(
    interpreter: &str,
    interpreter_args: &[&str],
    script_path: &std::path::Path,
    args: &[String],
    env: Option<&HashMap<String, String>>,
    verbose: bool,
) -> Result<ExecutionResult> {
    if verbose {
        println!("  Interpreter: {}", interpreter);
        println!("  Script path: {}", script_path.display());
        if !args.is_empty() {
            println!("  Arguments:   {}", args.join(" "));
        }
        println!();
    }

    let mut cmd = Command::new(interpreter);
    cmd.args(interpreter_args)
        .arg(script_path)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(vars) = env {
        cmd.env_clear();
        for (k, v) in vars {
            cmd.env(k, v);
        }
    }

    let mut child = cmd.spawn()?;

    let stdout_pipe = child.stdout.take().expect("stdout was piped");
    let stderr_pipe = child.stderr.take().expect("stderr was piped");

    let stdout_handle = std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout_pipe);
        let mut captured = String::new();
        let mut line = String::new();
        while reader.read_line(&mut line).unwrap_or(0) > 0 {
            print!("{}", line);
            captured.push_str(&line);
            line.clear();
        }
        captured
    });

    let stderr_handle = std::thread::spawn(move || {
        let mut reader = BufReader::new(stderr_pipe);
        let mut captured = String::new();
        let mut line = String::new();
        while reader.read_line(&mut line).unwrap_or(0) > 0 {
            eprint!("{}", line);
            captured.push_str(&line);
            line.clear();
        }
        captured
    });

    let status = child.wait()?;
    let stdout_str = stdout_handle.join().unwrap_or_default();
    let stderr_str = stderr_handle.join().unwrap_or_default();

    Ok(ExecutionResult {
        exit_code: status.code().unwrap_or(1),
        output: if stdout_str.is_empty() {
            None
        } else {
            Some(stdout_str)
        },
        error: if stderr_str.is_empty() {
            None
        } else {
            Some(stderr_str)
        },
    })
}

fn execute_script_safe_env(
    script: &Script,
    args: &[String],
    verbose: bool,
) -> Result<ExecutionResult> {
    let script_path = write_temp_script(script)?;
    let (interpreter, interpreter_args) = get_interpreter_command(&script.language);
    let safe_env = build_safe_env();

    if verbose {
        println!();
        println!("  {}:", "Content".dimmed());
        for line in script.content.lines() {
            println!("    {}", line.dimmed());
        }
        println!();
    }

    let result = spawn_and_collect(
        interpreter,
        &interpreter_args,
        &script_path,
        args,
        Some(&safe_env),
        verbose,
    );

    if let Err(e) = fs::remove_file(&script_path) {
        eprintln!("Warning: failed to remove temporary file: {}", e);
    }

    result
}

fn execute_script_isolated(
    script: &Script,
    args: &[String],
    verbose: bool,
) -> Result<ExecutionResult> {
    let sandbox_dir = std::env::temp_dir()
        .join("scriptvault")
        .join("isolated")
        .join(uuid::Uuid::new_v4().to_string());

    fs::create_dir_all(&sandbox_dir)?;

    let script_filename = format!("script.{}", script.language.extension());
    let script_path = sandbox_dir.join(&script_filename);

    fs::write(&script_path, &script.content)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&script_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms)?;
    }

    let mut env: HashMap<String, String> = HashMap::new();
    env.insert("HOME".into(), sandbox_dir.to_string_lossy().into_owned());
    env.insert("TMPDIR".into(), sandbox_dir.to_string_lossy().into_owned());
    env.insert("PATH".into(), std::env::var("PATH").unwrap_or_default());
    env.insert("ISOLATED".into(), "1".into());

    if let Ok(term) = std::env::var("TERM") {
        env.insert("TERM".into(), term);
    }
    if let Ok(lang) = std::env::var("LANG") {
        env.insert("LANG".into(), lang);
    }

    if verbose {
        println!("  Isolated directory: {}", sandbox_dir.display());
        println!();
        println!("  {}:", "Content".dimmed());
        for line in script.content.lines() {
            println!("    {}", line.dimmed());
        }
        println!();
    }

    let (interpreter, interpreter_args) = get_interpreter_command(&script.language);
    let result = spawn_and_collect(
        interpreter,
        &interpreter_args,
        &script_path,
        args,
        Some(&env),
        verbose,
    );

    if let Err(e) = fs::remove_dir_all(&sandbox_dir) {
        eprintln!("Warning: failed to remove isolated directory: {}", e);
    }

    result
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

pub fn show_history(args: HistoryArgs) -> Result<()> {
    if args.team {
        return Err(anyhow!("Team history is not yet available."));
    }

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

    if let Some(ref script_name) = args.script {
        let found = scripts.iter().any(|s| s.name == *script_name);
        if !found {
            println!(
                "Note: '{}' is not in your vault (it may have been deleted).",
                script_name
            );
            println!("History for deleted scripts cannot be filtered by name.");
            println!("Run 'sv history' to see all records including those marked [deleted].");
            return Ok(());
        }
    }

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
        "{:<20} {:<22} {:<15} {:<10} {:<10}",
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

        let script_display = match script_map.get(&record.script_id) {
            Some(name) => name.yellow().to_string(),
            None => "[deleted]".dimmed().to_string(),
        };

        let exit_status = if record.exit_code == 0 {
            record.exit_code.to_string().green()
        } else {
            record.exit_code.to_string().red()
        };

        let duration = format!("{:.2}s", record.duration_ms as f64 / 1000.0);

        println!(
            "{:<20} {:<22} {:<15} {:<10} {:<10}",
            time.to_string().dimmed(),
            script_display,
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
