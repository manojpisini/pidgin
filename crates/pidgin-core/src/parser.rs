use std::collections::BTreeMap;

use winnow::Parser;

use crate::ast::PgnPacket;
use crate::errors::ParseError;
use crate::lexer::{header_line, field_line};

pub fn parse_packet(input: &str) -> Result<PgnPacket, ParseError> {
    let input = input.trim();
    let mut lines: Vec<&str> = input.lines().collect();

    while lines.last().map_or(false, |l| l.trim().is_empty()) {
        lines.pop();
    }

    if lines.is_empty() {
        return Err(ParseError::MissingHeader);
    }

    let mut header_idx = None;
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        header_idx = Some(i);
        break;
    }

    let header_idx = header_idx.ok_or(ParseError::MissingHeader)?;
    let header_line_str = lines[header_idx].trim();

    if !header_line_str.starts_with('@') {
        return Err(ParseError::MissingHeader);
    }

    let mut hdr_input = header_line_str;
    let (directive, run_id) = header_line
        .parse_next(&mut hdr_input)
        .map_err(|_| ParseError::MissingHeader)?;

    if run_id.is_empty() {
        return Err(ParseError::EmptyRunId);
    }

    let mut fields = BTreeMap::new();

    for line in &lines[header_idx + 1..] {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let mut field_input = trimmed;
        let (name, value) = field_line
            .parse_next(&mut field_input)
            .map_err(|_| ParseError::MissingHeader)?;

        if fields.contains_key(&name) {
            return Err(ParseError::DuplicateField(name));
        }

        fields.insert(name, value);
    }

    Ok(PgnPacket {
        directive,
        run_id,
        fields,
    })
}
