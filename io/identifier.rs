use std::string::ToString;
use std::hash;
use std::mem;

#[derive(Copy, Show, PartialEq, Eq)]
pub enum ResourceType {
    Image,
    Mesh,
    Shader,
    Material,
    Scene,
    Sound,
    LuaScript,
    Locale,
    PlainText,
}

impl ResourceType {
    pub fn path_name(self) -> &'static str {
        return match self {
            ResourceType::Image => { "images" }
            ResourceType::Mesh => { "meshes" }
            ResourceType::LuaScript => { "scripts" }
            ResourceType::Shader => { "shaders" }
            ResourceType::Material => { "materials" }
            ResourceType::Scene => { "scenes" }
            ResourceType::Sound => { "sounds" }
            ResourceType::Locale => { "strings" }
            ResourceType::PlainText => { "misc" }
        }
    }

    pub fn path_extension(self) -> &'static str {
        return match self {
            ResourceType::Image => { "png" }
            ResourceType::Mesh => { "msh" }
            ResourceType::LuaScript => { "lua" }
            ResourceType::Shader => { "glsl" }
            ResourceType::Material => { "mtl" }
            ResourceType::Scene => { "scn" }
            ResourceType::Sound => { "wav" }
            ResourceType::Locale => { "json" }
            ResourceType::PlainText => { "txt" }
        }
    }
}

#[derive(Copy, Show, PartialEq, Eq)]
pub enum Locale {
    None,
    En,
    Fr,
    Es,
    De,
    Zh,
    Ar,
    // TODO
}

impl Locale {
    pub fn path_name(self) -> &'static  str {
        match self {
            Locale::None => { "" }
            Locale::En => { "en" }
            Locale::Fr => { "fr" }
            Locale::Es => { "es" }
            Locale::De => { "de" }
            Locale::Zh => { "zh" }
            Locale::Ar => { "ar" }
        }
    }
}

#[derive(Copy, Show, PartialEq, Eq)]
pub struct ResourceId {
    pub name: Name,
    pub resource_type: ResourceType,
    pub locale: Locale,
}

impl<H> hash::Hash<H> for ResourceId where H: hash::Hasher, H: hash::Writer {
    fn hash(&self, state: &mut H) {
        unsafe {
            let bytes :*const [u8; 10] = mem::transmute(&self);
            bytes.hash(state);
        }
    }
}

pub fn resource_id(resource_type: ResourceType, name: Name) -> ResourceId {
    ResourceId {
        name: name,
        resource_type: resource_type,
        locale: Locale::None,
    }
}

impl ResourceId {
    fn with_locale(self, locale: Locale) -> ResourceId {
        let mut result = self;
        result.locale = locale;
        return result;
    }
}


impl ToString for ResourceId {
    fn to_string(&self) -> String {
        if self.locale == Locale::None {
            return format!("{}/{}.{}",
                self.resource_type.path_name(),
                self.name.to_string(),
                self.resource_type.path_extension()
            );
        } else {
            return format!("{}/{}/{}.{}",
                self.resource_type.path_name(),
                self.locale.path_name(),
                self.name.to_string(),
                self.resource_type.path_extension()
            );
        }
    }
}

#[derive(Copy, Show, PartialEq, Eq)]
pub struct Name {
    pub hash: u64,
}

#[derive(Copy, Show, PartialEq)]
pub enum NameError {
    InvalidChar(char),
    TooManyCharacters(u32),
}

const MAX_CHARS: usize = 10;
const BITS_PER_CHAR: u64 = 6;
const MASK: u64 = 0b111111;

impl ToString for Name {
    fn to_string(&self) -> String {
        let mut result = String::with_capacity(12);

        for i in range(0, MAX_CHARS as u64) {
            let coded_ch: u8 = ((self.hash >> (i*BITS_PER_CHAR)) & MASK) as u8;
            match coded_ch {
                1 => { result.push('_'); }
                2 => { result.push('-'); }
                3 => { result.push(' '); }
                10 ... 19 => { result.push((('0' as u8) + (coded_ch - 10) as u8) as char); }
                20 ... 46 => { result.push((('a' as u8) + (coded_ch - 20) as u8) as char); }
                _ => { break; }
            }
        }
        return result ;
    }
}

impl Name {
    pub fn from_str(name_str: &str) -> Result<Name, NameError> {
        if name_str.len() > MAX_CHARS {
            return Err(NameError::TooManyCharacters(name_str.len() as u32));
        }
        let mut result = Name { hash: 0 };
        let mut i = 0;
        for c in name_str.chars() {
            match c {
                '_' => {
                    result.hash |= 1 << (i*BITS_PER_CHAR);
                }
                '-' => {
                    result.hash |= 2 << (i*BITS_PER_CHAR);
                }
                ' ' => {
                    result.hash |= 3 << (i*BITS_PER_CHAR);
                }
                '0' ... '9' => {
                    result.hash |= (c as u64 - '0' as u64 + 10) << (i*BITS_PER_CHAR);
                }
                'a' ... 'z' => {
                    result.hash |= (c as u64 - 'a' as u64 + 20) << (i*BITS_PER_CHAR);
                }
                _ => {
                    return Err(NameError::InvalidChar(c));
                }
            }
            i += 1;
        }
        return Ok(result);
    }
}

#[test]
fn name_conv() {
    let a = Name::from_str("a").unwrap();
    let o = Name::from_str("o").unwrap();
    let oo = Name::from_str("oo").unwrap();
    let zero = Name::from_str("0").unwrap();
    let dash = Name::from_str("-").unwrap();
    let underscore = Name::from_str("_").unwrap();
    let underscore2 = Name::from_str("__").unwrap();
    let foo = Name::from_str("foo").unwrap();
    let bar = Name::from_str("bar").unwrap();
    let foo_bar = Name::from_str("foo_bar").unwrap();
    let bar_foo = Name::from_str("bar-foo").unwrap();

    assert_eq!(a.to_string(), format!("a"));
    assert_eq!(o.to_string(), format!("o"));
    assert_eq!(oo.to_string(), format!("oo"));
    assert_eq!(zero.to_string(), format!("0"));
    assert_eq!(underscore.to_string(), format!("_"));
    assert_eq!(dash.to_string(), format!("-"));
    assert_eq!(underscore2.to_string(), format!("__"));
    assert_eq!(foo.to_string(), format!("foo"));
    assert_eq!(bar.to_string(), format!("bar"));
    assert_eq!(foo_bar.to_string(), format!("foo_bar"));
    assert_eq!(bar_foo.to_string(), format!("bar-foo"));
    assert_eq!(Name::from_str("inv@lid"), Err(NameError::InvalidChar('@')));
    assert_eq!(Name::from_str("qwertyuiopasd"), Err(NameError::TooManyCharacters(13)));
}

#[test]
fn resource_path() {
    assert_eq!(
        resource_id(ResourceType::Image, Name::from_str("foo").unwrap()).to_string(),
        format!("images/foo.png")
    );
    assert_eq!(
        resource_id(ResourceType::Shader, Name::from_str("uv").unwrap()).to_string(),
        format!("shaders/uv.glsl")
    );
    assert_eq!(
        resource_id(ResourceType::Material, Name::from_str("skin").unwrap()).to_string(),
        format!("materials/skin.mtl")
    );
    assert_eq!(
        resource_id(ResourceType::Image, Name::from_str("sign_1").unwrap()).with_locale(Locale::Fr).to_string(),
        format!("images/fr/sign_1.png")
    );
}

