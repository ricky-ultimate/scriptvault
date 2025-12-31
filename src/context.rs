use crate::script::ScriptContext;
use anyhow::Result;
use colored::*;
use git2::Repository;
use std::collections::HashMap;
use std::env;

pub fn detect_context() -> Result<ScriptContext> {
    let directory = env::current_dir()
        .ok()
        .map(|p| p.to_string_lossy().to_string());

    let (git_repo, git_branch) = detect_git_context();

    let mut environment = HashMap::new();

    // Capture relevant environment variables
    if let Ok(shell) = env::var("SHELL") {
        environment.insert("SHELL".to_string(), shell);
    }
    if let Ok(user) = env::var("USER") {
        environment.insert("USER".to_string(), user);
    }
    if let Ok(os) = env::var("OS") {
        environment.insert("OS".to_string(), os);
    }

    Ok(ScriptContext {
        directory,
        git_repo,
        git_branch,
        environment,
    })
}

fn detect_git_context() -> (Option<String>, Option<String>) {
    let current_dir = match env::current_dir() {
        Ok(dir) => dir,
        Err(_) => return (None, None),
    };

    let repo = match Repository::discover(current_dir) {
        Ok(r) => r,
        Err(_) => return (None, None),
    };

    // Get remote URL
    let git_repo = repo
        .find_remote("origin")
        .ok()
        .and_then(|remote| remote.url().map(|s| s.to_string()))
        .map(|url| normalize_git_url(&url));

    // Get current branch
    let git_branch = repo
        .head()
        .ok()
        .and_then(|head| head.shorthand().map(|s| s.to_string()));

    (git_repo, git_branch)
}

pub fn normalize_git_url(url: &str) -> String {
    // Convert git@github.com:user/repo.git to github.com/user/repo
    let url = url
        .trim_start_matches("git@")
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .replace(':', "/")
        .trim_end_matches(".git")
        .to_string();

    url
}

pub fn show_context() -> Result<()> {
    let ctx = detect_context()?;

    println!("{}", "Current Context".bold().cyan());
    println!();

    if let Some(dir) = ctx.directory {
        println!("  {}: {}", "Directory".bold(), dir.yellow());
    }

    if let Some(repo) = ctx.git_repo {
        println!("  {}: {}", "Git Repo".bold(), repo.green());
        if let Some(branch) = ctx.git_branch {
            println!("  {}: {}", "Branch".bold(), branch.blue());
        }
    } else {
        println!(
            "  {}: {}",
            "Git Repo".bold(),
            "Not in a git repository".dimmed()
        );
    }

    if !ctx.environment.is_empty() {
        println!();
        println!("  {}:", "Environment".bold());
        for (key, value) in &ctx.environment {
            println!("    {}: {}", key, value.dimmed());
        }
    }

    Ok(())
}

pub fn contexts_match(ctx1: &ScriptContext, ctx2: &ScriptContext) -> bool {
    // Check if contexts are similar enough

    // Exact git repo match is strong
    if ctx1.git_repo.is_some() && ctx1.git_repo == ctx2.git_repo {
        return true;
    }

    // Same directory is also a match
    if ctx1.directory.is_some() && ctx1.directory == ctx2.directory {
        return true;
    }

    // Check if one directory is a parent of the other
    if let (Some(dir1), Some(dir2)) = (&ctx1.directory, &ctx2.directory) {
        if dir1.starts_with(dir2) || dir2.starts_with(dir1) {
            return true;
        }
    }

    false
}
