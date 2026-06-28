use pidgin_lang::parser::parse_packet;
use proptest::prelude::*;

proptest! {
    #[test]
    fn parse_random_input_does_not_panic(input: String) {
        let _ = parse_packet(&input);
    }

    #[test]
    fn parse_large_input_does_not_panic(
        header in proptest::string::string_regex(".{0,50}").unwrap(),
        fields in proptest::collection::vec(proptest::string::string_regex(".{0,30}").unwrap(), 0..20),
    ) {
        let mut input = format!("@run {}", header);
        for (i, f) in fields.iter().enumerate() {
            if i % 2 == 0 {
                let _ = format!("{}=", f); // skip, just exercise parse
            }
            input.push_str(&format!("\nfield{}={}", i, f));
        }
        let _ = parse_packet(&input);
    }

    #[test]
    fn parse_malformed_headers_do_not_panic(
        prefix in proptest::string::string_regex(".{0,5}").unwrap(),
        body in proptest::string::string_regex(".{0,100}").unwrap(),
    ) {
        let input = format!("{}run test_run_id\n{}", prefix, body);
        let _ = parse_packet(&input);
    }
}
