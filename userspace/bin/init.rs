#![no_std]
#![no_main]

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    userlib::println(b"init: rust no_std userspace");
    let _ = userlib::spawn(b"/bin/shell");
    loop {
        userlib::sleep(100);
    }
}
