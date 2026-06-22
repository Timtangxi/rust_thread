#![no_std]
#![no_main]

unsafe extern "Rust" {
    fn main() -> i32;
}

#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start() -> ! {
    let code = unsafe { main() };
    userlib::exit(code)
}
