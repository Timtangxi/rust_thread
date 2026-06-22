#![no_std]
#![no_main]

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    let fd = userlib::open(b"/", 0);
    if fd < 0 {
        userlib::println(b"ls: open failed");
        return 1;
    }

    let mut entries = [userlib::DirEntry {
        ino: 0,
        file_type: 0,
        name_len: 0,
        name: [0; userlib::DIR_NAME_LEN],
    }; 4];
    loop {
        let n = userlib::getdents(fd as u32, &mut entries);
        if n <= 0 {
            break;
        }
        let count = n as usize / core::mem::size_of::<userlib::DirEntry>();
        for entry in entries.iter().take(count) {
            let len = entry.name_len as usize;
            let _ = userlib::write(1, &entry.name[..len]);
            let _ = userlib::write(1, b"\n");
        }
    }
    let _ = userlib::close(fd as u32);
    0
}
