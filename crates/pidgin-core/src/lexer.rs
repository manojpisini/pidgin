use winnow::ascii::multispace0;
use winnow::combinator::alt;
use winnow::prelude::*;
use winnow::token::{take_till, take_while};

use crate::ast::{Directive, FieldValue};

pub fn ident(input: &mut &str) -> ModalResult<String> {
    take_while(1.., |c: char| c.is_ascii_alphanumeric() || c == '_')
        .map(|s: &str| s.to_string())
        .parse_next(input)
}

pub fn bare_word(input: &mut &str) -> ModalResult<String> {
    take_while(1.., |c: char| {
        c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == ':' || c == '.'
    })
    .map(|s: &str| s.to_string())
    .parse_next(input)
}

pub fn quoted_string(input: &mut &str) -> ModalResult<String> {
    ('"', take_till(0.., |c: char| c == '"'), '"')
        .map(|(_lq, s, _rq): (_, &str, _)| s.to_string())
        .parse_next(input)
}

pub fn scalar_value(input: &mut &str) -> ModalResult<String> {
    alt((quoted_string, bare_word)).parse_next(input)
}

pub fn list_value(input: &mut &str) -> ModalResult<Vec<String>> {
    ('[', multispace0).parse_next(input)?;

    let mut items = Vec::new();
    if input.starts_with(']') {
        ']'.parse_next(input)?;
        return Ok(items);
    }

    items.push(scalar_value.parse_next(input)?);

    loop {
        multispace0.parse_next(input)?;
        if input.starts_with(',') {
            ','.parse_next(input)?;
            multispace0.parse_next(input)?;
            items.push(scalar_value.parse_next(input)?);
        } else {
            break;
        }
    }

    ']'.parse_next(input)?;
    Ok(items)
}

pub fn field_value(input: &mut &str) -> ModalResult<FieldValue> {
    alt((
        list_value.map(FieldValue::List),
        scalar_value.map(FieldValue::Scalar),
    ))
    .parse_next(input)
}

pub fn header_line(input: &mut &str) -> ModalResult<(Directive, String)> {
    '@'.parse_next(input)?;

    let directive_str: String = take_while(1.., |c: char| c.is_ascii_alphabetic())
        .map(|s: &str| s.to_string())
        .parse_next(input)?;

    let directive = match directive_str.as_str() {
        "run" => Directive::Run,
        "result" => Directive::Result,
        "approval" => Directive::Approval,
        "context" => Directive::Context,
        _ => return Err(winnow::error::ErrMode::Backtrack(winnow::error::ContextError::new())),
    };

    multispace0.parse_next(input)?;

    let run_id: String = take_while(1.., |c: char| {
        c.is_ascii_alphanumeric() || c == '_' || c == '.'
    })
    .map(|s: &str| s.to_string())
    .parse_next(input)?;

    Ok((directive, run_id))
}

pub fn field_line(input: &mut &str) -> ModalResult<(String, FieldValue)> {
    let name = ident.parse_next(input)?;
    '='.parse_next(input)?;
    let value = field_value.parse_next(input)?;
    Ok((name, value))
}

pub fn comment_line(input: &mut &str) -> ModalResult<()> {
    '#'.parse_next(input)?;
    take_till(0.., |c: char| c == '\n').parse_next(input)?;
    Ok(())
}
