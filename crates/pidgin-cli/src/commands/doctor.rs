use std::path::PathBuf;

pub fn run(host: PathBuf) {
    let config_dir = host.join(".pidgin");
    let checks = vec![
        (
            "WORKFLOW_REGISTRY.yaml",
            config_dir.join("WORKFLOW_REGISTRY.yaml"),
        ),
        (
            "ACTION_REGISTRY.yaml",
            config_dir.join("ACTION_REGISTRY.yaml"),
        ),
        ("SAFETY_RULES.yaml", config_dir.join("SAFETY_RULES.yaml")),
        (
            "REFERENCE_ALIASES.yaml",
            config_dir.join("REFERENCE_ALIASES.yaml"),
        ),
    ];

    let mut all_ok = true;
    for (name, path) in &checks {
        if path.exists() {
            println!("  OK  {}", name);
        } else {
            eprintln!("  MISS  {}", name);
            all_ok = false;
        }
    }

    for (name, path) in &checks {
        if path.exists() {
            match std::fs::read_to_string(path) {
                Ok(content) => {
                    if serde_yaml::from_str::<serde_yaml::Value>(&content).is_ok() {
                        println!("  OK  {} (valid YAML)", name);
                    } else {
                        eprintln!("  INVALID  {} (malformed YAML)", name);
                        all_ok = false;
                    }
                }
                Err(e) => {
                    eprintln!("  ERR  {}: {}", name, e);
                    all_ok = false;
                }
            }
        }
    }

    if all_ok {
        println!("All checks passed");
    } else {
        std::process::exit(4);
    }
}
