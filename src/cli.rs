use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "sv")]
#[command(author = "リッキー")]
#[command(version = "0.1.0")]
#[command(about = "ScriptVault - Your terminal script vault", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Auth(AuthCommand),
    Save(SaveArgs),
    Find(FindArgs),
    List(ListArgs),
    Info(InfoArgs),
    Run(RunArgs),
    History(HistoryArgs),
    Stats(StatsArgs),
    Versions(VersionArgs),
    Diff(DiffArgs),
    Checkout(CheckoutArgs),
    Share(ShareArgs),
    Team(TeamCommand),
    Context,
    Recommend,
    Export(ExportArgs),
    Sync,
    Storage(StorageCommand),
    Doctor,
    Status,
}

#[derive(Args, Debug)]
pub struct AuthCommand {
    #[command(subcommand)]
    pub action: AuthAction,
}

#[derive(Subcommand, Debug)]
pub enum AuthAction {
    Login(LoginArgs),
    Logout,
    Status,
}

#[derive(Args, Debug)]
pub struct LoginArgs {
    #[arg(long)]
    pub token: Option<String>,
}

#[derive(Args, Debug)]
pub struct SaveArgs {
    pub file: String,

    #[arg(long)]
    pub tags: Option<String>,

    #[arg(long)]
    pub description: Option<String>,

    #[arg(long)]
    pub yes: bool,
}

#[derive(Args, Debug)]
pub struct FindArgs {
    pub query: Option<String>,

    #[arg(long)]
    pub here: bool,

    #[arg(long)]
    pub tag: Option<String>,

    #[arg(long)]
    pub language: Option<String>,

    #[arg(long)]
    pub team: bool,

    #[arg(long)]
    pub git_repo: Option<String>,
}

#[derive(Args, Debug)]
pub struct ListArgs {
    #[arg(long)]
    pub mine: bool,

    #[arg(long)]
    pub team: bool,
}

#[derive(Args, Debug)]
pub struct InfoArgs {
    pub name: String,
}

#[derive(Args, Debug)]
pub struct RunArgs {
    pub script: String,

    pub args: Vec<String>,

    #[arg(long)]
    pub dry_run: bool,

    #[arg(long)]
    pub sandbox: bool,

    #[arg(long)]
    pub confirm: bool,

    #[arg(long)]
    pub verbose: bool,

    #[arg(long)]
    pub ci: bool,
}

#[derive(Args, Debug)]
pub struct HistoryArgs {
    pub script: Option<String>,

    #[arg(long)]
    pub failed: bool,

    #[arg(long)]
    pub recent: bool,
}

#[derive(Args, Debug)]
pub struct StatsArgs {
    pub name: String,
}

#[derive(Args, Debug)]
pub struct VersionArgs {
    pub name: String,
}

#[derive(Args, Debug)]
pub struct DiffArgs {
    pub name: String,
    pub version1: String,
    pub version2: String,
}

#[derive(Args, Debug)]
pub struct CheckoutArgs {
    pub script_version: String,
}

#[derive(Args, Debug)]
pub struct ShareArgs {
    pub name: String,

    #[arg(long)]
    pub team: bool,

    #[arg(long)]
    pub public: bool,
}

#[derive(Args, Debug)]
pub struct TeamCommand {
    #[command(subcommand)]
    pub action: TeamAction,
}

#[derive(Subcommand, Debug)]
pub enum TeamAction {
    Ls,
    Scripts,
    Permissions,
}

#[derive(Args, Debug)]
pub struct ExportArgs {
    #[arg(long, default_value = "markdown")]
    pub format: String,

    #[arg(long, short)]
    pub output: Option<String>,
}

#[derive(Args, Debug)]
pub struct StorageCommand {
    #[command(subcommand)]
    pub action: StorageAction,
}

#[derive(Subcommand, Debug)]
pub enum StorageAction {
    Status,
    Setup,
    Test,
    Info,
}
