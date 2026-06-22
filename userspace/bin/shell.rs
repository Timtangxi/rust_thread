#![no_std]
#![no_main]

const MAX_ARGS: usize = 8;
const MAX_LINE: usize = 160;
const PATH_BUF: usize = 96;

#[unsafe(no_mangle)]
pub fn main() -> i32 {
    let mut buf = [0u8; MAX_LINE];
    userlib::println(b"shell: ready");
    loop {
        userlib::print(b"$ ");
        let n = userlib::read(0, &mut buf);
        if n <= 0 {
            userlib::sleep(10);
            continue;
        }

        let line = trim_line(&mut buf[..n as usize]);
        if line.is_empty() {
            continue;
        }
        if line == b"exit" {
            return 0;
        }

        let mut argv = [core::ptr::null::<u8>(); MAX_ARGS + 1];
        let argc = split_args(line, &mut argv);
        if argc == 0 {
            continue;
        }

        let mut path = [0u8; PATH_BUF];
        let path_len = resolve_path(cstr_len(argv[0]), &mut path);
        if path_len == 0 {
            userlib::println(b"shell: bad command");
            continue;
        }

        let pid = userlib::spawnve(&path[..path_len], &argv[..argc + 1]);
        if pid < 0 {
            userlib::println(b"shell: exec failed");
        }
    }
}

fn trim_line(line: &mut [u8]) -> &mut [u8] {
    let mut len = line.len();
    while len != 0 && matches!(line[len - 1], b'\n' | b'\r' | b' ' | b'\t') {
        len -= 1;
    }
    &mut line[..len]
}

fn split_args(line: &mut [u8], argv: &mut [*const u8; MAX_ARGS + 1]) -> usize {
    let mut argc = 0usize;
    let mut index = 0usize;
    while index < line.len() && argc < MAX_ARGS {
        while index < line.len() && matches!(line[index], b' ' | b'\t') {
            line[index] = 0;
            index += 1;
        }
        if index >= line.len() {
            break;
        }
        argv[argc] = line[index..].as_ptr();
        argc += 1;
        while index < line.len() && !matches!(line[index], b' ' | b'\t') {
            index += 1;
        }
        if index < line.len() {
            line[index] = 0;
            index += 1;
        }
    }
    argv[argc] = core::ptr::null();
    argc
}

fn cstr_len(ptr: *const u8) -> &'static [u8] {
    let mut len = 0usize;
    unsafe {
        while *ptr.add(len) != 0 {
            len += 1;
        }
        core::slice::from_raw_parts(ptr, len)
    }
}

fn resolve_path(command: &[u8], out: &mut [u8; PATH_BUF]) -> usize {
    if command.is_empty() {
        return 0;
    }
    if command[0] == b'/' {
        return copy_path(command, out);
    }
    let prefix = b"/bin/";
    if prefix.len() + command.len() >= out.len() {
        return 0;
    }
    out[..prefix.len()].copy_from_slice(prefix);
    out[prefix.len()..prefix.len() + command.len()].copy_from_slice(command);
    prefix.len() + command.len()
}

fn copy_path(src: &[u8], out: &mut [u8; PATH_BUF]) -> usize {
    if src.len() >= out.len() {
        return 0;
    }
    out[..src.len()].copy_from_slice(src);
    src.len()
}
