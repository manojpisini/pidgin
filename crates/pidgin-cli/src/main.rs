use std::fs;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use pidgin_core::parser::parse_packet;
use pidgin_core::registry::load_workflow_registry;
use pidgin_core::validator::syntax::validate_syntax;
use pidgin_core::validator::schema::validate_schema;

#[derive(Parser)]
#[command(name = "pgn", about = "Pidgin — A compact agent handoff protocol runtime")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse a Pidgin packet and print the AST
    Parse {
        /// Path to the .pgn file
        file: PathBuf,
    },

    /// Validate a Pidgin packet (syntax + schema)
    Validate {
        /// Path to the .pgn file(s)
        #[arg(required = true)]
        files: Vec<PathBuf>,

        /// Host root directory (for config lookup)
        #[arg(long, default_value = ".")]
        host: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Parse { file } => {
            let content = match fs::read_to_string(&file) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error reading file: {}", e);
                    std::process::exit(1);
                }
            };

            match parse_packet(&content) {
                Ok(packet) => {
                    println!("{:#?}", packet);
                }
                Err(e) => {
                    eprintln!("Parse error: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Validate { files, host } => {
            let config_dir = host.join(".pidgin");
            let workflow_path = config_dir.join("WORKFLOW_REGISTRY.yaml");

            let workflows = match load_workflow_registry(&workflow_path) {
                Ok(w) => w,
                Err(e) => {
                    eprintln!("Error loading workflow registry: {}", e);
                    std::process::exit(4);
                }
            };

            let mut all_passed = true;

            for file in &files {
                let content = match fs::read_to_string(file) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("{}: FAIL (read error: {})", file.display(), e);
                        all_passed = false;
                        continue;
                    }
                };

                let packet = match parse_packet(&content) {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("{}: FAIL (parse error: {})", file.display(), e);
                        all_passed = false;
                        continue;
                    }
                };

                let syntax_errors = validate_syntax(&packet);
                let schema_errors = validate_schema(&packet, &workflows);

                let mut file_errors = syntax_errors;
                file_errors.extend(schema_errors);

                if file_errors.is_empty() {
                    println!("{}: PASS", file.display());
                } else {
                    eprintln!("{}: FAIL", file.display());
                    for err in &file_errors {
                        eprintln!("  [{}] {}", err.code, err.message);
                    }
                    all_passed = false;
                }
            }

            if !all_passed {
                std::process::exit(1);
            }
        }
    }
}
