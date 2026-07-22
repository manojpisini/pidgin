use std::fs;
use std::path::PathBuf;

use pidgin_lang::context::build_context_plan;
use pidgin_lang::parser::parse_packet;
use pidgin_lang::resolver::{resolve_all, ResolverContext};

use super::{canonicalize_host, get_required_inputs, load_pipeline_configs};

pub fn run(file: PathBuf, host: PathBuf, json: bool) {
    let cfg = load_pipeline_configs(&host);
    let host_root = canonicalize_host(&host);
    let content = fs::read_to_string(&file).unwrap_or_else(|e| {
        eprintln!("{}: Error reading file: {}", file.display(), e);
        std::process::exit(1);
    });
    let packet = parse_packet(&content).unwrap_or_else(|e| {
        eprintln!("{}: Parse error: {}", file.display(), e);
        std::process::exit(1);
    });
    let required_inputs = get_required_inputs(&packet, &cfg.workflows);
    let ctx = ResolverContext {
        host_root,
        aliases: cfg.aliases,
        required_inputs,
    };
    let resolved = resolve_all(&packet, &ctx);
    let plan = build_context_plan(&packet, &resolved);
    if json {
        println!("{}", serde_json::to_string_pretty(&plan).unwrap());
    } else {
        let yaml = serde_yaml::to_string(&plan).unwrap_or_else(|e| {
            eprintln!("Error serializing context plan: {}", e);
            std::process::exit(5);
        });
        println!("{}", yaml.trim());
    }
}
