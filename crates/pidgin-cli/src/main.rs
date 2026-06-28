use std::fs;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use pidgin_core::parser::parse_packet;

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
    }
}
