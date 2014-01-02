extern mod extra;
use json = io::json;

mod io {
    pub mod json;
}
//mod util {
//    pub mod rand;
//}
//mod logic {
//    pub mod entity;
//}

fn main() {
    let src = ~"
    {
        \"pi\": 3.14,
        \"foo\": [[1],2,3,4,5],
        \"bar\": true,
        \"baz\": {
            \"plop\": \"hello world! \",
            \"hey\": null,
            \"x\": false
        }
    }  ";

    let mut validator = json::Validator::new();
    println(src);

    println(" --------------- ");

    json::parse_with_handler(src.chars(), &mut validator as &mut json::Handler);

    match *validator.get_error() {
        Some(_) => {
            println("validation failed");
        }
        None => {
            println("validation suceeded");
        }
    }

    println(" --------------- ");

    for c in json::writer(json::parse_iter(json::tokenize(src.chars())), "  ", "\n") {
        print(c.to_str());
    }
}
