use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "sv")]
#[command(author = "リッキー")]
#[command(version)]
#[command(about = "ScriptVault - Your terminal script vault", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Auth(AuthCommand),
    Save(SaveArgs),
    Update(UpdateArgs),
    Find(FindArgs),
    Search(FindArgs),
    List(ListArgs),
    Info(InfoArgs),
    Run(RunArgs),
    Delete(DeleteArgs),
    Cat(CatArgs),
    Edit(EditArgs),
    Rename(RenameArgs),
    Copy(CopyArgs),
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
    Sync(SyncCommand),
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
    Register(RegisterArgs),
}

#[derive(Args, Debug)]
pub struct RegisterArgs {
    #[arg(long, value_name = "USERNAME")]
    pub username: Option<String>,
}

#[derive(Args, Debug)]
pub struct LoginArgs {
    #[arg(long, value_name = "NAME", help = "Set your local username")]
    pub token: Option<String>,
}

#[derive(Args, Debug)]
pub struct SaveArgs {
    #[arg(value_name = "FILE")]
    pub file: String,

    #[arg(long, value_name = "NAME", help = "Override the vault name (defaults to filename stem)")]
    pub name: Option<String>,

    #[arg(long, value_name = "TAGS")]
    pub tags: Option<String>,

    #[arg(long, value_name = "DESC")]
    pub description: Option<String>,

    #[arg(long, help = "Skip interactive prompts")]
    pub yes: bool,
}

#[derive(Args, Debug)]
pub struct UpdateArgs {
    #[arg(value_name = "FILE", help = "Path to the updated script file")]
    pub file: String,

    #[arg(long, value_name = "NAME", help = "Override vault name lookup (defaults to filename stem)")]
    pub name: Option<String>,
}

#[derive(Args, Debug)]
pub struct FindArgs {
    #[arg(value_name = "QUERY")]
    pub query: Option<String>,

    #[arg(long, help = "Show only scripts relevant to the current directory")]
    pub here: bool,

    #[arg(long, value_name = "TAG")]
    pub tag: Option<String>,

    #[arg(long, value_name = "LANG")]
    pub language: Option<String>,

    #[arg(long)]
    pub team: bool,

    #[arg(long, value_name = "REPO")]
    pub git_repo: Option<String>,

    #[arg(long, help = "Sort by most recently run")]
    pub recent: bool,
}

#[derive(Args, Debug)]
pub struct ListArgs {
    #[arg(long)]
    pub mine: bool,

    #[arg(long)]
    pub team: bool,

    #[arg(long)]
    pub all: bool,

    #[arg(long, help = "Sort by most recently run")]
    pub recent: bool,
}

#[derive(Args, Debug)]
pub struct InfoArgs {
    pub name: String,
}

#[derive(Args, Debug)]
pub struct RunArgs {
    #[arg(value_name = "SCRIPT")]
    pub script: String,

    #[arg(
        value_name = "ARGS",
        trailing_var_arg = true,
        allow_hyphen_values = true,
        help = "Arguments passed through to the script"
    )]
    pub args: Vec<String>,

    #[arg(long, help = "Show what would happen without executing")]
    pub dry_run: bool,

    #[arg(long, help = "Isolated sandbox environment (not yet available)")]
    pub sandbox: bool,

    #[arg(long, help = "Require confirmation before running")]
    pub confirm: bool,

    #[arg(long, short, help = "Show interpreter, path, and arguments before execution")]
    pub verbose: bool,

    #[arg(long, help = "Non-interactive mode, no prompts")]
    pub ci: bool,

    #[arg(long, help = "Pull latest version before running (requires cloud sync)")]
    pub update: bool,
}

#[derive(Args, Debug)]
pub struct DeleteArgs {
    pub name: String,

    #[arg(long, help = "Skip confirmation prompt")]
    pub yes: bool,
}

#[derive(Args, Debug)]
pub struct CatArgs {
    pub name: String,
}

#[derive(Args, Debug)]
pub struct EditArgs {
    pub name: String,
}

#[derive(Args, Debug)]
pub struct RenameArgs {
    pub old_name: String,
    pub new_name: String,
}

#[derive(Args, Debug)]
pub struct CopyArgs {
    pub source: String,
    pub dest: String,
}

#[derive(Args, Debug)]
pub struct HistoryArgs {
    #[arg(value_name = "SCRIPT")]
    pub script: Option<String>,

    #[arg(long, help = "Show only failed runs")]
    pub failed: bool,

    #[arg(long, help = "Show only the 10 most recent runs")]
    pub recent: bool,

    #[arg(long, help = "Show team execution history (requires cloud sync)")]
    pub team: bool,
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
pub struct SyncCommand {
    #[command(subcommand)]
    pub action: Option<SyncAction>,
}

#[derive(Subcommand, Debug)]
pub enum SyncAction {
    Status,
    Push,
    Pull,
    Resolve(ResolveArgs),
}

#[derive(Args, Debug)]
pub struct ResolveArgs {
    pub script: String,

    #[arg(long, conflicts_with = "take_remote")]
    pub take_local: bool,

    #[arg(long, conflicts_with = "take_local")]
    pub take_remote: bool,
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
