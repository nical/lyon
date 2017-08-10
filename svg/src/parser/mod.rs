
mod path;
mod style;
mod attribute;
mod error;

pub use svgparser::Color;
pub use svgparser::Length;
pub use svgparser::LengthUnit;
pub use svgparser::ElementId;
pub use svgparser::ValueId;

pub use self::attribute::{
    Attribute, AttributeId, AttributeValue, RefAttributeValue,
};
pub use self::error::ParserError;

pub use self::path::{PathTokenizer, build_path};
pub use self::style::StyleTokenizer;
