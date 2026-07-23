use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum Directive {
    Run,
    Result,
    Approval,
    Context,
}

impl Directive {
    pub fn directive_name(&self) -> &'static str {
        match self {
            Directive::Run => "run",
            Directive::Result => "result",
            Directive::Approval => "approval",
            Directive::Context => "context",
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum FieldValue {
    Scalar(String),
    List(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct PgnPacket {
    pub directive: Directive,
    pub run_id: String,
    pub fields: BTreeMap<String, FieldValue>,
}
