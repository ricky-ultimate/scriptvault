mod auth;
mod cli;
mod config;
mod constants;
mod context;
mod execution;
mod script;
mod storage;
mod sync;
mod utils;
mod vault;

use anyhow::Result;
use clap::Parser;
use cli::{AuthAction, Cli, Command, TeamAction};
use colored::*;

fn main() {
    if let Err(e) = run() {
        eprintln!("{} {}", "Error:".red().bold(), e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Auth(auth_cmd) => match auth_cmd.action {
            AuthAction::Login(args) => auth::login(args)?,
            AuthAction::Logout => auth::logout()?,
            AuthAction::Status => auth::status()?,
        },
        Command::Save(args) => vault::save_script(args)?,
        Command::Find(args) => vault::find_scripts(args)?,
        Command::List(args) => vault::list_scripts(args)?,
        Command::Info(args) => vault::show_info(args)?,
        Command::Run(args) => execution::run_script(args)?,
        Command::History(args) => execution::show_history(args)?,
        Command::Stats(args) => vault::show_stats(args)?,
        Command::Versions(args) => vault::show_versions(args)?,
        Command::Diff(args) => vault::diff_versions(args)?,
        Command::Checkout(args) => vault::checkout_version(args)?,
        Command::Share(args) => vault::share_script(args)?,
        Command::Team(team_cmd) => match team_cmd.action {
            TeamAction::Ls => vault::list_team_members()?,
            TeamAction::Scripts => vault::list_team_scripts()?,
            TeamAction::Permissions => vault::show_permissions()?,
        },
        Command::Context => context::show_context()?,
        Command::Recommend => vault::recommend_scripts()?,
        Command::Export(args) => vault::export_scripts(args)?,
        Command::Sync => sync::sync_vault()?,
        Command::Storage(storage_cmd) => {
            storage::commands::handle_storage_command(storage_cmd.action)?
        }
        Command::Doctor => utils::run_doctor()?,
        Command::Status => utils::check_status()?,
    }

    Ok(())
}
