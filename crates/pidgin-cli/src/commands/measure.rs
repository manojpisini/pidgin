use std::fs;
use std::path::PathBuf;

use pidgin_lang::metrics::{estimate_tokens, measure_packet};
use pidgin_lang::parser::parse_packet;

pub fn run(file: PathBuf, json: bool) {
    let content = fs::read_to_string(&file).unwrap_or_else(|e| {
        eprintln!("{}: Error reading file: {}", file.display(), e);
        std::process::exit(1);
    });
    let _tokens = estimate_tokens(&content);
    match parse_packet(&content) {
        Ok(packet) => {
            let report = measure_packet(&packet);
            if json {
                println!("{}", serde_json::to_string_pretty(&report).unwrap());
            } else {
                let yaml = serde_yaml::to_string(&report).unwrap_or_else(|e| {
                    eprintln!("Error: {}", e);
                    std::process::exit(5);
                });
                println!("{}", yaml.trim());
            }
        }
        Err(_) => {
            if json {
                let output = serde_json::json!({
                    "char_count": content.len(),
                    "estimated_tokens": _tokens,
                });
                println!("{}", serde_json::to_string_pretty(&output).unwrap());
            } else {
                println!("char_count: {}", content.len());
                println!("estimated_tokens: {}", _tokens);
            }
        }
    }
}
