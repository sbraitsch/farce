mod scaffold;

use scaffold::count_chars;

#[no_mangle]
pub extern "C" fn run(ptr: *const u8, length: i32) -> *const i32 {
    let bytes = unsafe { std::slice::from_raw_parts(ptr, length as usize) };
    let input = String::from_utf8(bytes.to_vec()).unwrap();
    let message = serde_json::to_string(&count_chars(&input)).unwrap();
    let ptr = message.as_ptr() as i32;
    let length = message.len() as i32;
    std::mem::forget(message);
    let res = Box::new([ptr, length]);
    Box::into_raw(res) as *const i32
}
