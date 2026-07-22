use serde::Serialize;

use crate::ast::PgnPacket;

#[derive(Debug, Serialize)]
pub struct TokenReport {
    pub char_count: usize,
    pub estimated_tokens: usize,
    pub line_count: usize,
    pub field_count: usize,
}

pub fn estimate_tokens(text: &str) -> usize {
    text.len().div_ceil(4)
}

pub fn measure_packet(packet: &PgnPacket) -> TokenReport {
    let mut char_count = 0;
    let mut line_count = 0;
    let field_count = packet.fields.len();

    // Header line
    let header = format!("@{} {}", packet.directive.directive_name(), packet.run_id);
    char_count += header.len();
    line_count += 1;

    // Field lines
    for (name, value) in &packet.fields {
        let line = match value {
            crate::ast::FieldValue::Scalar(s) => format!("{}={}", name, s),
            crate::ast::FieldValue::List(items) => format!("{}={}", name, items.join(",")),
        };
        char_count += line.len();
        line_count += 1;
    }

    TokenReport {
        char_count,
        estimated_tokens: char_count.div_ceil(4),
        line_count,
        field_count,
    }
}
