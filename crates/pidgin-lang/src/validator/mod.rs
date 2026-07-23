pub mod schema;
/// Validator — syntax and schema validation for parsed Pidgin packets.
///
/// This module contains two sub-validators:
///
/// - **`syntax`**: structural completeness — required fields present, types
///   match expectations (e.g., `in` fields must be lists), field cardinality.
/// - **`schema`**: business-rule validation — workflow exists in registry,
///   mode is in allowed modes, risk level is valid, output names are expected.
///
/// `ValidationError` has an error `code` (for programmatic handling) and a
/// `message` (for human-readable output).
pub mod syntax;

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub code: String,
    pub message: String,
}
