
use svgparser::{ Tokenize, TextFrame };
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
        StyleTokenizer {
            tokenizer: svgparser::style::Tokenizer::from_str(text)
        }
    }

    pub fn from_frame(frame: TextFrame) -> StyleTokenizer {
        StyleTokenizer {
            tokenizer: svgparser::style::Tokenizer::from_frame(frame)
        }
    }
}

impl<'l> Iterator for StyleTokenizer<'l> {
    type Item = Result<Attribute, StyleParserError>;

    fn next(&mut self) -> Option<Result<Attribute, StyleParserError>> {
        match self.tokenizer.parse_next() {
            Ok(token) => {
                match token {
                    svgparser::style::Token::SvgAttribute(id, value) => {
                        Some(parse_attribute(id, value))
                    }
                    svgparser::style::Token::EndOfStream => {
                        None
                    }
                    _ => self.next(),
                }
            }
            Err(_) => { Some(Err(StyleParserError)) }
        }
    }
}

fn parse_attribute(id: AttributeId, value: TextFrame) -> Result<Attribute, StyleParserError> {
    if let Ok(attr_value) = RefAttributeValue::from_frame(svgparser::ElementId::Rect, id, value) {
        return Ok(Attribute { id: id, value: AttributeValue::from_ref(attr_value) });
    }

    return Err(StyleParserError);
}
