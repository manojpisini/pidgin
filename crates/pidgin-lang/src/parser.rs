use std::collections::BTreeMap;

use winnow::Parser;

use crate::ast::PgnPacket;
use crate::errors::ParseError;
use crate::lexer::{header_line, field_line};

const MAX_PACKET_BYTES: usize = 1_000_000;
const MAX_FIELDS: usize = 100;
const MAX_FIELD_LENGTH: usize = 10_000;

pub fn parse_packet(input: &str) -> Result<PgnPacket, ParseError> {
    // Strip UTF-8 BOM if present
    let input = input.strip_prefix('\u{FEFF}').unwrap_or(input);

    if input.len() > MAX_PACKET_BYTES {
        return Err(ParseError::PacketTooLarge(input.len()));
    }

    let input = input.trim();
    let mut lines: Vec<&str> = input.lines().collect();

    while lines.last().is_some_and(|l| l.trim().is_empty()) {
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

    for (line_idx, line) in lines[header_idx + 1..].iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if fields.len() >= MAX_FIELDS {
            return Err(ParseError::TooManyFields(MAX_FIELDS));
        }

        let mut field_input = trimmed;
        let (name, value) = field_line
            .parse_next(&mut field_input)
            .map_err(|_| ParseError::InvalidField(header_idx + 1 + line_idx))?;

        if name.len() > MAX_FIELD_LENGTH {
            return Err(ParseError::FieldTooLong(header_idx + 1 + line_idx));
        }

        if fields.contains_key(&name) {
            return Err(ParseError::DuplicateField(name));
        }

        match &value {
            crate::ast::FieldValue::Scalar(s) => {
                if s.len() > MAX_FIELD_LENGTH {
                    return Err(ParseError::FieldTooLong(header_idx + 1 + line_idx));
                }
            }
            crate::ast::FieldValue::List(items) => {
                for item in items {
                    if item.len() > MAX_FIELD_LENGTH {
                        return Err(ParseError::FieldTooLong(header_idx + 1 + line_idx));
                    }
                }
            }
        }

        fields.insert(name, value);
    }

    Ok(PgnPacket {
        directive,
        run_id,
        fields,
    })
}
