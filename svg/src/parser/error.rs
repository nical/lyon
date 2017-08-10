
use std::fmt;
use svgparser::Error;

/// Errors which can occur when attempting to parse a token.
#[derive(Clone, Debug, PartialEq)]
pub enum ParserError {
    /// Error that occurs when attempting to get the next path token.
    PathToken(Error),
    /// Error that occurs when attempting to get the next style token.
    StyleToken(Error),
    ///Error during a style attribute parsing.
    StyleAttribute(Error),
}

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ParserError::PathToken(ref err) => write!(f, "Path token parsing error : {}", err),
            ParserError::StyleToken(ref err) => write!(f, "Style token parsing error : {}", err),
            ParserError::StyleAttribute(ref err) => write!(f, "Style attribute parsing error : {}", err),
        }
    }
}