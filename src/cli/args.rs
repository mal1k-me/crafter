use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "crafter")]
#[command(about = "Local CodeCrafters CLI - Run challenges offline", long_about = None)]
#[command(subcommand_required = true, arg_required_else_help = true)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Args, Clone, Copy, Default)]
pub(crate) struct OutputFmtQuiet {
    /// Quiet mode (minimal output, exit codes only).
    ///
    /// Can be combined with `--format`; when used together, output remains minimal.
    #[arg(short, long)]
    quiet: bool,

    /// Output format
    #[arg(long, value_enum)]
    format: Option<crafter::output::Format>,
}

#[derive(Args, Clone, Copy, Default)]
pub(crate) struct OutputFmtQuietVerbose {
    /// Enable verbose diagnostics (details/debug output where available)
    #[arg(short, long)]
    verbose: bool,

    /// Quiet mode (minimal output, exit codes only).
    ///
    /// Can be combined with `--format`; when used together, output remains minimal.
    #[arg(short, long, conflicts_with = "verbose")]
    quiet: bool,

    /// Output format
    #[arg(long, value_enum)]
    format: Option<crafter::output::Format>,
}

#[derive(Args, Clone, Copy, Default)]
pub(crate) struct OutputFmtQuietFull {
    /// Quiet mode (minimal output, exit codes only).
    ///
    /// Can be combined with `--format`; when used together, output remains minimal.
    #[arg(short, long)]
    quiet: bool,

    /// Output format
    #[arg(long, value_enum)]
    format: Option<crafter::output::Format>,

    /// Show absolute paths instead of ~-abbreviated paths (where paths are displayed)
    #[arg(long)]
    full_paths: bool,
}

#[derive(Args, Clone, Copy, Default)]
pub(crate) struct OutputFmtQuietRawFull {
    /// Quiet mode (minimal output, exit codes only).
    ///
    /// Can be combined with `--format`; when used together, output remains minimal.
    #[arg(short, long)]
    quiet: bool,

    /// Output format
    #[arg(long, value_enum)]
    format: Option<crafter::output::Format>,

    /// Show file sizes as raw bytes (where size fields are displayed)
    #[arg(long)]
    raw_sizes: bool,

    /// Show absolute paths instead of ~-abbreviated paths (where paths are displayed)
    #[arg(long)]
    full_paths: bool,
}

#[derive(Args, Clone, Copy, Default)]
pub(crate) struct OutputFmtQuietVerboseFull {
    /// Enable verbose diagnostics (details/debug output where available)
    #[arg(short, long)]
    verbose: bool,

    /// Quiet mode (minimal output, exit codes only).
    ///
    /// Can be combined with `--format`; when used together, output remains minimal.
    #[arg(short, long, conflicts_with = "verbose")]
    quiet: bool,

    /// Output format
    #[arg(long, value_enum)]
    format: Option<crafter::output::Format>,

    /// Show absolute paths instead of ~-abbreviated paths (where paths are displayed)
    #[arg(long)]
    full_paths: bool,
}

#[derive(Clone, Copy, Default)]
pub(crate) struct ResolvedOutputArgs {
    pub(crate) verbose: bool,
    pub(crate) quiet: bool,
    pub(crate) format: Option<crafter::output::Format>,
    pub(crate) raw_sizes: bool,
    pub(crate) full_paths: bool,
}

impl ResolvedOutputArgs {
    const fn from_quiet_format(output: &OutputFmtQuiet) -> Self {
        Self {
            verbose: false,
            quiet: output.quiet,
            format: output.format,
            raw_sizes: false,
            full_paths: false,
        }
    }

    const fn from_quiet_full(output: &OutputFmtQuietFull) -> Self {
        Self {
            verbose: false,
            quiet: output.quiet,
            format: output.format,
            raw_sizes: false,
            full_paths: output.full_paths,
        }
    }

    const fn from_quiet_raw_full(output: &OutputFmtQuietRawFull) -> Self {
        Self {
            verbose: false,
            quiet: output.quiet,
            format: output.format,
            raw_sizes: output.raw_sizes,
            full_paths: output.full_paths,
        }
    }

    const fn from_quiet_verbose_full(output: &OutputFmtQuietVerboseFull) -> Self {
        Self {
            verbose: output.verbose,
            quiet: output.quiet,
            format: output.format,
            raw_sizes: false,
            full_paths: output.full_paths,
        }
    }
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Base setup and configuration
    Base {
        #[command(flatten)]
        output: OutputFmtQuietVerboseFull,
        #[command(subcommand)]
        action: BaseAction,
    },
    /// Challenge management
    Challenge {
        #[command(subcommand)]
        action: ChallengeAction,
    },
    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Run tests (shortcut)
    Test {
        #[command(flatten)]
        output: OutputFmtQuietVerboseFull,
        /// Stage to test (defaults to first stage in course-definition.yml)
        #[arg(conflicts_with = "all")]
        stage: Option<String>,
        /// Run all stages sequentially
        #[arg(long)]
        all: bool,
        /// Continue on failure (only with --all)
        #[arg(long, requires = "all")]
        continue_on_failure: bool,
        /// Skip validation before running tests
        #[arg(long)]
        skip_validation: bool,
    },
    /// Tester management
    Tester {
        #[command(subcommand)]
        action: TesterAction,
    },
}

#[derive(Subcommand, Clone, Copy)]
pub(crate) enum BaseAction {
    /// Initialize crafter configuration
    Setup,
    /// Show crafter setup status
    Status,
}

#[derive(Subcommand)]
pub(crate) enum ConfigAction {
    /// Show current configuration
    Show {
        #[command(flatten)]
        output: OutputFmtQuietVerboseFull,
    },
    /// Get a configuration value
    Get {
        #[command(flatten)]
        output: OutputFmtQuiet,
        /// Configuration key (e.g., "output.format", "auto_update")
        key: String,
    },
    /// Set a configuration value
    Set {
        #[command(flatten)]
        output: OutputFmtQuiet,
        /// Configuration key (e.g., "output.format", "auto_update")
        key: String,
        /// New value
        value: String,
    },
    /// Reset configuration to defaults
    Reset {
        #[command(flatten)]
        output: OutputFmtQuiet,
    },
    /// Show configuration file path
    Path {
        #[command(flatten)]
        output: OutputFmtQuietFull,
    },
}

#[derive(Subcommand)]
pub(crate) enum ChallengeAction {
    /// Initialize a new challenge project
    Init {
        #[command(flatten)]
        output: OutputFmtQuietVerboseFull,
        /// Challenge name (e.g., shell, redis, grep)
        challenge: String,
        /// Programming language
        language: String,
        /// Initialize git repository and create initial commit
        #[arg(long)]
        git: bool,
    },
    /// List available challenges
    List {
        #[command(flatten)]
        output: OutputFmtQuiet,
        /// Only show installed challenges
        #[arg(long)]
        installed: bool,
    },
    /// List all stages for a challenge
    Stages {
        #[command(flatten)]
        output: OutputFmtQuiet,
        /// Challenge name (optional, auto-detects from current directory)
        challenge: Option<String>,
    },
    /// Update challenge repository
    Update {
        #[command(flatten)]
        output: OutputFmtQuiet,
        /// Challenge to update (optional, updates all if not specified)
        challenge: Option<String>,
    },
    /// Show current project status
    Status {
        #[command(flatten)]
        output: OutputFmtQuietRawFull,
    },
    /// List available languages for a challenge
    Languages {
        #[command(flatten)]
        output: OutputFmtQuiet,
        /// Challenge name
        challenge: String,
    },
    /// Validate current project configuration
    Validate {
        #[command(flatten)]
        output: OutputFmtQuiet,
        /// Show all checks including successful ones
        #[arg(short = 'a', long)]
        all_checks: bool,
        /// Attempt to auto-fix supported issues (currently tester-related only)
        #[arg(long)]
        fix: bool,
    },
}

#[derive(Subcommand)]
pub(crate) enum TesterAction {
    /// Download/build tester for a challenge
    Build {
        #[command(flatten)]
        output: OutputFmtQuietVerboseFull,
        /// Challenge name
        challenge: String,
        /// Force rebuild
        #[arg(long)]
        force: bool,
        /// Specific version to download
        #[arg(long)]
        version: Option<String>,
    },
    /// List installed testers
    List {
        #[command(flatten)]
        output: OutputFmtQuietRawFull,
    },
    /// Clean tester cache
    Clean {
        #[command(flatten)]
        output: OutputFmtQuietRawFull,
        /// Challenge to clean (optional, cleans all if not specified)
        challenge: Option<String>,
    },
}

pub(crate) fn resolve_output_args(command: &Commands) -> ResolvedOutputArgs {
    match command {
        Commands::Base { output, .. } => ResolvedOutputArgs::from_quiet_verbose_full(output),
        Commands::Challenge { action } => match action {
            ChallengeAction::Init { output, .. } => {
                ResolvedOutputArgs::from_quiet_verbose_full(output)
            }
            ChallengeAction::List { output, .. }
            | ChallengeAction::Stages { output, .. }
            | ChallengeAction::Update { output, .. }
            | ChallengeAction::Languages { output, .. }
            | ChallengeAction::Validate { output, .. } => {
                ResolvedOutputArgs::from_quiet_format(output)
            }
            ChallengeAction::Status { output } => ResolvedOutputArgs::from_quiet_raw_full(output),
        },
        Commands::Config { action } => match action {
            ConfigAction::Show { output } => ResolvedOutputArgs::from_quiet_verbose_full(output),
            ConfigAction::Path { output } => ResolvedOutputArgs::from_quiet_full(output),
            ConfigAction::Get { output, .. }
            | ConfigAction::Set { output, .. }
            | ConfigAction::Reset { output } => ResolvedOutputArgs::from_quiet_format(output),
        },
        Commands::Test { output, .. } => ResolvedOutputArgs::from_quiet_verbose_full(output),
        Commands::Tester { action } => match action {
            TesterAction::Build { output, .. } => {
                ResolvedOutputArgs::from_quiet_verbose_full(output)
            }
            TesterAction::List { output } | TesterAction::Clean { output, .. } => {
                ResolvedOutputArgs::from_quiet_raw_full(output)
            }
        },
    }
}