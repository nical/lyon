
use super::Stream;
use super::attribute::{Attribute, AttributeId, AttributeValue, RefAttributeValue};
use std::str;
use svgparser;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct StyleParserError;

pub struct StyleTokenizer<'l> {
    tokenizer: svgparser::style::Tokenizer<'l>
}

impl<'l> StyleTokenizer<'l> {
    pub fn new(text: &str) -> StyleTokenizer {
        StyleTokenizer::from_stream(Stream::new(text.as_bytes()))
    }

    pub fn from_stream(stream: Stream) -> StyleTokenizer {
        StyleTokenizer {
            tokenizer: svgparser::style::Tokenizer::new(stream)
        }
    }
}

impl<'l> Iterator for StyleTokenizer<'l> {
    type Item = Result<Attribute, StyleParserError>;
    fn next(&mut self) -> Option<Result<Attribute, StyleParserError>> {
        return match self.tokenizer.next() {
            Some(Ok(svgparser::style::Token::Attribute(name, mut stream))) => {
                Some(parse_attribute(name, &mut stream))
            }
            Some(Err(_)) => { Some(Err(StyleParserError)) }
            None => { None }
            _ => { self.next() }
        };
    }
}

fn parse_attribute(name: &[u8], value: &mut Stream) -> Result<Attribute, StyleParserError> {
    if let Some(attr_name) = AttributeId::from_name(unsafe { str::from_utf8_unchecked(name) }) {
        if let Ok(attr_value) = RefAttributeValue::from_stream(svgparser::ElementId::Style, attr_name, value) {
            return Ok(Attribute { id: attr_name, value: AttributeValue::from_ref(attr_value) });
        }
    }

    return Err(StyleParserError);
}