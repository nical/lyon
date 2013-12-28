use std::util::{swap};

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
    TOKEN_VALUE(Value),
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

pub struct Tokenizer2<'l, T> {
    priv src: T,
    priv front: char,
    priv finished: bool
}

impl<'l, T: Iterator<char>> Tokenizer2<'l, T> {
    pub fn new<'l, T>(s: T) -> Tokenizer2<'l, T> {
        return Tokenizer2 {
            src: s,
            front: ' ',
            finished: false,
        }
    }

    fn front_char(&self) -> Option<char> { Some(self.front) }

    fn next_char(&mut self) -> Option<char> {
        match self.src.next() {
            Some(c) => { self.front = c; }
            None => { self.finished = true; }
        }
        return self.front_char();
    }

    fn tokenize(&mut self) -> Token {
        // skip white spaces
        loop {
            match self.front_char() {
                Some(' ')  => { self.next_char(); }
                Some('\t') => { self.next_char(); }
                Some('\n') => { self.next_char(); }
                Some(_)    => { break; },
                None       => { return TOKEN_END; }
            }
        }
        let mut buffer : ~str = ~"";
        let is_string = match self.front_char() {
            Some(s) => { s == '\"' },
            None => { false },
        };
        if is_string { self.next_char(); } // skip the first '"'
        loop {
            if is_string {
                match self.next_char() {
                    Some('\"') => { return TOKEN_VALUE(VALUE_STRING(buffer)); },
                    Some(s) => { buffer.push_char(s); },
                    None => { return TOKEN_ERROR; },
                }
            } else {
                if buffer.len() == 0 {
                    match self.next_char() {
                        Some(',')  => return TOKEN_COMA,
                        Some(':')  => return TOKEN_COLON,
                        Some('{')  => return TOKEN_BEGIN_OBJECT,
                        Some('}')  => return TOKEN_END_OBJECT,
                        Some('[')  => return TOKEN_BEGIN_ARRAY,
                        Some(']')  => return TOKEN_END_ARRAY,
                        Some(' ')  => return str_to_token_value(buffer),
                        Some('\t') => return str_to_token_value(buffer),
                        Some('\n') => return str_to_token_value(buffer),
                        Some(s)    => buffer.push_char(s),
                        None       => return  str_to_token_value(buffer),
                    }                
                } else {
                    match self.front_char() {
                        Some(s) => {
                            match s {
                                ',' | ':' | '{' | '}' | '[' | ']' |
                                ' ' | '\t' | '\n' => return str_to_token_value(buffer),
                                _ => buffer.push_char(s),
                            }
                        },
                        None => return str_to_token_value(buffer),
                    }
                    self.next_char();
                }
            }
        }
    }
}

impl<'l, T: Iterator<char>> Iterator<Token> for  Tokenizer2<'l, T> {
    fn next<'l>(&'l mut self) -> Option<Token> {
        if self.finished {
            return None;
        }
        let result = self.tokenize();
        match result {
            TOKEN_END => { self.finished = true; }
            TOKEN_ERROR => { self.finished = true; }
            _ => {}
        }
        return Some(result);
    }


}


pub struct Tokenizer<'l> {
    src: &'l mut TextStream,
    finished: bool
}

impl<'l> Tokenizer<'l> {
    pub fn new<'l>(s: &'l mut TextStream) -> Tokenizer<'l> {
        return Tokenizer {
            src: s,
            finished: false,
        }
    }
}

impl<'l> Iterator<Token> for Tokenizer<'l> {
    fn next<'l>(&'l mut self) -> Option<Token> {
        if self.finished {
            return None;
        }
        let result = tokenize(self.src);
        match result {
            TOKEN_END => { self.finished = true; }
            TOKEN_ERROR => { self.finished = true; }
            _ => {}
        }
        return Some(result);
    }
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
                Some('\"') => { return TOKEN_VALUE(VALUE_STRING(buffer)); },
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
                    Some(' ')  => return str_to_token_value(buffer),
                    Some('\t') => return str_to_token_value(buffer),
                    Some('\n') => return str_to_token_value(buffer),
                    Some(s)    => buffer.push_char(s),
                    None       => return  str_to_token_value(buffer),
                }                
            } else {
                match src.front() {
                    Some(s) => {
                        match s {
                            ',' | ':' | '{' | '}' | '[' | ']' |
                            ' ' | '\t' | '\n' => return str_to_token_value(buffer),
                            _ => buffer.push_char(s),
                        }
                    },
                    None => return str_to_token_value(buffer),
                }
                src.next();
            }
        }
    }
}

pub struct Parser<'l> {
    priv src: &'l mut Iterator<Token>,
    priv namespace: ~[NameSpace],
    priv expected: ExpectedToken,
    priv finished: bool,
}

impl<'l> Parser<'l> {
    pub fn new<'l>(s: &'l mut Iterator<Token>) -> Parser<'l> {
        return Parser {
            src: s,
            namespace: ~[],
            expected: EXPECT_VALUE|EXPECT_END,
            finished: false,
        }
    }

    pub fn namespace<'l>(&'l self) -> &'l [NameSpace] {
        return self.namespace.slice(0,self.namespace.len());
    }

    fn parse<'l>(&'l mut self) -> Token {
        loop {
            let token = match self.src.next() {
                            None => { TOKEN_END }
                            Some(t) => { t }
                        };
            println("Parseer.parse: "+token_to_str(&token));
            let container = if self.namespace.len() == 0 { CONTAINER_ROOT }
                            else {
                                match self.namespace[self.namespace.len()-1] {
                                    NAME_INDEX(_) => { CONTAINER_ARRAY }
                                    NAME_STRING(_) => { CONTAINER_OBJECT }
                                }
                            };
            if !is_expected(&token, self.expected, container) {
                println(format!("unexpected Token (expecting {})", self.expected));
                return TOKEN_ERROR; //ERROR_UNEXPECTED_TOKEN(token.clone());
            }
            let res: bool;
            match token {
                TOKEN_BEGIN_OBJECT => {
                    self.namespace.push(NAME_STRING(~""));
                    self.expected = EXPECT_NAME|EXPECT_END;
                    // TODO: namespace change must apply after returned value
                    return TOKEN_BEGIN_OBJECT;
                }
                TOKEN_END_OBJECT => {
                    self.namespace.pop();
                    self.expected = if self.namespace.len() == 0 { EXPECT_END }
                                     else { EXPECT_COMA|EXPECT_END };
                    return TOKEN_END_OBJECT;
                }
                TOKEN_BEGIN_ARRAY => {
                    //res =parser.on_begin_array(state.namespace.slice(0,state.namespace.len()));
                    self.namespace.push(NAME_INDEX(0));
                    self.expected = EXPECT_VALUE|EXPECT_END;
                    return TOKEN_BEGIN_ARRAY; // TODO
                }
                TOKEN_END_ARRAY => {
                    self.namespace.pop();
                    self.expected = if self.namespace.len() == 0 { EXPECT_END }
                                     else { EXPECT_COMA|EXPECT_END };
                    return TOKEN_END_ARRAY;
                }
                TOKEN_VALUE(ref v) => {
                    match v {
                        &VALUE_STRING(ref s) => {
                            if self.expected&EXPECT_VALUE != 0 {
                                self.expected = if self.namespace.len() == 0 { EXPECT_END }
                                                 else { EXPECT_COMA|EXPECT_END };
                                return TOKEN_VALUE(VALUE_STRING(s.clone()));
                            } else if self.expected&EXPECT_NAME != 0 {
                                self.namespace[self.namespace.len()-1] = NAME_STRING(s.clone());
                                self.expected = EXPECT_COLON;
                            } else {
                                fail!("unexpected string should have been caught already");
                                return TOKEN_ERROR;
                            }
                        }
                        val => {
                            self.expected = if self.namespace.len() == 0 { EXPECT_END }
                                             else { EXPECT_COMA|EXPECT_END };
                            return TOKEN_VALUE((*val).clone());
                        }
                    }
                }
                TOKEN_END => {
                    if self.namespace.len() > 0 {
                        println(format!("error: unexpected end with namspace.len() = {}", self.namespace.len()));
                        return TOKEN_ERROR;// TODO(ERROR_UNTERMINATED_JSON);
                    }
                    return TOKEN_END;
                }
                TOKEN_COLON => {
                    self.expected = EXPECT_VALUE;
                }
                TOKEN_COMA => {
                    match self.namespace[self.namespace.len()-1] {
                        NAME_INDEX(ref mut i) => {
                            self.expected = EXPECT_VALUE|EXPECT_END;
                            *i += 1;
                        }
                        NAME_STRING(ref mut s) => {
                            self.expected = EXPECT_NAME|EXPECT_END;
                        }
                    }
                }
                TOKEN_ERROR => {
                    println("Tokenizer return TOKEN_ERROR");
                    // right now unterminated strings is the only thing the tokenizer
                    // is able to detect
                    return TOKEN_ERROR; // TODO return the error type!
                }
            }
        }
    }
}

impl<'l> Iterator<Token> for Parser<'l> {
    fn next<'l>(&'l mut self) -> Option<Token> {
        println("Parser::next");
        if self.finished {
            println("Parser: finished");
            return None;
        }
        let result = self.parse();
        match result {
            TOKEN_END => { self.finished = true; }
            TOKEN_ERROR => { self.finished = true; }
            _ => {}
        }
        println(" -> Parser: "+token_to_str(&result));
        return Some(result);
    }
}


fn is_expected(token: &Token, expected: ExpectedToken, container: ContainerType) -> bool {
    return match *token {
        TOKEN_VALUE(VALUE_STRING(_)) => { expected&EXPECT_VALUE != 0 || expected&EXPECT_NAME != 0 }
        TOKEN_VALUE(_)      => { expected&EXPECT_VALUE != 0 }
        TOKEN_END           => { expected&EXPECT_END != 0 && container==CONTAINER_ROOT }
        TOKEN_END_ARRAY     => { expected&EXPECT_END != 0 && container==CONTAINER_ARRAY }
        TOKEN_END_OBJECT    => { expected&EXPECT_END != 0 && container==CONTAINER_OBJECT }
        TOKEN_BEGIN_ARRAY   => { expected&EXPECT_VALUE != 0 }
        TOKEN_BEGIN_OBJECT  => { expected&EXPECT_VALUE != 0 }
        TOKEN_COMA          => { expected&EXPECT_COMA  != 0 }
        TOKEN_COLON         => { expected&EXPECT_COLON != 0 }
                          _ => { false }
    }
}

fn is_value(token: &Token) -> bool {
    match *token {
        TOKEN_VALUE(_)     => true,
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
                TOKEN_END => ~"<end>",
       TOKEN_VALUE(ref v) => match *v {
                                VALUE_STRING(ref s) => ~"\"" + s.clone() + "\"",
                                    VALUE_NUMBER(n) => format!("{}", n as f64),
                                         VALUE_NULL => ~"<null>",
                                   VALUE_BOOLEAN(b) => if b {~"<true>"}
                                                       else {~"<false>"},
                             },
              TOKEN_ERROR =>  ~"<error>",
    }
}

fn str_to_token_value(src: &str) -> Token {
    //println("str_to_token("+src+")");
    match src {
        "true"  => TOKEN_VALUE(VALUE_BOOLEAN(true)),
        "false" => TOKEN_VALUE(VALUE_BOOLEAN(false)),
        "null"  => TOKEN_VALUE(VALUE_NULL),
        _       => {
            match (from_str::<f64>(src)) {
                Some(f) => TOKEN_VALUE(VALUE_NUMBER(f)),
                None    => TOKEN_VALUE(VALUE_STRING(src.to_owned())),
            }
        }
    }
}

pub trait Handler {
    fn on_begin_object(&mut self, namespace: &[NameSpace]) -> bool;
    fn on_end_object(&mut self, namespace: &[NameSpace]) -> bool;
    fn on_begin_array(&mut self, namespace: &[NameSpace]) -> bool;
    fn on_end_array(&mut self, namespace: &[NameSpace]) -> bool;
    fn on_value(&mut self, namespace: &[NameSpace], value: &Value) -> bool;
    fn on_end(&mut self) -> bool;
    fn on_error(&mut self, error: Error);
}

pub fn parse_with_handler<T:Iterator<char>>(src: T, handler: &mut Handler) {
    //let mut tokenizer = Tokenizer2::new<'l, T>(src);
    let mut tokenizer = Tokenizer2 {
        src: src,
        front: ' ',
        finished: false,
    };

    let mut parser = Parser::new(&mut tokenizer as &mut Iterator<Token>);
    loop {
        let token = match parser.next() {
                        Some(t) => { t }
                        None => { return; }
                    };
        let status;
        match token {
            TOKEN_BEGIN_OBJECT => {
                status = handler.on_begin_object(parser.namespace());
            }
            TOKEN_END_OBJECT => {
                status = handler.on_end_object(parser.namespace());
            }
            TOKEN_BEGIN_ARRAY => {
                status = handler.on_begin_array(parser.namespace());
            }
            TOKEN_END_ARRAY => {
                status = handler.on_end_array(parser.namespace());
            }
            TOKEN_VALUE(ref v) => {
                status = handler.on_value(parser.namespace(), v);
            }
            TOKEN_END => {
                handler.on_end();
                status = false;
            }
            TOKEN_ERROR => {
                handler.on_error(ERROR_UNKNOWN); // TODO
                status = false;
            }
            ref unexpected => {
                handler.on_error(ERROR_UNEXPECTED_TOKEN(unexpected.clone()));
                status = false;
            }
        }
        if !status {
            return;
        }
    }
}

pub trait TextStream {
    fn next(&mut self) -> Option<char>;
    fn front(&mut self) -> Option<char>;
    fn empty(&mut self) -> bool;
}

struct Adaptor<'l, T> {
    src: T,
    buffer: Option<char>,
}

impl<'l, T> Adaptor<'l, T> {
    fn new<'l>(src: T) -> Adaptor<'l, T> {
        return Adaptor {
            src: src,
            buffer: None,
        }
    }
}

impl<'l, T: Iterator<char>> TextStream for Adaptor<'l, T> {
    fn next(&mut self) -> Option<char> {
        match self.buffer {
            Some(s) => {
                let c = s;
                self.buffer = None;
                return Some(s)
            }
            None => {
                return self.src.next();
            }
        }
    }
    fn front(&mut self) -> Option<char> {
        match self.buffer {
            None => {
                self.buffer = self.src.next()
            }
            _ => {}
        }
            return self.buffer;
    }
    fn empty(&mut self) -> bool {
        match self.front() {
            Some(_) => { false }
            None => { true }
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

impl Handler for Validator {
    fn on_begin_object(&mut self, _namespace: &[NameSpace]) -> bool { true }
    fn on_end_object(&mut self, _namespace: &[NameSpace]) -> bool { true }
    fn on_begin_array(&mut self, _namespace: &[NameSpace]) -> bool { true }
    fn on_end_array(&mut self, _namespace: &[NameSpace]) -> bool { true }
    fn on_value(&mut self, _namespace: &[NameSpace], _value: &Value) -> bool { true }
    fn on_end(&mut self) -> bool { true }
    fn on_error(&mut self, error: Error) {
        println("Validator: Error found");
        self.error = Some(error);
    }
}

pub fn validate<T:Iterator<char>>(src: T) -> bool {
    let mut validator = Validator::new();
    parse_with_handler(src, &mut validator as &mut Handler);
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
            TOKEN_END => { TOKEN_END }
            TOKEN_ERROR => { TOKEN_ERROR }
            TOKEN_VALUE(ref v) => { TOKEN_VALUE(v.clone()) }
        }
    }
}

impl Clone for Value {
    fn clone(&self) -> Value {
        match *self {
            VALUE_STRING(ref s) => VALUE_STRING(s.clone()),
            VALUE_BOOLEAN(b) => VALUE_BOOLEAN(b),
            VALUE_NUMBER(n) => VALUE_NUMBER(n),
            VALUE_NULL => VALUE_NULL,
        }
    }
}

impl Clone for NameSpace {
    fn clone(&self) -> NameSpace {
        match *self {
            NAME_INDEX(i) => { return NAME_INDEX(i); }
            NAME_STRING(ref s) => { return NAME_STRING(s.clone()); }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{validate, TextStream, Adaptor};

    #[test]
    fn test_single_valid() {
        assert!(validate(" ".chars()));
        assert!(validate("".chars()));
        assert!(validate("null".chars()));
        assert!(validate("42".chars()));
        assert!(validate("\"text\"".chars()));
    }

    #[test]
    fn test_simple_valid() {
        assert!(validate("[]".chars()));
        assert!(validate("[1,2,3,4]".chars()));
        assert!(validate("{}".chars()));
        assert!(validate("{foo: null}".chars()));
        assert!(validate("[[[null]]]".chars()));
    }

    #[test]
    fn test_long_valid() {
        let mut t1 = ~"{a: 3.14, foo: [1,2,3,4,5], bar: true, baz: {plop:\"hello world! \", hey:null, x: false}}  ";
        assert!(validate(t1.chars()));
    }

    #[test]
    fn test_invalid() {
        assert!(!validate("[".chars()));
        assert!(!validate("[{}".chars()));
        assert!(!validate("\"unterminated string".chars()));
    }
}