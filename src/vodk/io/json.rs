use std::util::{swap};

pub trait TextStream {
    fn next(&mut self) -> Option<char>;
    fn front(&self) -> Option<char>;
    fn empty(&self) -> bool;
}

pub enum Error {
    ERROR_UNKNOWN,
    ERROR_UNTERMINATED_STRING,
    ERROR_UNTERMINATED_JSON,
    ERROR_UNEXPECTED_TOKEN(Token),
}

pub enum Token {
    TOKEN_COLON,
    TOKEN_COMA,
    TOKEN_BEGIN_OBJECT,
    TOKEN_END_OBJECT,
    TOKEN_BEGIN_ARRAY,
    TOKEN_END_ARRAY,
    TOKEN_NULL,
    TOKEN_BOOLEAN(bool),
    TOKEN_NUMBER(f64),
    TOKEN_STRING(~str),
    TOKEN_END,
    TOKEN_ERROR,
}

pub enum Value {
    VALUE_NULL,
    VALUE_BOOLEAN(bool),
    VALUE_NUMBER(f64),
    VALUE_STRING(~str),
}

pub enum NameSpace {
    NAME_STRING(~str),
    NAME_INDEX(uint),
}

impl Clone for NameSpace {
    fn clone(&self) -> NameSpace {
        match *self {
            NAME_INDEX(i) => { return NAME_INDEX(i); }
            NAME_STRING(ref s) => { return NAME_STRING(s.clone()); }
        }
    }
}

type ExpectedToken = int;
static EXPECT_VALUE: ExpectedToken  = 1;
static EXPECT_NAME: ExpectedToken   = 2;
static EXPECT_COMA: ExpectedToken   = 4;
static EXPECT_COLON: ExpectedToken  = 8;
static EXPECT_END: ExpectedToken    = 16;

type ContainerType = int;
static CONTAINER_ARRAY: ContainerType = 1;
static CONTAINER_OBJECT: ContainerType = 2;
static CONTAINER_ROOT: ContainerType = 3;

pub struct ParserState {
    namespace: ~[NameSpace],
    expected: ExpectedToken,
}

impl ParserState {
    pub fn new() -> ParserState {
        return ParserState {
            namespace: ~[],
            expected: EXPECT_VALUE|EXPECT_END,
        }
    }
}

pub trait CustomParser {
    fn on_begin_object(&mut self, namespace: &[NameSpace]) -> bool;
    fn on_end_object(&mut self, namespace: &[NameSpace]) -> bool;
    fn on_begin_array(&mut self, namespace: &[NameSpace]) -> bool;
    fn on_end_array(&mut self, namespace: &[NameSpace]) -> bool;
    fn on_value(&mut self, namespace: &[NameSpace], value: &Value) -> bool;
    fn on_end(&mut self) -> bool;
    fn on_error(&mut self, error: Error);
}

pub fn parse(src: &mut TextStream, parser: &mut CustomParser) {
    let mut state = ParserState::new();
    loop {
        let token = tokenize(src);
        if !parse_step(&token, &mut state, parser) {
            break;
        }
    }
}

pub fn parse_step(token: &Token, state: &mut ParserState, parser: &mut CustomParser) -> bool {
    let container = if state.namespace.len() == 0 { CONTAINER_ROOT }
                    else {
                        match state.namespace[state.namespace.len()-1] {
                            NAME_INDEX(_) => { CONTAINER_ARRAY }
                            NAME_STRING(_) => { CONTAINER_OBJECT }                            
                        }
                    };

    if !is_expected(token, state.expected, container) {
        parser.on_error(ERROR_UNEXPECTED_TOKEN(token.clone()));
        println("unexpected token "+token_to_str(token));
        return false;
    }

    let res: bool;
    match *token {
        TOKEN_BEGIN_OBJECT => {
            res = parser.on_begin_object(state.namespace);
            state.namespace.push(NAME_STRING(~""));
            state.expected = EXPECT_NAME|EXPECT_END;
        }
        TOKEN_END_OBJECT => {
            state.namespace.pop();
            res = parser.on_end_object(state.namespace.slice(0,state.namespace.len()));
            state.expected = if state.namespace.len() == 0 { EXPECT_END }
                             else { EXPECT_COMA|EXPECT_END };
        }
        TOKEN_BEGIN_ARRAY => {
            res =parser.on_begin_array(state.namespace.slice(0,state.namespace.len()));
            state.namespace.push(NAME_INDEX(0));
            state.expected = EXPECT_VALUE|EXPECT_END;
        }
        TOKEN_END_ARRAY => {
            state.namespace.pop();
            res = parser.on_end_array(state.namespace.slice(0,state.namespace.len()));
            state.expected = if state.namespace.len() == 0 { EXPECT_END }
                             else { EXPECT_COMA|EXPECT_END };
        }
        TOKEN_BOOLEAN(b) => {
            res = parser.on_value(state.namespace.slice(0,state.namespace.len()),
                                  &VALUE_BOOLEAN(b));
        }
        TOKEN_NUMBER(n) => {
            res = parser.on_value(state.namespace.slice(0,state.namespace.len()),
                                  &VALUE_NUMBER(n));
        }
        TOKEN_NULL => {
            res = parser.on_value(state.namespace.slice(0,state.namespace.len()),
                                  &VALUE_NULL);
        }
        TOKEN_STRING(ref s) => {
            if state.expected&EXPECT_VALUE != 0 {
                res = parser.on_value(state.namespace.slice(0,state.namespace.len()),
                                      &VALUE_STRING(s.clone()));
                state.expected = if state.namespace.len() == 0 { EXPECT_END }
                                 else { EXPECT_COMA|EXPECT_END };
            } else if state.expected&EXPECT_NAME != 0 {
                state.namespace[state.namespace.len()-1] = NAME_STRING(s.clone());
                state.expected = EXPECT_COLON;
                res = true;
            } else {
                fail!("unexpected string should have been caught already");
                res = false;
            }
        }
        TOKEN_END => {
            if state.namespace.len() > 0 {
                parser.on_error(ERROR_UNTERMINATED_JSON);
            }
            parser.on_end();
            res = false;
        }
        TOKEN_COLON => {
            state.expected = EXPECT_VALUE;
            res = true;
        }
        TOKEN_COMA => {
            match state.namespace[state.namespace.len()-1] {
                NAME_INDEX(ref mut i) => {
                    state.expected = EXPECT_VALUE|EXPECT_END;
                    *i += 1;
                }
                NAME_STRING(ref mut s) => {
                    state.expected = EXPECT_NAME|EXPECT_END;
                }
            }
            res = true;
        }
        TOKEN_ERROR => {
            // right now unterminated strings is the only thing the tokenizer
            // is able to detect
            parser.on_error(ERROR_UNTERMINATED_STRING);
            return false;
        }
    }
    match *token {
        TOKEN_STRING(_) => {}
        TOKEN_BEGIN_ARRAY => {}
        TOKEN_BEGIN_OBJECT => {}
        _ => {
            if is_value(token) {
                state.expected = if state.namespace.len() == 0 { EXPECT_END }
                                 else { EXPECT_COMA|EXPECT_END };
            }
        }
    }
    return res;
}

pub fn tokenize(src: &mut TextStream) -> Token {
    // skip white spaces
    loop {
        match src.front() {
            Some(' ')  => { src.next(); }
            Some('\t') => { src.next(); }
            Some('\n') => { src.next(); }
            Some(_)    => { break; },
            None       => { return TOKEN_END; }
        }
    }

    let mut buffer : ~str = ~"";

    let is_string = match src.front() {
        Some(s) => { s == '\"' },
        None => { false },
    };

    if is_string { src.next(); } // skip the first '"'

    loop {
        if is_string {
            match src.next() {
                Some('\"') => { return TOKEN_STRING(buffer); },
                Some(s) => { buffer.push_char(s); },
                None => { return TOKEN_ERROR; },
            }
        } else {
            if buffer.len() == 0 {
                match src.next() {
                    Some(',')  => return TOKEN_COMA,
                    Some(':')  => return TOKEN_COLON,
                    Some('{')  => return TOKEN_BEGIN_OBJECT,
                    Some('}')  => return TOKEN_END_OBJECT,
                    Some('[')  => return TOKEN_BEGIN_ARRAY,
                    Some(']')  => return TOKEN_END_ARRAY,
                    Some(' ')  => return str_to_token(buffer),
                    Some('\t') => return str_to_token(buffer),
                    Some('\n') => return str_to_token(buffer),
                    Some(s)    => buffer.push_char(s),
                    None       => return  str_to_token(buffer),
                }                
            } else {
                match src.front() {
                    Some(s) => {
                        match s {
                            ',' | ':' | '{' | '}' | '[' | ']' |
                            ' ' | '\t' | '\n' => return str_to_token(buffer),
                            _ => buffer.push_char(s),
                        }
                    },
                    None => return str_to_token(buffer),
                }
                src.next();
            }
        }
    }
}

fn is_expected(token: &Token, expected: ExpectedToken, container: ContainerType) -> bool {
    return match *token {
        TOKEN_END           => { expected&EXPECT_END != 0 && container==CONTAINER_ROOT }
        TOKEN_END_ARRAY     => { expected&EXPECT_END != 0 && container==CONTAINER_ARRAY }
        TOKEN_END_OBJECT    => { expected&EXPECT_END != 0 && container==CONTAINER_OBJECT }
        TOKEN_STRING(_)     => { expected&EXPECT_VALUE != 0 || expected&EXPECT_NAME != 0 }
        TOKEN_BOOLEAN(_)    => { expected&EXPECT_VALUE != 0 }
        TOKEN_NUMBER(_)     => { expected&EXPECT_VALUE != 0 }
        TOKEN_NULL          => { expected&EXPECT_VALUE != 0 }
        TOKEN_BEGIN_ARRAY   => { expected&EXPECT_VALUE != 0 }
        TOKEN_BEGIN_OBJECT  => { expected&EXPECT_VALUE != 0 }
        TOKEN_COMA          => { expected&EXPECT_COMA  != 0 }
        TOKEN_COLON         => { expected&EXPECT_COLON != 0 }
                          _ => { false }
    }
}

fn is_value(token: &Token) -> bool {
    match *token {
        TOKEN_STRING(_)    => true,
        TOKEN_BOOLEAN(_)   => true,
        TOKEN_NUMBER(_)    => true,
        TOKEN_NULL         => true,
        TOKEN_BEGIN_OBJECT => true,
        TOKEN_BEGIN_ARRAY  => true,
        _                  => false,
    }
}


pub fn token_to_str(token: &Token) -> ~str {
    return match *token {
              TOKEN_COLON => ~":",
               TOKEN_COMA => ~",",
       TOKEN_BEGIN_OBJECT => ~"{",
         TOKEN_END_OBJECT => ~"}",
        TOKEN_BEGIN_ARRAY => ~"[",
          TOKEN_END_ARRAY => ~"]",
               TOKEN_NULL => ~"<null>",
                TOKEN_END => ~"<end>",
          TOKEN_NUMBER(f) => format!("{}", f as f64),
      TOKEN_STRING(ref s) => ~"\"" + s.clone() + "\"",
         TOKEN_BOOLEAN(b) => if b {~"<true>"}
                             else {~"<false>"},
           TOKEN_ERROR    =>  ~"<error>",
    }
}

fn str_to_token(src: &str) -> Token {
    //println("str_to_token("+src+")");
    match src {
        "true"  => TOKEN_BOOLEAN(true),
        "false" => TOKEN_BOOLEAN(false),
        "null"  => TOKEN_NULL,
        _       => {
            match (from_str::<f64>(src)) {
                Some(f) => TOKEN_NUMBER(f),
                None    => TOKEN_STRING(src.to_owned()),
            }
        }
    }
}

pub struct Validator {
    error: Option<Error>
}

impl Validator {
    pub fn new() -> Validator { Validator { error: None } }
    pub fn error<'l>(&'l self) -> &'l Option<Error> { &'l self.error }
    pub fn is_valid(&self) -> bool { match self.error { Some(_) => false, None => true } }
}

impl CustomParser for Validator {
    fn on_begin_object(&mut self, _namespace: &[NameSpace]) -> bool { true }
    fn on_end_object(&mut self, _namespace: &[NameSpace]) -> bool { true }
    fn on_begin_array(&mut self, _namespace: &[NameSpace]) -> bool { true }
    fn on_end_array(&mut self, _namespace: &[NameSpace]) -> bool { true }
    fn on_value(&mut self, _namespace: &[NameSpace], _value: &Value) -> bool { true }
    fn on_end(&mut self) -> bool { true }
    fn on_error(&mut self, error: Error) {
        self.error = Some(error);
    }
}

pub fn validate(src: &mut TextStream) -> bool {
    let mut validator = Validator::new();
    parse(src, &mut validator as &mut CustomParser);
    return validator.is_valid();
}

impl Clone for Token {
    fn clone(&self) -> Token {
        match *self {
            TOKEN_COLON => { TOKEN_COLON }
            TOKEN_COMA => { TOKEN_COMA }
            TOKEN_BEGIN_OBJECT => { TOKEN_BEGIN_OBJECT }
            TOKEN_END_OBJECT => { TOKEN_END_OBJECT }
            TOKEN_BEGIN_ARRAY => { TOKEN_BEGIN_ARRAY }
            TOKEN_END_ARRAY => { TOKEN_END_ARRAY }
            TOKEN_NULL => { TOKEN_NULL }
            TOKEN_END => { TOKEN_END }
            TOKEN_ERROR => { TOKEN_ERROR }
            TOKEN_BOOLEAN(b) => { TOKEN_BOOLEAN(b) }
            TOKEN_NUMBER(f) => { TOKEN_NUMBER(f) }
            TOKEN_STRING(ref s) => { TOKEN_STRING(s.clone()) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{validate, TextStream};

    #[test]
    fn test_valid() {
        let mut t1 = ~"{a: 3.14, foo: [1,2,3,4,5], bar: true, baz: {plop:\"hello world! \", hey:null, x: false}}  ";
        assert!(validate(&mut t1 as &mut TextStream));
        assert!(validate(&mut ~" " as &mut TextStream));
        assert!(validate(&mut ~"" as &mut TextStream));
        assert!(validate(&mut ~"null" as &mut TextStream));
        assert!(validate(&mut ~"42" as &mut TextStream));
        assert!(validate(&mut ~"\"text\"" as &mut TextStream));
    }

    #[test]
    fn test_invalid() {
        assert!(!validate(&mut ~"[" as &mut TextStream));
        assert!(!validate(&mut ~"[{}" as &mut TextStream));
        assert!(!validate(&mut ~"\"unterminated string" as &mut TextStream));
    }
}