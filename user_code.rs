
#[no_mangle]
pub extern "C" fn run() -> *const i32 {
    let message = format!("{:?}", execute());
    let ptr = message.as_ptr() as i32;
    let length = message.len() as i32;
    std::mem::forget(message);
    let res = Box::new([ptr, length]);
    Box::into_raw(res) as *const i32
}

pub fn execute() -> impl std::fmt::Debug {
	vec![3,2,1]
}
