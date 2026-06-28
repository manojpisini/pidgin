use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Directive {
    Run,
    Result,
    Approval,
    Context,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue {
    Scalar(String),
    List(Vec<String>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PgnPacket {
    pub directive: Directive,
    pub run_id: String,
    pub fields: BTreeMap<String, FieldValue>,
}
