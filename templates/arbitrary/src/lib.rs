mod scaffold;

use scaffold::execute;

#[no_mangle]
pub extern "C" fn run() -> *const i32 {
    let message = serde_json::to_string(&execute()).unwrap();
    let ptr = message.as_ptr() as i32;
    let length = message.len() as i32;
    std::mem::forget(message);
    let res = Box::new([ptr, length]);
    Box::into_raw(res) as *const i32
}
