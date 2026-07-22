use std::fs;
use std::path::{Path, PathBuf};

use super::{
    DEFAULT_ACTION_REGISTRY, DEFAULT_REFERENCE_ALIASES, DEFAULT_SAFETY_RULES,
    DEFAULT_WORKFLOW_REGISTRY,
};

pub fn run(host: PathBuf, force: bool) {
    let config_dir = host.join(".pidgin");
    let logs_dir = config_dir.join("logs");

    if config_dir.exists() && !force {
        eprintln!(
            "{} already exists (use --force to overwrite)",
            config_dir.display()
        );
        std::process::exit(1);
    }

    fs::create_dir_all(&logs_dir).unwrap_or_else(|e| {
        eprintln!("error: cannot create {}: {}", logs_dir.display(), e);
        std::process::exit(1);
    });

    let files: Vec<(&str, &Path, &str)> = vec![
        (
            "WORKFLOW_REGISTRY.yaml",
            &config_dir,
            DEFAULT_WORKFLOW_REGISTRY,
        ),
        ("ACTION_REGISTRY.yaml", &config_dir, DEFAULT_ACTION_REGISTRY),
        ("SAFETY_RULES.yaml", &config_dir, DEFAULT_SAFETY_RULES),
        (
            "REFERENCE_ALIASES.yaml",
            &config_dir,
            DEFAULT_REFERENCE_ALIASES,
        ),
    ];

    let mut count = 0;
    for (name, dir, content) in &files {
        let path = dir.join(name);
        if path.exists() && !force {
            println!("  SKIP {}", name);
            continue;
        }
        fs::write(&path, content).unwrap_or_else(|e| {
            eprintln!("error: cannot write {}: {}", path.display(), e);
            std::process::exit(1);
        });
        println!("  CREATE {}", name);
        count += 1;
    }

    if count > 0 {
        println!("initialized pidgin host config in {}", config_dir.display());
    } else {
        println!("nothing to do (use --force to overwrite existing files)");
    }
}
