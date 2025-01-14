mod submission;

use submission::execute;
use std::collections::HashMap;
use rand::seq::SliceRandom;
use rand::thread_rng;

#[no_mangle]
pub extern "C" fn run() -> *const i32 {
    let mut letter_map: HashMap<char, char> = HashMap::new();
    let mut shift: Vec<char> = ('a'..='z').collect();
    shift.shuffle(&mut thread_rng());

    for (idx, c) in ('a'..='z').enumerate() {
        letter_map.insert(c, shift[idx]);
    }

    let input = "frontend development sucks";
    let encoded: String =  input.chars()
        .map(|c| {
            *letter_map.get(&c).unwrap_or(&c)
        })
        .collect();

    let message = serde_json::to_string(&execute(&encoded, letter_map)).unwrap();
    let ptr = message.as_ptr() as i32;
    let length = message.len() as i32;
    std::mem::forget(message);
    let res = Box::new([ptr, length]);
    Box::into_raw(res) as *const i32
}