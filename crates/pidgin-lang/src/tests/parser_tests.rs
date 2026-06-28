#[cfg(test)]
#[allow(clippy::module_inception)]
mod parser_tests {
    use crate::parser::parse_packet;
    use crate::ast::{Directive, FieldValue};

    const EXAMPLE_PACKET: &str = include_str!("../../../../examples/basic/generic_task.pgn");

    #[test]
    fn parse_example_packet() {
        let packet = parse_packet(EXAMPLE_PACKET).unwrap();

        assert_eq!(packet.directive, Directive::Run);
        assert_eq!(packet.run_id, "example.task");
        assert_eq!(
            packet.fields.get("wf"),
            Some(&FieldValue::Scalar("generic_review".to_string()))
        );
        assert_eq!(
            packet.fields.get("mode"),
            Some(&FieldValue::Scalar("draft".to_string()))
        );
        assert_eq!(
            packet.fields.get("in"),
            Some(&FieldValue::List(vec![
                "primary_subject".to_string(),
                "source_refs".to_string(),
            ]))
        );
        assert_eq!(
            packet.fields.get("out"),
            Some(&FieldValue::List(vec![
                "review_notes".to_string(),
            ]))
        );
        assert_eq!(
            packet.fields.get("do"),
            Some(&FieldValue::List(vec![
                "draft".to_string(),
                "review".to_string(),
            ]))
        );
        assert_eq!(
            packet.fields.get("deny"),
            Some(&FieldValue::List(vec![
                "publish".to_string(),
                "send".to_string(),
                "delete".to_string(),
                "secrets".to_string(),
            ]))
        );
        assert_eq!(
            packet.fields.get("risk"),
            Some(&FieldValue::Scalar("med".to_string()))
        );
        assert_eq!(
            packet.fields.get("human"),
            Some(&FieldValue::Scalar("yes".to_string()))
        );
    }

    #[test]
    fn parse_missing_header_returns_error() {
        let input = "wf=generic_review\nmode=draft";
        let result = parse_packet(input);
        assert!(result.is_err());
    }

    #[test]
    fn parse_empty_input_returns_error() {
        let result = parse_packet("");
        assert!(result.is_err());
    }

    #[test]
    fn parse_duplicate_field_returns_error() {
        let input = "@run test.dup\nwf=a\nwf=b";
        let result = parse_packet(input);
        assert!(result.is_err());
    }

    #[test]
    fn parse_with_comments() {
        let input = "@run test.comment\n# this is a comment\nwf=review\n# another comment";
        let packet = parse_packet(input).unwrap();
        assert_eq!(packet.directive, Directive::Run);
        assert_eq!(packet.fields.len(), 1);
    }

    #[test]
    fn parse_list_with_spaces() {
        let input = "@run test.spaces\nin=[ a , b , c ]";
        let packet = parse_packet(input).unwrap();
        assert_eq!(
            packet.fields.get("in"),
            Some(&FieldValue::List(vec![
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
            ]))
        );
    }

    #[test]
    fn parse_list_value_with_spaces_directly() {
        use crate::lexer::list_value;
        use winnow::Parser;
        let mut input = "[ a , b , c ]";
        let result = list_value.parse_next(&mut input);
        println!("Result: {:?}", result);
        println!("Remaining input: {:?}", input);
        let result = result.unwrap();
        assert_eq!(result, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    }
}
