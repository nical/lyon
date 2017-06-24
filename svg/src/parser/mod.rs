
mod path;
mod style;
mod attribute;

pub use svgparser::Color;
pub use svgparser::Length;
pub use svgparser::LengthUnit;
pub use svgparser::ElementId;
pub use svgparser::ValueId;

pub use self::attribute::{
    Attribute, AttributeId, AttributeValue, RefAttributeValue,
};

pub use self::path::{PathTokenizer, build_path, ParserError};
pub use self::style::StyleTokenizer;
