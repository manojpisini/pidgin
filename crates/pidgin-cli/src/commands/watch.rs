use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use super::{canonicalize_host, load_pipeline_configs, process_packet};

pub fn run(folder: PathBuf, host: PathBuf) {
    let config_dir = host.join(".pidgin");
    let inbox = folder.join(".pidgin").join("inbox");
    fs::create_dir_all(&inbox).unwrap_or_else(|e| {
        eprintln!("error: cannot create inbox {}: {}", inbox.display(), e);
        std::process::exit(1);
    });
    let outbox = folder.join(".pidgin").join("generated");
    fs::create_dir_all(&outbox).unwrap_or_else(|e| {
        eprintln!("error: cannot create outbox {}: {}", outbox.display(), e);
        std::process::exit(1);
    });

    let cfg = load_pipeline_configs(&folder);
    let host_root = canonicalize_host(&folder);

    let mut seen: HashSet<PathBuf> = HashSet::new();
    eprintln!(
        "watching {} for .pgn files (poll every 2s)...",
        inbox.display()
    );

    loop {
        let entries = match fs::read_dir(&inbox) {
            Ok(e) => e,
            Err(_) => {
                std::thread::sleep(Duration::from_secs(2));
                continue;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("pgn") {
                continue;
            }
            if !seen.insert(path.clone()) {
                continue;
            }

            eprintln!("processing: {}", path.display());
            process_packet(&path, &cfg, &host_root, &outbox, &config_dir);
        }

        std::thread::sleep(Duration::from_secs(2));
    }
}
