use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "sv")]
#[command(author = "リッキー")]
#[command(version = "0.1.0")]
#[command(about = "ScriptVault - Your terminal script time-machine", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Authenticate with ScriptVault
    Auth(AuthCommand),

    /// Save a script to your vault
    Save(SaveArgs),

    /// Find and search scripts
    Find(FindArgs),

    /// List your scripts
    List(ListArgs),

    /// Get detailed information about a script
    Info(InfoArgs),

    /// Run a script from your vault
    Run(RunArgs),

    /// View script execution history
    History(HistoryArgs),

    /// View script statistics
    Stats(StatsArgs),

    /// Manage script versions
    Versions(VersionArgs),

    /// Compare script versions
    Diff(DiffArgs),

    /// Check out a specific version
    Checkout(CheckoutArgs),

    /// Share a script with team or community
    Share(ShareArgs),

    /// Manage team
    Team(TeamCommand),

    /// Show current context
    Context,

    /// Get script recommendations
    Recommend,

    /// Export scripts
    Export(ExportArgs),

    /// Sync with cloud
    Sync,

    /// Check CLI health
    Doctor,

    /// Check service status
    Status,
}

#[derive(Args, Debug)]
pub struct AuthCommand {
    #[command(subcommand)]
    pub action: AuthAction,
}

#[derive(Subcommand, Debug)]
pub enum AuthAction {
    /// Login to ScriptVault
    Login(LoginArgs),
    /// Logout from ScriptVault
    Logout,
    /// Check authentication status
    Status,
}

#[derive(Args, Debug)]
pub struct LoginArgs {
    /// Use API token instead of OAuth
    #[arg(long)]
    pub token: Option<String>,
}

#[derive(Args, Debug)]
pub struct SaveArgs {
    /// Path to the script file
    pub file: String,

    /// Tags for the script (space-separated)
    #[arg(long)]
    pub tags: Option<String>,

    /// Description of the script
    #[arg(long)]
    pub description: Option<String>,

    /// Save with git context
    #[arg(long)]
    pub git: bool,

    /// Skip interactive prompts
    #[arg(long)]
    pub yes: bool,
}

#[derive(Args, Debug)]
pub struct FindArgs {
    /// Search query
    pub query: Option<String>,

    /// Find scripts relevant to current directory
    #[arg(long)]
    pub here: bool,

    /// Filter by tag
    #[arg(long)]
    pub tag: Option<String>,

    /// Filter by language
    #[arg(long)]
    pub language: Option<String>,

    /// Show team scripts only
    #[arg(long)]
    pub team: bool,

    /// Show all scripts (including public)
    #[arg(long)]
    pub all: bool,

    /// Filter by git repository
    #[arg(long)]
    pub git_repo: Option<String>,
}

#[derive(Args, Debug)]
pub struct ListArgs {
    /// Show only your scripts
    #[arg(long)]
    pub mine: bool,

    /// Show team scripts
    #[arg(long)]
    pub team: bool,

    /// Show all scripts
    #[arg(long)]
    pub all: bool,
}

#[derive(Args, Debug)]
pub struct InfoArgs {
    /// Script name
    pub name: String,
}

#[derive(Args, Debug)]
pub struct RunArgs {
    /// Script name or path
    pub script: String,

    /// Arguments to pass to the script
    pub args: Vec<String>,

    /// Show what would happen without executing
    #[arg(long)]
    pub dry_run: bool,

    /// Run in isolated sandbox environment
    #[arg(long)]
    pub sandbox: bool,

    /// Require step-by-step confirmation
    #[arg(long)]
    pub confirm: bool,

    /// Update to latest version before running
    #[arg(long)]
    pub update: bool,

    /// Verbose output
    #[arg(long, short)]
    pub verbose: bool,

    /// CI mode (no interactive prompts)
    #[arg(long)]
    pub ci: bool,

    /// Check if only failed runs
    #[arg(long)]
    pub failed: bool,

    /// Check permissions before running
    #[arg(long)]
    pub check_permissions: bool,
}

#[derive(Args, Debug)]
pub struct HistoryArgs {
    /// Script name (optional, shows all if omitted)
    pub script: Option<String>,

    /// Show only failed runs
    #[arg(long)]
    pub failed: bool,

    /// Show recent runs
    #[arg(long)]
    pub recent: bool,

    /// Show team history
    #[arg(long)]
    pub team: bool,
}

#[derive(Args, Debug)]
pub struct StatsArgs {
    /// Script name
    pub name: String,
}

#[derive(Args, Debug)]
pub struct VersionArgs {
    /// Script name
    pub name: String,
}

#[derive(Args, Debug)]
pub struct DiffArgs {
    /// Script name
    pub name: String,

    /// First version to compare
    pub version1: String,

    /// Second version to compare
    pub version2: String,
}

#[derive(Args, Debug)]
pub struct CheckoutArgs {
    /// Script name with version (e.g., script-name@v1.0)
    pub script_version: String,
}

#[derive(Args, Debug)]
pub struct ShareArgs {
    /// Script name
    pub name: String,

    /// Share with team
    #[arg(long)]
    pub team: bool,

    /// Share publicly
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
    /// List team members
    Ls,
    /// List team scripts
    Scripts,
    /// View team permissions
    Permissions,
}

#[derive(Args, Debug)]
pub struct ExportArgs {
    /// Export format (markdown, cheatsheet, json)
    #[arg(long, default_value = "markdown")]
    pub format: String,

    /// Output file (stdout if not specified)
    #[arg(long, short)]
    pub output: Option<String>,
}
