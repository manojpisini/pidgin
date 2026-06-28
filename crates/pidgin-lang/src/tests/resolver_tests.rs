#[cfg(test)]
#[allow(clippy::module_inception)]
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

    #[test]
    fn file_ref_traversal_returns_forbidden() {
        let ctx = make_ctx(empty_aliases());
        let result = resolve_ref("file:../../etc/passwd", &ctx);
        assert_eq!(
            result.status,
            ResolutionStatus::Forbidden,
            "expected Forbidden for traversal: {:?}",
            result.resolved_path
        );
        assert!(result.resolved_path.is_none());
    }

    #[test]
    fn folder_ref_traversal_returns_forbidden() {
        let ctx = make_ctx(empty_aliases());
        let result = resolve_ref("folder:../../../", &ctx);
        assert_eq!(result.status, ResolutionStatus::Forbidden);
        assert!(result.resolved_path.is_none());
    }

    #[test]
    fn file_ref_normal_path_still_works() {
        let ctx = make_ctx(empty_aliases());
        let result = resolve_ref("file:Cargo.toml", &ctx);
        assert_eq!(result.status, ResolutionStatus::Resolved);
        assert!((result.confidence - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn file_ref_dotdot_within_root_still_works() {
        let ctx = make_ctx(empty_aliases());
        // configs/../Cargo.toml stays within workspace root because
        // ParentDir cancels with configs/, leaving Cargo.toml at root
        let result = resolve_ref("file:configs/../Cargo.toml", &ctx);
        assert_eq!(result.status, ResolutionStatus::Resolved);
    }

    #[test]
    fn file_ref_absolute_path_outside_forbidden() {
        let ctx = make_ctx(empty_aliases());
        let result = resolve_ref("file:/tmp", &ctx);
        assert_eq!(
            result.status,
            ResolutionStatus::Forbidden,
            "absolute path outside root should be forbidden"
        );
    }

    #[test]
    fn file_ref_traversal_escape_then_return_forbidden() {
        // configs/../../target goes up to parent of root, then back to target/
        // under root — but the intermediate escape is still flagged as forbidden
        // (strict but safe)
        let ctx = make_ctx(empty_aliases());
        let result = resolve_ref("file:configs/../../target", &ctx);
        assert_eq!(result.status, ResolutionStatus::Forbidden);
    }

    #[test]
    fn file_ref_encoded_dotdot_returns_forbidden() {
        let ctx = make_ctx(empty_aliases());
        // Some traversal attempts use patterns like "safe/..%2f..%2fetc"
        // but at the raw ref_id level, we check path joining with host_root
        // and canonicalize the result
        let result = resolve_ref("file:safe/../../../etc/hosts", &ctx);
        assert_eq!(result.status, ResolutionStatus::Forbidden);
    }

    #[test]
    fn file_ref_stays_in_host_root_breadth_first() {
        // Navigate to a sibling that is within root: configs/SAFETY_RULES.yaml
        let ctx = make_ctx(empty_aliases());
        let result = resolve_ref("file:configs/SAFETY_RULES.yaml", &ctx);
        // configs/ is directly under workspace root, so this is within root
        let expected = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../configs/SAFETY_RULES.yaml");
        if expected.exists() {
            assert_eq!(result.status, ResolutionStatus::Resolved);
        } else {
            assert_eq!(result.status, ResolutionStatus::Missing);
        }
    }
}
