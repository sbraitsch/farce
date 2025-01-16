use serde::Serialize;

#[derive(Serialize)]
#[repr(C)]
struct Custom {
    text: String,
    number: i32,
    list: Vec<i32>
}

pub fn execute() -> impl Serialize {
    Custom {
        text: String::from("Hello, World!"),
        number: 42,
        list: vec![-42, 420]
    }
}
