use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod commands;

#[derive(Parser)]
#[command(
    name = "pgn",
    about = "Pidgin — A compact agent handoff protocol runtime",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse a Pidgin packet and print the AST
    Parse {
        file: PathBuf,
        #[arg(long)]
        json: bool,
    },

    /// Validate a Pidgin packet (syntax + schema)
    Validate {
        #[arg(required = true)]
        files: Vec<PathBuf>,
        #[arg(long, default_value = ".")]
        host: PathBuf,
        #[arg(long)]
        json: bool,
    },

    /// Validate → safety gate → resolve, end to end
    Check {
        file: PathBuf,
        #[arg(long, default_value = ".")]
        host: PathBuf,
        #[arg(long)]
        json: bool,
    },

    /// Resolve all short references in a packet
    Resolve {
        file: PathBuf,
        #[arg(long, default_value = ".")]
        host: PathBuf,
        #[arg(long)]
        json: bool,
    },

    /// Expand a packet into its executable form
    Expand {
        file: PathBuf,
        #[arg(long, default_value = ".")]
        host: PathBuf,
        #[arg(long)]
        r#out: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },

    /// Build a context plan for what to retrieve
    ContextPlan {
        file: PathBuf,
        #[arg(long, default_value = ".")]
        host: PathBuf,
        #[arg(long)]
        json: bool,
    },

    /// Estimate token cost of a packet
    Measure {
        file: PathBuf,
        #[arg(long)]
        json: bool,
    },

    /// Run the full pipeline (parse → validate → safety → resolve → expand)
    Run {
        file: PathBuf,
        #[arg(long, default_value = ".")]
        host: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
    },

    /// Watch a folder for new .pgn files and process them automatically
    Watch {
        #[arg(default_value = ".")]
        folder: PathBuf,
        #[arg(long, default_value = ".")]
        host: PathBuf,
    },

    /// Check host configuration
    Doctor {
        #[arg(long, default_value = ".")]
        host: PathBuf,
    },

    /// Scaffold a default .pidgin/ config directory
    Init {
        #[arg(long, default_value = ".")]
        host: PathBuf,
        #[arg(long)]
        force: bool,
    },

    /// Print full documentation for agents (grammar, CLI, safety, integration)
    Docs,

    /// Start the HTTP server
    #[cfg(feature = "server")]
    Serve {
        #[arg(long, default_value = "0.0.0.0:3847")]
        bind: std::net::SocketAddr,
        #[arg(long, default_value = ".")]
        host: PathBuf,
    },
}

#[cfg(feature = "server")]
#[tokio::main]
async fn main() {
    run().await;
}

#[cfg(not(feature = "server"))]
fn main() {
    run();
}

#[cfg(feature = "server")]
async fn run() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Parse { file, json } => commands::parse::run(file, json),
        Commands::Validate { files, host, json } => commands::validate::run(files, host, json),
        Commands::Check { file, host, json } => commands::check::run(file, host, json),
        Commands::Resolve { file, host, json } => commands::resolve::run(file, host, json),
        Commands::Expand {
            file,
            host,
            r#out,
            json,
        } => commands::expand::run(file, host, r#out, json),
        Commands::ContextPlan { file, host, json } => commands::context_plan::run(file, host, json),
        Commands::Measure { file, json } => commands::measure::run(file, json),
        Commands::Run { file, host, out } => commands::run::run(file, host, out),
        Commands::Watch { folder, host } => commands::watch::run(folder, host),
        Commands::Doctor { host } => commands::doctor::run(host),
        Commands::Init { host, force } => commands::init::run(host, force),
        Commands::Docs => commands::docs::run(),
        #[cfg(feature = "server")]
        Commands::Serve { bind, host } => commands::serve::run(bind, host).await,
    }
}

#[cfg(not(feature = "server"))]
fn run() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Parse { file, json } => commands::parse::run(file, json),
        Commands::Validate { files, host, json } => commands::validate::run(files, host, json),
        Commands::Check { file, host, json } => commands::check::run(file, host, json),
        Commands::Resolve { file, host, json } => commands::resolve::run(file, host, json),
        Commands::Expand {
            file,
            host,
            r#out,
            json,
        } => commands::expand::run(file, host, r#out, json),
        Commands::ContextPlan { file, host, json } => commands::context_plan::run(file, host, json),
        Commands::Measure { file, json } => commands::measure::run(file, json),
        Commands::Run { file, host, out } => commands::run::run(file, host, out),
        Commands::Watch { folder, host } => commands::watch::run(folder, host),
        Commands::Doctor { host } => commands::doctor::run(host),
        Commands::Init { host, force } => commands::init::run(host, force),
        Commands::Docs => commands::docs::run(),
    }
}
