extern mod extra;

use json = io::json;

mod io {
    pub mod json;
}
mod util {
    pub mod rand;
}
mod logic {
    pub mod entity;
}

impl json::TextStream for ~str {

    fn next(&mut self) -> Option<char> {
        if self.empty() { return None; }
        return Some(self.shift_char());
    }

    fn front(&mut self) -> Option<char> {
        if self.empty() { return None; }
        return Some(self[0] as char);
    }

    fn empty(&mut self) -> bool {
        return self.len() == 0;
    }
}

struct JSONPrettyPrinter {
    indentation: int,
}

impl JSONPrettyPrinter {
    fn new() -> JSONPrettyPrinter { JSONPrettyPrinter{ indentation: 0 }}
    fn print_indent(&self) {
        for _n in range(0,self.indentation) {
            print("    ");
        }
    }
}

impl json::Handler for JSONPrettyPrinter {
    fn on_begin_object(&mut self, _namespace: &[json::NameSpace]) -> bool {
        self.print_indent();
        println("{");
        self.indentation += 1;
        return true;
    }
    fn on_end_object(&mut self, _namespace: &[json::NameSpace]) -> bool {
        self.indentation -= 1;
        println("");
        self.print_indent();
        print("}");
        return true;
    }
    fn on_begin_array(&mut self, _namespace: &[json::NameSpace]) -> bool {
        self.print_indent();
        println("[");
        self.indentation += 1;
        return true;
    }
    fn on_end_array(&mut self, _namespace: &[json::NameSpace]) -> bool {
        println("");
        self.print_indent();
        print("]");
        return true;
    }
    fn on_value(&mut self, _namespace: &[json::NameSpace], _value: &json::Value) -> bool {
        self.print_indent();
        println("<value>");
        return true;
    }
    fn on_end(&mut self) -> bool {
        self.print_indent();
        println("[end]");
        return true;
    }
    fn on_error(&mut self, _error: json::Error) {
        println("[error]");
    }
}

fn main() {
    let test = ~"{a: 3.14, foo: [1,2,3,4,5], bar: true, baz: {plop:\"hello world! \", hey:null, x: false}}  ";

    let mut prettifier = JSONPrettyPrinter::new();
    let mut validator = json::Validator::new();
    println(test);

    json::parse_with_handler(test.chars(), &mut prettifier as &mut json::Handler);
    json::parse_with_handler(test.chars(), &mut validator as &mut json::Handler);

    match *validator.error() {
        Some(_) => {
            println("validation failed");
        }
        None => {
            println("validation suceeded");
        }
    }
}
