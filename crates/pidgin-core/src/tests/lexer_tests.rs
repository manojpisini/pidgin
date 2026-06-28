#[cfg(test)]
mod lexer_tests {
    use crate::lexer::{header_line, field_line, list_value, scalar_value};
    use crate::ast::{Directive, FieldValue};
    use winnow::Parser;

    #[test]
    fn parse_header_run() {
        let mut input = "@run EP012.dist";
        let result = header_line.parse_next(&mut input).unwrap();
        assert_eq!(result, (Directive::Run, "EP012.dist".to_string()));
    }

    #[test]
    fn parse_header_result() {
        let mut input = "@result EP012.dist";
        let result = header_line.parse_next(&mut input).unwrap();
        assert_eq!(result, (Directive::Result, "EP012.dist".to_string()));
    }

    #[test]
    fn parse_header_approval() {
        let mut input = "@approval EP012.dist";
        let result = header_line.parse_next(&mut input).unwrap();
        assert_eq!(result, (Directive::Approval, "EP012.dist".to_string()));
    }

    #[test]
    fn parse_header_context() {
        let mut input = "@context EP012.dist";
        let result = header_line.parse_next(&mut input).unwrap();
        assert_eq!(result, (Directive::Context, "EP012.dist".to_string()));
    }

    #[test]
    fn parse_field_scalar() {
        let mut input = "wf=generic_review";
        let result = field_line.parse_next(&mut input).unwrap();
        assert_eq!(
            result,
            ("wf".to_string(), FieldValue::Scalar("generic_review".to_string()))
        );
    }

    #[test]
    fn parse_field_list() {
        let mut input = "in=[a,b,c]";
        let result = field_line.parse_next(&mut input).unwrap();
        assert_eq!(
            result,
            (
                "in".to_string(),
                FieldValue::List(vec!["a".to_string(), "b".to_string(), "c".to_string()])
            )
        );
    }

    #[test]
    fn parse_list_empty() {
        let mut input = "[]";
        let result = list_value.parse_next(&mut input).unwrap();
        assert_eq!(result, Vec::<String>::new());
    }

    #[test]
    fn parse_list_single() {
        let mut input = "[hello]";
        let result = list_value.parse_next(&mut input).unwrap();
        assert_eq!(result, vec!["hello".to_string()]);
    }

    #[test]
    fn parse_list_multiple() {
        let mut input = "[a,b,c]";
        let result = list_value.parse_next(&mut input).unwrap();
        assert_eq!(result, vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    }

    #[test]
    fn parse_scalar_bare_word() {
        let mut input = "draft";
        let result = scalar_value.parse_next(&mut input).unwrap();
        assert_eq!(result, "draft".to_string());
    }

    #[test]
    fn parse_scalar_quoted() {
        let mut input = "\"hello world\"";
        let result = scalar_value.parse_next(&mut input).unwrap();
        assert_eq!(result, "hello world".to_string());
    }

    #[test]
    fn parse_scalar_with_colon() {
        let mut input = "ep:EP012";
        let result = scalar_value.parse_next(&mut input).unwrap();
        assert_eq!(result, "ep:EP012".to_string());
    }
}
