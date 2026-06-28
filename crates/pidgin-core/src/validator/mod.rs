pub mod syntax;
pub mod schema;

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub code: String,
    pub message: String,
}
