#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("missing @ header line")]
    MissingHeader,

    #[error("unterminated quoted string at byte {0}")]
    UnterminatedString(usize),

    #[error("duplicate field: {0}")]
    DuplicateField(String),

    #[error("malformed list at byte {0}")]
    MalformedList(usize),

    #[error("invalid directive: {0}")]
    InvalidDirective(String),

    #[error("empty run_id")]
    EmptyRunId,
}
