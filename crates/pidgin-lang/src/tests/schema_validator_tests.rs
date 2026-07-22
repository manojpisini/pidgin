#[cfg(test)]
#[allow(clippy::module_inception)]
mod schema_validator_tests {
    use crate::parser::parse_packet;
    use crate::registry::load_workflow_registry;
    use crate::validator::schema::validate_schema;
    use std::path::Path;

    fn load_workflows() -> crate::registry::WorkflowRegistry {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.pidgin/WORKFLOW_REGISTRY.yaml");
        load_workflow_registry(&path).unwrap()
    }

    #[test]
    fn valid_example_packet() {
        let workflows = load_workflows();
        let input = include_str!("../../../../examples/basic/generic_task.pgn");
        let packet = parse_packet(input).unwrap();
        let errors = validate_schema(&packet, &workflows);
        assert!(errors.is_empty());
    }

    #[test]
    fn unknown_workflow() {
        let workflows = load_workflows();
        let input = "@run test.unknown\nwf=totally_made_up\nmode=draft\nin=[a]\nout=[b]";
        let packet = parse_packet(input).unwrap();
        let errors = validate_schema(&packet, &workflows);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "PGN_E002");
    }

    #[test]
    fn invalid_mode() {
        let workflows = load_workflows();
        let input = "@run test.badmode\nwf=generic_review\nmode=publish_now\nin=[a]\nout=[b]";
        let packet = parse_packet(input).unwrap();
        let errors = validate_schema(&packet, &workflows);
        assert!(errors.iter().any(|e| e.code == "PGN_E003"));
    }

    #[test]
    fn invalid_risk_level() {
        let workflows = load_workflows();
        let input =
            "@run test.badrisk\nwf=generic_review\nmode=draft\nin=[a]\nout=[b]\nrisk=extreme";
        let packet = parse_packet(input).unwrap();
        let errors = validate_schema(&packet, &workflows);
        assert!(errors.iter().any(|e| e.code == "PGN_E004"));
    }
}
