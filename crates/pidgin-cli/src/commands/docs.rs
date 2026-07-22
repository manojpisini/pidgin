use std::fs;
use std::path::PathBuf;

pub fn run() {
    let spec_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("docs")
        .join("SPEC.md");
    match fs::read_to_string(&spec_path) {
        Ok(content) => println!("{}", content),
        Err(_) => {
            eprintln!("docs/SPEC.md not found in repository");
            std::process::exit(1);
        }
    }
}
