use std::char;
//use unicode::str as unicode_str;
//use unicode::str::Utf16Item;

pub struct Tokenizer<T> {
    stream: T,
    string_buffer: String,
    line: u32,
    column: u32,
    ch: char,
    finished: bool,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ErrorType {
    EndOfStreamWhileParsingScope,
    InvalidCharacter,
    InvalidEscape,
    InvalidNumber,
    InvalidUnicodeCodePoint,
    LoneLeadingSurrogateInHexEscape,
    UnexpectedEndOfHexEscape,
    UnexpectedIdentifier,
    UnmatchedDelimiter,
    EOFWhileParsingString,
    InternalError,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Error {
    pub error: ErrorType,
    pub line: u32,
    pub column: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    Identifier(String),
    StringLiteral(String),
    FloatNumber(f64),
    IntNumber(u64),
    // key words
    Let,
    Function,
    Loop,
    For,
    In,
    Return,
    Break,
    If,
    Else,
    Match,
    Struct,
    Enum,
    Mut,
    Impl,
    This,
    As,
    Module,
    Use,
    Void,
    Uint32Type,
    Int32Type,
    Float32Type,
    ByteType,
    StringType,
    BooleanType,
    // operators and special chars
    CmpEqual,
    CmpNotEqual,
    CmpInferiorEqual,
    CmpSuperiorEqual,
    InPlaceAdd,
    InPlaceSub,
    InPlaceMul,
    InPlaceDiv,
    ShiftRight,
    ShiftLeft,
    NamespaceSeparator,
    FatArrow,
    Arrow,
    Dot,
    DotDot,
    Or,
    And,
    Pipe,
    Amperstand,
    Add,
    Sub,
    Mul,
    Div,
    Comma,
    Semicolon,
    Colon,
    Inferior,
    Superior,
    Interrogation,
    Not,
    Sharp,
    At,
    Exponent,
    Modulo,
    Dollar,
    OpenParenthese,
    CloseParenthese,
    OpenSquareBracket,
    CloseSquareBracket,
    OpenCurlyBracket,
    CloseCurlyBracket,
    //
    EndOfStream,
}

impl<T: Iterator<Item=char>> Tokenizer<T> {

    pub fn new(stream: T) -> Tokenizer<T> {
        Tokenizer {
            stream: stream,
            string_buffer: String::new(),
            ch: ' ',
            line: 0,
            column: 0,
            finished: false,
        }
    }

    pub fn eof(&self) -> bool { self.finished }

    fn bump(&mut self) {
        match self.stream.next() {
            Some(c) => { self.ch = c; }
            None => { self.finished = true; }
        }
        if self.ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
    }

    fn next_char(&mut self) -> Option<char> {
        self.bump();
        return if self.finished { Some(self.ch) }
               else { None };
    }

    fn error(&mut self, err: ErrorType) -> Error {
        self.finished = true;
        Error {
            line: self.line,
            column: self.column,
            error: err,
        }
    }

    fn parse(&mut self) -> Result<Token, Error> {
        self.parse_whitespace();
        if self.finished { return Ok(Token::EndOfStream); }
        match self.ch {
            '0'...'9' => {
                return self.parse_number();
            }
            '-' | '/' | '+' | '=' | '%' | '!' | '>' | '<' |
            '.' | '|' | '?' | ',' | '&' | '*' | ':' | '#' |
            '@' | ';' | '^' | '$' | '{' | '}' | '[' | ']' |
            ')' | '(' => {
                return self.parse_special_chars();
            }
            'a'...'z' | 'A'...'Z' | '_' => {
                self.string_buffer.clear();
                return self.parse_word();
            }
            '\"' => {
                return self.parse_str()
            }
            _ => { return Err(self.error(ErrorType::InvalidCharacter)); }
        }
    }

    fn parse_whitespace(&mut self) {
        while !self.finished && (
              self.ch == ' ' ||
              self.ch == '\n' ||
              self.ch == '\t' ||
              self.ch == '\r') { self.bump(); }
    }

    fn parse_special_chars(&mut self) -> Result<Token, Error> {
        let first = self.ch;
        self.bump();
        return match (first, self.ch)  {
            ('=','=') => { self.bump(); Ok(Token::CmpEqual) }
            ('!','=') => { self.bump(); Ok(Token::CmpNotEqual) }
            ('<','=') => { self.bump(); Ok(Token::CmpInferiorEqual) }
            ('>','=') => { self.bump(); Ok(Token::CmpSuperiorEqual) }
            ('+','=') => { self.bump(); Ok(Token::InPlaceAdd) }
            ('-','=') => { self.bump(); Ok(Token::InPlaceSub) }
            ('*','=') => { self.bump(); Ok(Token::InPlaceMul) }
            ('/','=') => { self.bump(); Ok(Token::InPlaceDiv) }
            (':',':') => { self.bump(); Ok(Token::NamespaceSeparator) }
            ('=','>') => { self.bump(); Ok(Token::FatArrow) }
            ('-','>') => { self.bump(); Ok(Token::Arrow) }
            ('.','.') => { self.bump(); Ok(Token::DotDot) }
            ('|','|') => { self.bump(); Ok(Token::Or) }
            ('&','&') => { self.bump(); Ok(Token::And) }
            ('>','>') => { self.bump(); Ok(Token::ShiftRight) }
            ('<','<') => { self.bump(); Ok(Token::ShiftLeft) }
            ('|', _ ) => Ok(Token::Pipe),
            ('&', _ ) => Ok(Token::Amperstand),
            ('+', _ ) => Ok(Token::Add),
            ('-', _ ) => Ok(Token::Sub),
            ('*', _ ) => Ok(Token::Mul),
            ('/', _ ) => Ok(Token::Div),
            (',', _ ) => Ok(Token::Comma),
            (';', _ ) => Ok(Token::Semicolon),
            (':', _ ) => Ok(Token::Colon),
            ('<', _ ) => Ok(Token::Inferior),
            ('>', _ ) => Ok(Token::Superior),
            ('.', _ ) => Ok(Token::Dot),
            ('?', _ ) => Ok(Token::Interrogation),
            ('!', _ ) => Ok(Token::Not),
            ('#', _ ) => Ok(Token::Sharp),
            ('@', _ ) => Ok(Token::At),
            ('^', _ ) => Ok(Token::Exponent),
            ('%', _ ) => Ok(Token::Modulo),
            ('$', _ ) => Ok(Token::Dollar),
            ('{', _ ) => Ok(Token::OpenCurlyBracket),
            ('}', _ ) => Ok(Token::CloseCurlyBracket),
            ('(', _ ) => Ok(Token::OpenParenthese),
            (')', _ ) => Ok(Token::CloseParenthese),
            ('[', _ ) => Ok(Token::OpenSquareBracket),
            (']', _ ) => Ok(Token::CloseSquareBracket),
            // TODO: := $ ~
            _ => { Err(self.error(ErrorType::InternalError)) }
        }
    }

    fn parse_word(&mut self) -> Result<Token, Error> {
        let mut first_char = true;
        loop {
            match (first_char, self.ch) {
                ( false,  '0'...'9' ) => {
                    self.string_buffer.push(self.ch);
                    self.bump();
                }
                ( _ , 'a'...'z') | (_,'A'...'Z') | (_, '_') => {
                    self.string_buffer.push(self.ch);
                    self.bump();
                }
                (_,_) => { break; }
            }
            first_char = false;
        }

        {
            let word = &self.string_buffer[..];
            if word == "struct" { return Ok(Token::Struct); }
            if word == "enum"   { return Ok(Token::Enum); }
            if word == "let"    { return Ok(Token::Let); }
            if word == "fn"     { return Ok(Token::Function); }
            if word == "return" { return Ok(Token::Return); }
            if word == "loop"   { return Ok(Token::Loop); }
            if word == "for"    { return Ok(Token::For); }
            if word == "in"     { return Ok(Token::In); }
            if word == "break"  { return Ok(Token::Break); }
            if word == "if"     { return Ok(Token::If); }
            if word == "else"   { return Ok(Token::Else); }
            if word == "match"  { return Ok(Token::Match); }
            if word == "for"    { return Ok(Token::For); }
            if word == "mod"    { return Ok(Token::Module); }
            if word == "use"    { return Ok(Token::Use); }
            if word == "mut"    { return Ok(Token::Mut); }
            if word == "impl"   { return Ok(Token::Impl); }
            if word == "self"   { return Ok(Token::This); }
            if word == "as"     { return Ok(Token::As); }
            if word == "void"   { return Ok(Token::Void); }
            if word == "usize32" { return Ok(Token::Uint32Type); }
            if word == "int32"  { return Ok(Token::Int32Type); }
            if word == "float32"{ return Ok(Token::Float32Type); }
            if word == "byte"   { return Ok(Token::ByteType); }
            if word == "bool"   { return Ok(Token::BooleanType); }
            if word == "str"    { return Ok(Token::StringType); }
        }

        return Ok(Token::Identifier(self.string_buffer.clone()));
    }

    fn parse_number(&mut self) -> Result<Token, Error> {
        let res = self.parse_integer();
        let mut fres = res as f64;
        let mut is_int = true;

        if self.ch == '.' {
            fres = self.parse_decimal(fres);
            is_int = false;
        }

        if self.ch == 'e' || self.ch == 'E' {
            fres = try!(self.parse_exponent(fres));
            is_int = false;
        }

        return if is_int { Ok(Token::IntNumber(res)) } else { Ok(Token::FloatNumber(fres)) };
    }

    fn parse_integer(&mut self) -> u64 {
        let mut res = 0;

        while !self.eof() {
            match self.ch {
                c @ '0' ... '9' => {
                    res *= 10;
                    res += (c as u64) - ('0' as u64);
                    self.bump();
                }
                _ => break,
            }
        }

        return res
    }

    fn parse_decimal(&mut self, mut res: f64) -> f64 {
        self.bump();

        let mut dec = 1.0;
        while !self.eof() {
            match self.ch {
                c @ '0' ... '9' => {
                    dec /= 10.0;
                    res += (((c as isize) - ('0' as isize)) as f64) * dec;
                    self.bump();
                }
                _ => break,
            }
        }

        return res;
    }

    fn parse_exponent(&mut self, mut res: f64) -> Result<f64, Error> {
        self.bump();

        let mut exp = 0;
        let mut neg_exp = false;

        if self.ch == '+' {
            self.bump();
        } else if self.ch == '-' {
            self.bump();
            neg_exp = true;
        }

        // Make sure a digit follows the exponent place.
        match self.ch {
            '0'...'9' => (),
            _ => return Err(self.error(ErrorType::InvalidNumber))
        }
        while !self.eof() {
            match self.ch {
                c @ '0'...'9' => {
                    exp *= 10;
                    exp += (c as usize) - ('0' as usize);

                    self.bump();
                }
                _ => break
            }
        }

        let exp = 10_f64.powi(exp as i32);
        if neg_exp {
            res /= exp;
        } else {
            res *= exp;
        }

        Ok(res)
    }

    fn decode_hex_escape(&mut self) -> Result<u16, Error> {
        let mut i = 0;
        let mut n = 0u16;
        while i < 4 && !self.eof() {
            self.bump();
            n = match self.ch {
                c @ '0'...'9' => n * 16 + ((c as u16) - ('0' as u16)),
                'a' | 'A' => n * 16 + 10,
                'b' | 'B' => n * 16 + 11,
                'c' | 'C' => n * 16 + 12,
                'd' | 'D' => n * 16 + 13,
                'e' | 'E' => n * 16 + 14,
                'f' | 'F' => n * 16 + 15,
                _ => return Err(self.error(ErrorType::InvalidEscape))
            };

            i += 1;
        }

        // Error out if we didn't parse 4 digits.
        if i != 4 {
            return Err(self.error(ErrorType::InvalidEscape));
        }

        Ok(n)
    }

    fn parse_str(&mut self) -> Result<Token, Error> {
        let mut escape = false;
        let mut res = String::new();

        loop {
            self.bump();
            if self.eof() {
                return Err(self.error(ErrorType::EOFWhileParsingString));
            }

            if escape {
                match self.ch {
                    '"' => res.push('"'),
                    '\\' => res.push('\\'),
                    '/' => res.push('/'),
                    'b' => res.push('\x08'),
                    'f' => res.push('\x0c'),
                    'n' => res.push('\n'),
                    'r' => res.push('\r'),
                    't' => res.push('\t'),
//                    'u' => match try!(self.decode_hex_escape()) {
//                        0xDC00...0xDFFF => return Err(self.error(ErrorType::LoneLeadingSurrogateInHexEscape)),
//
//                        // Non-BMP characters are encoded as a sequence of
//                        // two hex escapes, representing UTF-16 surrogates.
//                        n1 @ 0xD800...0xDBFF => {
//                            match (self.next_char(), self.next_char()) {
//                                (Some('\\'), Some('u')) => (),
//                                _ => return Err(self.error(ErrorType::UnexpectedEndOfHexEscape)),
//                            }
//
//                            let buf = [n1, try!(self.decode_hex_escape())];
//                             match unicode_str::utf16_items(&buf).next() {
//                                Some(Utf16Item::ScalarValue(c)) => res.push(c),
//                                _ => return Err(self.error(ErrorType::LoneLeadingSurrogateInHexEscape)),
//                            }
//                        }
//
//                        n => match char::from_u32(n as u32) {
//                            Some(c) => res.push(c),
//                            None => return Err(self.error(ErrorType::InvalidUnicodeCodePoint)),
//                        },
//                    },
                    _ => return Err(self.error(ErrorType::InvalidEscape)),
                }
                escape = false;
            } else if self.ch == '\\' {
                escape = true;
            } else {
                match self.ch {
                    '"' => {
                        self.bump();
                        return Ok(Token::StringLiteral(res));
                    },
                    c => res.push(c),
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn tokenize_simple() {
        let src = "struct foo { bar: isize32, baz: bool }";
        let mut tok = Tokenizer::new(src.chars());
        assert_eq!(tok.parse(), Ok(Struct));
        assert_eq!(tok.parse(), Ok(Identifier(String::from_str("foo"))));
        assert_eq!(tok.parse(), Ok(OpenCurlyBracket));
        assert_eq!(tok.parse(), Ok(Identifier(String::from_str("bar"))));
        assert_eq!(tok.parse(), Ok(Colon));
        assert_eq!(tok.parse(), Ok(Int32Type));
        assert_eq!(tok.parse(), Ok(Comma));
        assert_eq!(tok.parse(), Ok(Identifier(String::from_str("baz"))));
        assert_eq!(tok.parse(), Ok(Colon));
        assert_eq!(tok.parse(), Ok(BooleanType));
        assert_eq!(tok.parse(), Ok(CloseCurlyBracket));
        assert_eq!(tok.parse(), Ok(EndOfStream));
        assert!(tok.eof());

        let src = "
        fn add(a: float32, b: float32) -> float32 {
            return a + b;
        }
        ";
        let mut tok = Tokenizer::new(src.chars());
        assert_eq!(tok.parse(), Ok(Function));
        assert_eq!(tok.parse(), Ok(Identifier(String::from_str("add"))));
        assert_eq!(tok.parse(), Ok(OpenParenthese));
        assert_eq!(tok.parse(), Ok(Identifier(String::from_str("a"))));
        assert_eq!(tok.parse(), Ok(Colon));
        assert_eq!(tok.parse(), Ok(Float32Type));
        assert_eq!(tok.parse(), Ok(Comma));
        assert_eq!(tok.parse(), Ok(Identifier(String::from_str("b"))));
        assert_eq!(tok.parse(), Ok(Colon));
        assert_eq!(tok.parse(), Ok(Float32Type));
        assert_eq!(tok.parse(), Ok(CloseParenthese));
        assert_eq!(tok.parse(), Ok(Arrow));
        assert_eq!(tok.parse(), Ok(Float32Type));
        assert_eq!(tok.parse(), Ok(OpenCurlyBracket));
        assert_eq!(tok.parse(), Ok(Return));
        assert_eq!(tok.parse(), Ok(Identifier(String::from_str("a"))));
        assert_eq!(tok.parse(), Ok(Add));
        assert_eq!(tok.parse(), Ok(Identifier(String::from_str("b"))));
        assert_eq!(tok.parse(), Ok(Semicolon));
        assert_eq!(tok.parse(), Ok(CloseCurlyBracket));
        assert_eq!(tok.parse(), Ok(EndOfStream));
        assert!(tok.eof());
    }
}
