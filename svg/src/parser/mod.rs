
pub mod path;
pub mod style;
pub mod attribute;

pub use svgparser::Stream;
pub use svgparser::RgbColor;
pub use svgparser::Length;
pub use svgparser::LengthUnit;
pub use svgparser::ElementId;
pub use svgparser::ValueId;

pub use self::attribute::{
    Attribute, AttributeId, AttributeValue, RefAttributeValue,
};
