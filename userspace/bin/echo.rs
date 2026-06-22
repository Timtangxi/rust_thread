#![no_std]
#![no_main]

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    userlib::println(b"echo: userspace command");
    0
}
