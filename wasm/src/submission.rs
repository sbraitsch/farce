use serde::Serialize;
#[derive(Serialize)]
struct Something {
	some: i32,
	thing:i32
}
pub fn execute() -> impl Serialize {
	Something { some: 42, thing: 69 }
}
