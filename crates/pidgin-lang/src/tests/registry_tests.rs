#[cfg(test)]
#[allow(clippy::module_inception)]
mod registry_tests {
    use crate::registry::{load_action_registry, load_safety_rules, load_workflow_registry};
    use std::path::Path;

    #[test]
    fn load_workflow_registry_success() {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../configs/WORKFLOW_REGISTRY.yaml");
        let registry = load_workflow_registry(&path).unwrap();
        assert!(registry.workflows.contains_key("generic_review"));
    }

    #[test]
    fn load_action_registry_success() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../configs/ACTION_REGISTRY.yaml");
        let registry = load_action_registry(&path).unwrap();
        assert!(registry.safe.contains(&"read".to_string()));
        assert!(registry.human_gated.contains(&"publish".to_string()));
    }

    #[test]
    fn load_safety_rules_success() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../configs/SAFETY_RULES.yaml");
        let rules = load_safety_rules(&path).unwrap();
        assert!(rules.private_paths.contains(&".env".to_string()));
    }

    #[test]
    fn load_malformed_yaml_returns_error() {
        let bad_yaml = "invalid: [yaml: broken";
        let result = serde_yaml::from_str::<crate::registry::WorkflowRegistry>(bad_yaml);
        assert!(result.is_err());
    }
}
