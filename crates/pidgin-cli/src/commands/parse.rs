use std::fs;
use std::path::PathBuf;

use pidgin_lang::parser::parse_packet;

pub fn run(file: PathBuf) {
    let content = match fs::read_to_string(&file) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            std::process::exit(1);
        }
    };
    match parse_packet(&content) {
        Ok(packet) => println!("{:#?}", packet),
        Err(e) => {
            eprintln!("Parse error: {}", e);
            std::process::exit(1);
        }
    }
}
