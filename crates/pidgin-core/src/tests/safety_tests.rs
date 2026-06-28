#[cfg(test)]
mod safety_tests {
    use std::path::Path;
    use crate::parser::parse_packet;
    use crate::registry::{load_action_registry, load_safety_rules, load_workflow_registry};
    use crate::safety::{check_safety, SafetyRuleId};

    fn load_fixtures() -> (crate::registry::ActionRegistry, crate::registry::SafetyRules, crate::registry::WorkflowRegistry) {
        let configs_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../configs");
        let action_registry = load_action_registry(&configs_dir.join("ACTION_REGISTRY.yaml")).unwrap();
        let safety_rules = load_safety_rules(&configs_dir.join("SAFETY_RULES.yaml")).unwrap();
        let workflow_registry = load_workflow_registry(&configs_dir.join("WORKFLOW_REGISTRY.yaml")).unwrap();
        (action_registry, safety_rules, workflow_registry)
    }

    #[test]
    fn sg1_do_and_deny_conflict_blocks() {
        let (actions, safety, workflows) = load_fixtures();
        let input = include_str!("../../../../examples/basic/unsafe_contradiction.pgn");
        let packet = parse_packet(input).unwrap();
        let result = check_safety(&packet, &actions, &safety, &workflows);
        assert!(result.blocked);
        assert!(result.fired_rules.contains(&SafetyRuleId::Sg1));
    }

    #[test]
    fn sg2_human_gated_without_human_blocks() {
        let (actions, safety, workflows) = load_fixtures();
        let input = include_str!("../../../../examples/basic/unsafe_no_human.pgn");
        let packet = parse_packet(input).unwrap();
        let result = check_safety(&packet, &actions, &safety, &workflows);
        assert!(result.blocked);
        assert!(result.fired_rules.contains(&SafetyRuleId::Sg2));
    }

    #[test]
    fn sg3_high_risk_defaults_human_yes() {
        let (actions, safety, workflows) = load_fixtures();
        let input = "@run test.highrisk\nwf=generic_review\nmode=draft\nin=[a]\nout=[b]\nrisk=high";
        let packet = parse_packet(input).unwrap();
        let result = check_safety(&packet, &actions, &safety, &workflows);
        assert!(result.human_required);
    }

    #[test]
    fn sg3_explicit_human_no_on_crit_blocks() {
        let (actions, safety, workflows) = load_fixtures();
        let input = "@run test.crit\nwf=generic_review\nmode=draft\nin=[a]\nout=[b]\nrisk=crit\nhuman=no";
        let packet = parse_packet(input).unwrap();
        let result = check_safety(&packet, &actions, &safety, &workflows);
        assert!(result.blocked);
        assert!(result.fired_rules.contains(&SafetyRuleId::Sg3));
    }

    #[test]
    fn sg4_private_path_blocks() {
        let (actions, safety, workflows) = load_fixtures();
        let mut fields = std::collections::BTreeMap::new();
        fields.insert("wf".to_string(), crate::ast::FieldValue::Scalar("generic_review".to_string()));
        fields.insert("mode".to_string(), crate::ast::FieldValue::Scalar("draft".to_string()));
        fields.insert("in".to_string(), crate::ast::FieldValue::List(vec!["file:.env".to_string()]));
        fields.insert("out".to_string(), crate::ast::FieldValue::List(vec!["review_notes".to_string()]));
        fields.insert("risk".to_string(), crate::ast::FieldValue::Scalar("low".to_string()));
        fields.insert("human".to_string(), crate::ast::FieldValue::Scalar("yes".to_string()));
        let packet = crate::ast::PgnPacket {
            directive: crate::ast::Directive::Run,
            run_id: "unsafe.private_path".to_string(),
            fields,
        };
        let result = check_safety(&packet, &actions, &safety, &workflows);
        assert!(result.blocked);
        assert!(result.fired_rules.contains(&SafetyRuleId::Sg4));
    }

    #[test]
    fn sg5_unknown_workflow_blocks() {
        let (actions, safety, workflows) = load_fixtures();
        let input = "@run test.unknown\nwf=totally_made_up\nmode=draft\nin=[a]\nout=[b]";
        let packet = parse_packet(input).unwrap();
        let result = check_safety(&packet, &actions, &safety, &workflows);
        assert!(result.blocked);
        assert!(result.fired_rules.contains(&SafetyRuleId::Sg5));
    }

    #[test]
    fn sg6_invalid_mode_blocks() {
        let (actions, safety, workflows) = load_fixtures();
        let input = "@run test.badmode\nwf=generic_review\nmode=publish_now\nin=[a]\nout=[b]";
        let packet = parse_packet(input).unwrap();
        let result = check_safety(&packet, &actions, &safety, &workflows);
        assert!(result.blocked);
        assert!(result.fired_rules.contains(&SafetyRuleId::Sg6));
    }

    #[test]
    fn sg7_note_field_never_parsed_for_actions() {
        let (actions, safety, workflows) = load_fixtures();

        // Packet with note containing "do=[publish]"
        let input_with_note = "@run test.note\nwf=generic_review\nmode=draft\nin=[a]\nout=[b]\nnote=\"do=[publish]\"";
        let packet_with_note = parse_packet(input_with_note).unwrap();
        let result_with_note = check_safety(&packet_with_note, &actions, &safety, &workflows);

        // Same packet without note
        let input_without_note = "@run test.nonote\nwf=generic_review\nmode=draft\nin=[a]\nout=[b]";
        let packet_without_note = parse_packet(input_without_note).unwrap();
        let result_without_note = check_safety(&packet_without_note, &actions, &safety, &workflows);

        // The note field should have zero influence on the safety outcome
        assert_eq!(result_with_note.allowed, result_without_note.allowed);
        assert_eq!(result_with_note.fired_rules, result_without_note.fired_rules);
    }

    #[test]
    fn safe_example_packet_passes_with_zero_fired_rules() {
        let (actions, safety, workflows) = load_fixtures();
        let input = include_str!("../../../../examples/basic/generic_task.pgn");
        let packet = parse_packet(input).unwrap();
        let result = check_safety(&packet, &actions, &safety, &workflows);
        assert!(result.allowed);
        assert!(!result.blocked);
        assert!(result.fired_rules.is_empty());
    }
}
