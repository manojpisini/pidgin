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

#[derive(Debug, Serialize)]
pub struct TokenSavingsReport {
    pub pgn_tokens: usize,
    pub verbose_tokens: usize,
    pub savings_ratio: f32,
    pub savings_pct_display: String,
}

pub fn compare_verbose(pgn_text: &str, verbose_text: &str) -> TokenSavingsReport {
    let pgn_tokens = estimate_tokens(pgn_text);
    let verbose_tokens = estimate_tokens(verbose_text);
    let savings_ratio = if verbose_tokens > 0 {
        1.0 - (pgn_tokens as f32 / verbose_tokens as f32)
    } else {
        0.0
    };
    let savings_pct = (savings_ratio * 100.0).round() as i32;

    TokenSavingsReport {
        pgn_tokens,
        verbose_tokens,
        savings_ratio,
        savings_pct_display: format!("{}%", savings_pct),
    }
}
