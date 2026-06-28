#[cfg(test)]
#[allow(clippy::module_inception)]
mod syntax_validator_tests {
    use crate::parser::parse_packet;
    use crate::validator::syntax::validate_syntax;

    #[test]
    fn valid_example_packet() {
        let input = include_str!("../../../../examples/basic/generic_task.pgn");
        let packet = parse_packet(input).unwrap();
        let errors = validate_syntax(&packet);
        assert!(errors.is_empty());
    }

    #[test]
    fn missing_wf_field() {
        let input = "@run test.missing\nmode=draft\nin=[a]\nout=[b]";
        let packet = parse_packet(input).unwrap();
        let errors = validate_syntax(&packet);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "PGN_E001");
        assert!(errors[0].message.contains("wf"));
    }

    #[test]
    fn in_field_not_a_list() {
        let input = "@run test.notlist\nwf=generic_review\nmode=draft\nin=not_a_list\nout=[b]";
        let packet = parse_packet(input).unwrap();
        let errors = validate_syntax(&packet);
        assert!(errors.iter().any(|e| e.code == "PGN_E016"));
    }

    #[test]
    fn result_directive_requires_status() {
        let input = "@result test.result\nwf=generic_review\nout=[a]";
        let packet = parse_packet(input).unwrap();
        let errors = validate_syntax(&packet);
        assert!(errors.iter().any(|e| e.message.contains("status")));
    }
}
