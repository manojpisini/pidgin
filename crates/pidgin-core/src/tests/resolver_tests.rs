#[cfg(test)]
mod resolver_tests {
    use std::path::Path;

    use crate::ast::{Directive, FieldValue, PgnPacket};
    use crate::resolver::{
        expand_alias, load_aliases, parse_ref, resolve_all, resolve_ref, ReferenceAliases,
        ResolverContext, ResolutionStatus,
    };

    fn empty_aliases() -> ReferenceAliases {
        ReferenceAliases {
            aliases: std::collections::BTreeMap::new(),
            common: std::collections::BTreeMap::new(),
        }
    }

    fn make_ctx(aliases: ReferenceAliases) -> ResolverContext {
        ResolverContext {
            host_root: Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
            aliases,
            required_inputs: vec![],
        }
    }

    #[test]
    fn parse_namespace_ref() {
        let (ns, id) = parse_ref("file:.env").unwrap();
        assert_eq!(ns, "file");
        assert_eq!(id, ".env");
    }

    #[test]
    fn parse_multi_colon_ref() {
        let (ns, id) = parse_ref("ep:EP012").unwrap();
        assert_eq!(ns, "ep");
        assert_eq!(id, "EP012");
    }

    #[test]
    fn parse_bare_alias() {
        let (ns, id) = parse_ref("primary_subject").unwrap();
        assert_eq!(ns, "");
        assert_eq!(id, "primary_subject");
    }

    #[test]
    fn parse_empty_ref_returns_none() {
        assert!(parse_ref("").is_none());
        assert!(parse_ref("   ").is_none());
    }

    #[test]
    fn expand_known_alias() {
        let mut aliases = empty_aliases();
        aliases
            .common
            .insert("script".to_string(), "file:scripts/build.sh".to_string());
        assert_eq!(
            expand_alias("script", &aliases),
            Some("file:scripts/build.sh".to_string())
        );
    }

    #[test]
    fn expand_unknown_alias() {
        let aliases = empty_aliases();
        assert_eq!(expand_alias("nonexistent", &aliases), None);
    }

    #[test]
    fn file_ref_exists_resolved() {
        let ctx = make_ctx(empty_aliases());
        let result = resolve_ref("file:Cargo.toml", &ctx);
        assert_eq!(result.status, ResolutionStatus::Resolved);
        assert!((result.confidence - 1.0).abs() < f32::EPSILON);
        assert!(result.resolved_path.unwrap().ends_with("Cargo.toml"));
    }

    #[test]
    fn file_ref_not_exists_missing() {
        let ctx = make_ctx(empty_aliases());
        let result = resolve_ref("file:nonexistent_file_xyz", &ctx);
        assert_eq!(result.status, ResolutionStatus::Missing);
        assert!((result.confidence - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn folder_ref_exists_resolved() {
        let ctx = make_ctx(empty_aliases());
        let result = resolve_ref("folder:configs", &ctx);
        assert_eq!(result.status, ResolutionStatus::Resolved);
        assert!((result.confidence - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn unknown_namespace_unresolved() {
        let ctx = make_ctx(empty_aliases());
        let result = resolve_ref("ep:ABC123", &ctx);
        assert_eq!(result.status, ResolutionStatus::Unresolved);
        assert!((result.confidence - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn bare_alias_unresolved() {
        let ctx = make_ctx(empty_aliases());
        let result = resolve_ref("primary_subject", &ctx);
        assert_eq!(result.status, ResolutionStatus::Unresolved);
    }

    #[test]
    fn alias_expands_file() {
        let mut aliases = empty_aliases();
        aliases
            .common
            .insert("manifest".to_string(), "file:Cargo.toml".to_string());
        let ctx = make_ctx(aliases);
        let result = resolve_ref("manifest", &ctx);
        assert_eq!(result.status, ResolutionStatus::Resolved);
    }

    #[test]
    fn resolve_all_returns_in_and_out() {
        let mut fields = std::collections::BTreeMap::new();
        fields.insert(
            "in".to_string(),
            FieldValue::List(vec!["file:Cargo.toml".to_string()]),
        );
        fields.insert(
            "out".to_string(),
            FieldValue::List(vec!["ep:OUT001".to_string()]),
        );
        fields.insert(
            "wf".to_string(),
            FieldValue::Scalar("generic_review".to_string()),
        );
        let packet = PgnPacket {
            directive: Directive::Run,
            run_id: "test.resolve_all".to_string(),
            fields,
        };

        let ctx = make_ctx(empty_aliases());
        let results = resolve_all(&packet, &ctx);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].original, "file:Cargo.toml");
        assert_eq!(results[1].original, "ep:OUT001");
    }

    #[test]
    fn required_input_field_marked_required() {
        let ctx = ResolverContext {
            host_root: Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
            aliases: empty_aliases(),
            required_inputs: vec!["file:.env".to_string()],
        };
        let result = resolve_ref("file:.env", &ctx);
        assert!(result.required);
    }

    #[test]
    fn non_required_input_not_marked() {
        let ctx = ResolverContext {
            host_root: Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
            aliases: empty_aliases(),
            required_inputs: vec!["something_else".to_string()],
        };
        let result = resolve_ref("file:.env", &ctx);
        assert!(!result.required);
    }

    #[test]
    fn load_aliases_from_config() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../configs/REFERENCE_ALIASES.yaml");
        let aliases = load_aliases(&path).unwrap();
        assert!(aliases.aliases.is_empty());
        assert!(aliases.common.is_empty());
    }
}
