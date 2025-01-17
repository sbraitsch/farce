use serde::Serialize;
use std::collections::HashMap;

pub fn count_chars(input: &str) -> HashMap<char, i32> {
    input.chars().fold(HashMap::new(), |mut acc, c| {
        *acc.entry(c).or_insert(0) += 1;
        acc
    })
}
