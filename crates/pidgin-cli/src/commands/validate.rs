use std::fs;
use std::path::PathBuf;

use pidgin_lang::parser::parse_packet;
use pidgin_lang::validator::schema::validate_schema;
use pidgin_lang::validator::syntax::validate_syntax;

use super::load_pipeline_configs;

pub fn run(files: Vec<PathBuf>, host: PathBuf, _json: bool) {
    let cfg = load_pipeline_configs(&host);
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
        let mut errors = validate_syntax(&packet);
        errors.extend(validate_schema(&packet, &cfg.workflows));
        if errors.is_empty() {
            println!("{}: PASS", file.display());
        } else {
            eprintln!("{}: FAIL", file.display());
            for err in &errors {
                eprintln!("  [{}] {}", err.code, err.message);
            }
            all_passed = false;
        }
    }
    if !all_passed {
        std::process::exit(1);
    }
}
