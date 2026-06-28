#[cfg(test)]
#[allow(clippy::module_inception)]
mod expander_tests {
    use std::collections::BTreeMap;
    use std::path::Path;

    use crate::ast::{Directive, FieldValue, PgnPacket};
    use crate::expander::{expand_to_approval_request, expand_to_run_packet, ExpandedRef};
    use crate::registry::{load_workflow_registry, WorkflowRegistry};
    use crate::resolver::{ResolvedRef, ResolutionStatus};
    use crate::safety::SafetyResult;

    fn test_workflows() -> WorkflowRegistry {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../configs/WORKFLOW_REGISTRY.yaml");
        load_workflow_registry(&path).unwrap()
    }

    fn default_safety() -> SafetyResult {
        SafetyResult {
            allowed: true,
            blocked: false,
            fired_rules: vec![],
            human_required: false,
            effective_risk: "med".to_string(),
        }
    }

    fn make_packet() -> PgnPacket {
        let mut fields = BTreeMap::new();
        fields.insert(
            "wf".to_string(),
            FieldValue::Scalar("generic_review".to_string()),
        );
        fields.insert("mode".to_string(), FieldValue::Scalar("draft".to_string()));
        fields.insert(
            "in".to_string(),
            FieldValue::List(vec!["primary_subject".to_string(), "source_refs".to_string()]),
        );
        fields.insert(
            "out".to_string(),
            FieldValue::List(vec!["review_notes".to_string()]),
        );
        fields.insert(
            "do".to_string(),
            FieldValue::List(vec!["draft".to_string(), "review".to_string()]),
        );
        fields.insert(
            "deny".to_string(),
            FieldValue::List(vec!["publish".to_string()]),
        );
        fields.insert("risk".to_string(), FieldValue::Scalar("med".to_string()));
        PgnPacket {
            directive: Directive::Run,
            run_id: "test.expand".to_string(),
            fields,
        }
    }

    #[test]
    fn expand_run_packet_includes_metadata() {
        let packet = make_packet();
        let resolved = vec![];
        let safety = default_safety();
        let workflows = test_workflows();

        let expanded = expand_to_run_packet(&packet, &resolved, &safety, &workflows);

        assert_eq!(expanded.spec_version, "1.0");
        assert_eq!(expanded.run_id, "test.expand");
        assert_eq!(expanded.workflow, "generic_review");
        assert_eq!(expanded.mode, "draft");
    }

    #[test]
    fn expand_run_packet_includes_actions() {
        let packet = make_packet();
        let resolved = vec![];
        let safety = default_safety();
        let workflows = test_workflows();

        let expanded = expand_to_run_packet(&packet, &resolved, &safety, &workflows);

        assert_eq!(expanded.do_actions, vec!["draft", "review"]);
        assert_eq!(expanded.deny_actions, vec!["publish"]);
    }

    #[test]
    fn expand_run_packet_includes_resolved_refs() {
        let packet = make_packet();
        let resolved = vec![
            ResolvedRef {
                original: "primary_subject".to_string(),
                namespace: String::new(),
                ref_id: "primary_subject".to_string(),
                resolved_path: None,
                confidence: 0.0,
                required: true,
                status: ResolutionStatus::Unresolved,
            },
            ResolvedRef {
                original: "source_refs".to_string(),
                namespace: String::new(),
                ref_id: "source_refs".to_string(),
                resolved_path: None,
                confidence: 0.0,
                required: true,
                status: ResolutionStatus::Unresolved,
            },
        ];
        let safety = default_safety();
        let workflows = test_workflows();

        let expanded = expand_to_run_packet(&packet, &resolved, &safety, &workflows);

        assert_eq!(expanded.inputs.len(), 2);
        assert_eq!(expanded.inputs[0].reference, "primary_subject");
    }

    #[test]
    fn expand_run_packet_uses_safety_risk() {
        let packet = make_packet();
        let resolved = vec![];
        let safety = SafetyResult {
            effective_risk: "high".to_string(),
            ..default_safety()
        };
        let workflows = test_workflows();

        let expanded = expand_to_run_packet(&packet, &resolved, &safety, &workflows);

        assert_eq!(expanded.effective_risk, "high");
    }

    #[test]
    fn expand_run_packet_includes_executor_from_workflow() {
        let packet = make_packet();
        let resolved = vec![];
        let safety = default_safety();
        let workflows = test_workflows();

        let expanded = expand_to_run_packet(&packet, &resolved, &safety, &workflows);

        assert_eq!(expanded.recommended_executor, "claude-code");
        assert_eq!(expanded.fallback_executor, "opencode");
        assert_eq!(expanded.ttl, "24h");
    }

    #[test]
    fn expand_approval_request_includes_actions_and_risk() {
        let packet = make_packet();
        let safety = SafetyResult {
            effective_risk: "crit".to_string(),
            human_required: true,
            ..default_safety()
        };
        let workflows = test_workflows();

        let approval = expand_to_approval_request(&packet, &safety, &workflows);

        assert_eq!(approval.spec_version, "1.0");
        assert_eq!(approval.run_id, "test.expand");
        assert_eq!(approval.workflow, "generic_review");
        assert_eq!(approval.risk, "crit");
        assert!(approval.human_required);
        assert_eq!(approval.actions, vec!["draft", "review"]);
    }

    #[test]
    fn expanded_ref_from_resolved_ref() {
        let r = ResolvedRef {
            original: "file:Cargo.toml".to_string(),
            namespace: "file".to_string(),
            ref_id: "Cargo.toml".to_string(),
            resolved_path: Some(Path::new("Cargo.toml").to_path_buf()),
            confidence: 1.0,
            required: true,
            status: ResolutionStatus::Resolved,
        };
        let er = ExpandedRef::from(&r);
        assert_eq!(er.reference, "file:Cargo.toml");
        assert_eq!(er.status, "Resolved");
        assert!((er.confidence - 1.0).abs() < f32::EPSILON);
        assert_eq!(er.path, Some("Cargo.toml".to_string()));
    }

    #[test]
    fn expand_with_note() {
        let mut packet = make_packet();
        packet
            .fields
            .insert("note".to_string(), FieldValue::Scalar("test note".to_string()));
        let resolved = vec![];
        let safety = default_safety();
        let workflows = test_workflows();

        let expanded = expand_to_run_packet(&packet, &resolved, &safety, &workflows);

        assert_eq!(expanded.note, Some("test note".to_string()));
    }

    #[test]
    fn expand_without_note() {
        let packet = make_packet();
        let resolved = vec![];
        let safety = default_safety();
        let workflows = test_workflows();

        let expanded = expand_to_run_packet(&packet, &resolved, &safety, &workflows);

        assert_eq!(expanded.note, None);
    }
}
