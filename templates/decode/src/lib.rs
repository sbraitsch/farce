mod scaffold;

use rand::seq::SliceRandom;
use rand::thread_rng;
use scaffold::decode;
use serde::Serialize;
use std::collections::HashMap;

#[repr(C)]
#[derive(Serialize)]
struct WorkResult {
    success: bool,
    expected: String,
    result: String,
}

#[no_mangle]
pub extern "C" fn run() -> *const i32 {
    let mut letter_map: HashMap<char, char> = HashMap::new();
    let mut shift: Vec<char> = ('a'..='z').collect();
    shift.shuffle(&mut thread_rng());

    for (idx, c) in ('a'..='z').enumerate() {
        letter_map.insert(c, shift[idx]);
    }

    let input = "frontend development sucks";
    let encoded: String = input
        .chars()
        .map(|c| *letter_map.get(&c).unwrap_or(&c))
        .collect();
    let decoded = &decode(&encoded, letter_map);

    let out = WorkResult {
        success: decoded == input,
        expected: input.to_owned(),
        result: decoded.to_owned(),
    };

    let message = serde_json::to_string(&out).unwrap();
    let ptr = message.as_ptr() as i32;
    let length = message.len() as i32;
    std::mem::forget(message);
    let res = Box::new([ptr, length]);
    Box::into_raw(res) as *const i32
}
