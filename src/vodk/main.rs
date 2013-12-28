extern mod extra;

use logic::entity::{EntityGroup, EntityID, EntityIndex};
use json = io::json;
use std::rc;
use std::util::{swap};

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

    fn front(&self) -> Option<char> {
        if self.empty() { return None; }
        return Some(self[0] as char);
    }

    fn empty(&self) -> bool {
        return self.len() == 0;
    }
}

struct JSONPrettyPrinter {
    indentation: int,
}

/*

{
    foo: [
        1,
        2,
        3
    ]
}

*/

impl JSONPrettyPrinter {
    fn new() -> JSONPrettyPrinter { JSONPrettyPrinter{ indentation: 0 }}
    fn print_indent(&self) {
        for _n in range(0,self.indentation) {
            print("    ");
        }
    }
}

impl json::CustomParser for JSONPrettyPrinter {
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

//Shows how to borrow a reference in a container
//
//struct Container {
//    elements: ~[int],
//}
//
//impl Container {
//    fn get_mut<'l>(&'l mut self, idx: u32) -> &'l mut int { return &mut self.elements[idx]; }
//    fn get<'l>(&'l self, idx: u32) -> &'l int { return &self.elements[idx]; }
//    fn test() {
//        let mut c = Container { elements: ~[42, 12, 5] };
//        {
//            let mut b0 : &mut int = c.get_mut(0);
//            *b0 = 10;
//        }
//        let b1 = c.get(1);
//        let b2 = c.get(2);
//    }
//}

fn main() {
    let mut test = ~"{a: 3.14, foo: [1,2,3,4,5], bar: true, baz: {plop:\"hello world! \", hey:null, x: false}}  ";
    let mut test2 = test.clone();

    let mut parser = JSONPrettyPrinter::new();
    let mut validator = json::Validator::new();
    println(test);

    json::parse(&mut test   as &mut json::TextStream,
                &mut parser as &mut json::CustomParser);

    json::parse(&mut test2     as &mut json::TextStream,
                &mut validator as &mut json::CustomParser);

    match *validator.error() {
        Some(_) => {
            println("validation failed");
        }
        None => {
            println("validation suceeded");
        }
    }

    println("");

    //let id = EntityID::new(EntityGroup(1), EntityIndex(42));
    //let id_copy = id;
}
