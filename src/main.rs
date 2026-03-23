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

use anyhow::{anyhow, Result};
use clap::Parser;
use cli::{AuthAction, Cli, Command, SyncAction, TeamAction};
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
        Command::Update(args) => vault::update_script_from_file(args)?,
        Command::Find(args) | Command::Search(args) => vault::find_scripts(args)?,
        Command::List(args) => vault::list_scripts(args)?,
        Command::Info(args) => vault::show_info(args)?,
        Command::Run(args) => execution::run_script(args)?,
        Command::Delete(args) => vault::delete_script(args)?,
        Command::Cat(args) => vault::cat_script(args)?,
        Command::Edit(args) => vault::edit_script(args)?,
        Command::Rename(args) => vault::rename_script(args)?,
        Command::Copy(args) => vault::copy_script(args)?,
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
        Command::Sync(sync_cmd) => match sync_cmd.action {
            None | Some(SyncAction::Push) => sync::push_all()?,
            Some(SyncAction::Pull) => sync::pull_all()?,
            Some(SyncAction::Status) => sync::show_status()?,
            Some(SyncAction::Resolve(args)) => {
                let resolution = if args.take_local {
                    sync::ConflictResolution::TakeLocal
                } else if args.take_remote {
                    sync::ConflictResolution::TakeRemote
                } else {
                    return Err(anyhow!(
                        "Specify --take-local or --take-remote to resolve the conflict"
                    ));
                };
                sync::resolve_conflict(&args.script, resolution)?;
            }
        },
        Command::Storage(storage_cmd) => {
            storage::commands::handle_storage_command(storage_cmd.action)?
        }
        Command::Doctor => utils::run_doctor()?,
        Command::Status => utils::check_status()?,
    }

    Ok(())
}
