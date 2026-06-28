#[cfg(test)]
#[allow(clippy::module_inception)]
mod ast_tests {
    use crate::ast::{Directive, FieldValue, PgnPacket};
    use std::collections::BTreeMap;

    #[test]
    fn construct_pgn_packet() {
        let mut fields = BTreeMap::new();
        fields.insert("wf".to_string(), FieldValue::Scalar("generic_review".to_string()));
        fields.insert("mode".to_string(), FieldValue::Scalar("draft".to_string()));

        let packet = PgnPacket {
            directive: Directive::Run,
            run_id: "example.task".to_string(),
            fields,
        };

        assert_eq!(packet.directive, Directive::Run);
        assert_eq!(packet.run_id, "example.task");
        assert_eq!(
            packet.fields.get("wf"),
            Some(&FieldValue::Scalar("generic_review".to_string()))
        );
    }
}
