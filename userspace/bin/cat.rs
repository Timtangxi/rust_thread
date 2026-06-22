#![no_std]
#![no_main]

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    let fd = userlib::open(b"/proc/meminfo", 0);
    if fd < 0 {
        userlib::println(b"cat: open failed");
        return 1;
    }
    let mut buf = [0u8; 128];
    loop {
        let n = userlib::read(fd as u32, &mut buf);
        if n <= 0 {
            break;
        }
        let _ = userlib::write(1, &buf[..n as usize]);
    }
    let _ = userlib::close(fd as u32);
    0
}
