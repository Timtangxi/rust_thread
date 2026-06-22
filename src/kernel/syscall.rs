#![allow(dead_code)]

pub const KERNEL_ABI_MAGIC: u32 = 0x8000_0000;

pub const SYS_YIELD: u32 = 0;
pub const SYS_SLEEP: u32 = 1;
pub const SYS_EXIT: u32 = 2;
pub const SYS_BLOCK: u32 = 3;
pub const SYS_WAKE: u32 = 4;
pub const SYS_WRITE: u32 = 5;
pub const SYS_READ: u32 = 6;
pub const SYS_OPEN: u32 = 7;
pub const SYS_CLOSE: u32 = 8;
pub const SYS_WAIT: u32 = 9;
pub const SYS_SPAWN: u32 = 10;
pub const SYS_EXEC: u32 = 11;
pub const SYS_LSEEK: u32 = 12;
pub const SYS_STAT: u32 = 13;
pub const SYS_GETDENTS: u32 = 14;
pub const SYS_MKDIR: u32 = 15;
pub const SYS_UNLINK: u32 = 16;
pub const SYS_RENAME: u32 = 17;
pub const SYS_PIPE: u32 = 18;
pub const SYS_BRK: u32 = 19;
pub const SYS_MMAP: u32 = 20;
pub const SYS_MUNMAP: u32 = 21;
pub const SYS_MPROTECT: u32 = 22;
pub const SYS_READV: u32 = 23;
pub const SYS_WRITEV: u32 = 24;
pub const SYS_FCNTL: u32 = 25;
pub const SYS_IOCTL: u32 = 26;
pub const SYS_ACCESS: u32 = 27;
pub const SYS_FSTAT: u32 = 28;
pub const SYS_NEWFSTATAT: u32 = 29;
pub const SYS_GETPID: u32 = 30;
pub const SYS_GETPPID: u32 = 31;
pub const SYS_UNAME: u32 = 32;
pub const SYS_GETTIMEOFDAY: u32 = 33;
pub const SYS_CLOCK_GETTIME: u32 = 34;
pub const SYS_EXECVE: u32 = 35;
pub const SYS_WAIT4: u32 = 36;
pub const SYS_DUP: u32 = 37;
pub const SYS_DUP2: u32 = 38;
pub const SYS_DUP3: u32 = 39;
pub const SYS_PIPE2: u32 = 40;
pub const SYS_GETCWD: u32 = 41;
pub const SYS_CHDIR: u32 = 42;
pub const SYS_READLINK: u32 = 43;
pub const SYS_POLL: u32 = 44;
pub const SYS_SELECT: u32 = 45;
pub const SYS_RT_SIGACTION: u32 = 46;
pub const SYS_RT_SIGPROCMASK: u32 = 47;

pub const LINUX_SYS_EXIT: u32 = 1;
pub const LINUX_SYS_RESTART_SYSCALL: u32 = 0;
pub const LINUX_SYS_FORK: u32 = 2;
pub const LINUX_SYS_READ: u32 = 3;
pub const LINUX_SYS_WRITE: u32 = 4;
pub const LINUX_SYS_OPEN: u32 = 5;
pub const LINUX_SYS_CLOSE: u32 = 6;
pub const LINUX_SYS_WAITPID: u32 = 7;
pub const LINUX_SYS_CREAT: u32 = 8;
pub const LINUX_SYS_LINK: u32 = 9;
pub const LINUX_SYS_UNLINK: u32 = 10;
pub const LINUX_SYS_EXECVE: u32 = 11;
pub const LINUX_SYS_CHDIR: u32 = 12;
pub const LINUX_SYS_MKNOD: u32 = 14;
pub const LINUX_SYS_LSEEK: u32 = 19;
pub const LINUX_SYS_MOUNT: u32 = 21;
pub const LINUX_SYS_UMOUNT: u32 = 22;
pub const LINUX_SYS_GETPID: u32 = 20;
pub const LINUX_SYS_RENAME: u32 = 38;
pub const LINUX_SYS_MKDIR: u32 = 39;
pub const LINUX_SYS_RMDIR: u32 = 40;
pub const LINUX_SYS_KILL: u32 = 37;
pub const LINUX_SYS_TIMES: u32 = 43;
pub const LINUX_SYS_GETUID: u32 = 24;
pub const LINUX_SYS_ACCESS: u32 = 33;
pub const LINUX_SYS_DUP: u32 = 41;
pub const LINUX_SYS_PIPE: u32 = 42;
pub const LINUX_SYS_BRK: u32 = 45;
pub const LINUX_SYS_GETGID: u32 = 47;
pub const LINUX_SYS_GETEUID: u32 = 49;
pub const LINUX_SYS_GETEGID: u32 = 50;
pub const LINUX_SYS_IOCTL: u32 = 54;
pub const LINUX_SYS_FCNTL: u32 = 55;
pub const LINUX_SYS_UMASK: u32 = 60;
pub const LINUX_SYS_DUP2: u32 = 63;
pub const LINUX_SYS_GETPPID: u32 = 64;
pub const LINUX_SYS_GETRUSAGE: u32 = 77;
pub const LINUX_SYS_GETTIMEOFDAY: u32 = 78;
pub const LINUX_SYS_GETGROUPS: u32 = 80;
pub const LINUX_SYS_SETGROUPS: u32 = 81;
pub const LINUX_SYS_SELECT: u32 = 82;
pub const LINUX_SYS_READLINK: u32 = 85;
pub const LINUX_SYS_MMAP: u32 = 90;
pub const LINUX_SYS_MUNMAP: u32 = 91;
pub const LINUX_SYS_TRUNCATE: u32 = 92;
pub const LINUX_SYS_FTRUNCATE: u32 = 93;
pub const LINUX_SYS_STATFS: u32 = 99;
pub const LINUX_SYS_FSTATFS: u32 = 100;
pub const LINUX_SYS_STAT: u32 = 106;
pub const LINUX_SYS_LSTAT: u32 = 107;
pub const LINUX_SYS_FSTAT: u32 = 108;
pub const LINUX_SYS_SYSINFO: u32 = 116;
pub const LINUX_SYS_FSYNC: u32 = 118;
pub const LINUX_SYS_WAIT4: u32 = 114;
pub const LINUX_SYS_CLONE: u32 = 120;
pub const LINUX_SYS_UNAME: u32 = 122;
pub const LINUX_SYS_MPROTECT: u32 = 125;
pub const LINUX_SYS_GETRLIMIT: u32 = 76;
pub const LINUX_SYS_LLSEEK: u32 = 140;
pub const LINUX_SYS_FLOCK: u32 = 143;
pub const LINUX_SYS_MSYNC: u32 = 144;
pub const LINUX_SYS_FDATASYNC: u32 = 148;
pub const LINUX_SYS_SCHED_YIELD: u32 = 158;
pub const LINUX_SYS_NANOSLEEP: u32 = 162;
pub const LINUX_SYS_PRCTL: u32 = 172;
pub const LINUX_SYS_MADVISE: u32 = 220;
pub const LINUX_SYS_GETDENTS: u32 = 141;
pub const LINUX_SYS_NEWSELECT: u32 = 142;
pub const LINUX_SYS_READV: u32 = 145;
pub const LINUX_SYS_WRITEV: u32 = 146;
pub const LINUX_SYS_POLL: u32 = 168;
pub const LINUX_SYS_RT_SIGRETURN: u32 = 173;
pub const LINUX_SYS_RT_SIGACTION: u32 = 174;
pub const LINUX_SYS_RT_SIGPROCMASK: u32 = 175;
pub const LINUX_SYS_GETCWD: u32 = 183;
pub const LINUX_SYS_VFORK: u32 = 190;
pub const LINUX_SYS_UGETRLIMIT: u32 = 191;
pub const LINUX_SYS_MMAP2: u32 = 192;
pub const LINUX_SYS_STAT64: u32 = 195;
pub const LINUX_SYS_LSTAT64: u32 = 196;
pub const LINUX_SYS_FSTAT64: u32 = 197;
pub const LINUX_SYS_GETUID32: u32 = 199;
pub const LINUX_SYS_GETGID32: u32 = 200;
pub const LINUX_SYS_GETEUID32: u32 = 201;
pub const LINUX_SYS_GETEGID32: u32 = 202;
pub const LINUX_SYS_GETGROUPS32: u32 = 205;
pub const LINUX_SYS_SETGROUPS32: u32 = 206;
pub const LINUX_SYS_GETDENTS64: u32 = 217;
pub const LINUX_SYS_FCNTL64: u32 = 221;
pub const LINUX_SYS_GETTID: u32 = 224;
pub const LINUX_SYS_EXIT_GROUP: u32 = 248;
pub const LINUX_SYS_SET_TID_ADDRESS: u32 = 256;
pub const LINUX_SYS_FUTEX: u32 = 240;
pub const LINUX_SYS_CLOCK_GETTIME: u32 = 263;
pub const LINUX_SYS_CLOCK_GETRES: u32 = 264;
pub const LINUX_SYS_CLOCK_NANOSLEEP: u32 = 265;
pub const LINUX_SYS_STATFS64: u32 = 266;
pub const LINUX_SYS_FSTATFS64: u32 = 267;
pub const LINUX_SYS_TGKILL: u32 = 268;
pub const LINUX_SYS_SOCKET: u32 = 281;
pub const LINUX_SYS_BIND: u32 = 282;
pub const LINUX_SYS_CONNECT: u32 = 283;
pub const LINUX_SYS_LISTEN: u32 = 284;
pub const LINUX_SYS_ACCEPT: u32 = 285;
pub const LINUX_SYS_GETSOCKNAME: u32 = 286;
pub const LINUX_SYS_GETPEERNAME: u32 = 287;
pub const LINUX_SYS_SOCKETPAIR: u32 = 288;
pub const LINUX_SYS_SEND: u32 = 289;
pub const LINUX_SYS_SENDTO: u32 = 290;
pub const LINUX_SYS_RECV: u32 = 291;
pub const LINUX_SYS_RECVFROM: u32 = 292;
pub const LINUX_SYS_SHUTDOWN: u32 = 293;
pub const LINUX_SYS_SETSOCKOPT: u32 = 294;
pub const LINUX_SYS_GETSOCKOPT: u32 = 295;
pub const LINUX_SYS_SENDMSG: u32 = 296;
pub const LINUX_SYS_RECVMSG: u32 = 297;
pub const LINUX_SYS_SET_ROBUST_LIST: u32 = 338;
pub const LINUX_SYS_GET_ROBUST_LIST: u32 = 339;
pub const LINUX_SYS_OPENAT: u32 = 322;
pub const LINUX_SYS_MKDIRAT: u32 = 323;
pub const LINUX_SYS_MKNODAT: u32 = 324;
pub const LINUX_SYS_FCHOWNAT: u32 = 325;
pub const LINUX_SYS_FSTATAT64: u32 = 327;
pub const LINUX_SYS_UNLINKAT: u32 = 328;
pub const LINUX_SYS_RENAMEAT: u32 = 329;
pub const LINUX_SYS_LINKAT: u32 = 330;
pub const LINUX_SYS_FCHMODAT: u32 = 333;
pub const LINUX_SYS_FACCESSAT: u32 = 334;
pub const LINUX_SYS_READLINKAT: u32 = 332;
pub const LINUX_SYS_PSELECT6: u32 = 335;
pub const LINUX_SYS_PPOLL: u32 = 336;
pub const LINUX_SYS_DUP3: u32 = 358;
pub const LINUX_SYS_PIPE2: u32 = 359;
pub const LINUX_SYS_ACCEPT4: u32 = 366;
pub const LINUX_SYS_RENAMEAT2: u32 = 382;
pub const LINUX_SYS_GETRANDOM: u32 = 384;
pub const LINUX_SYS_STATX: u32 = 397;
pub const LINUX_SYS_CLOCK_GETTIME64: u32 = 403;
pub const LINUX_SYS_CLOCK_GETRES_TIME64: u32 = 406;
pub const LINUX_SYS_CLOCK_NANOSLEEP_TIME64: u32 = 407;
pub const ARM_SYSCALL_BASE: u32 = 0x0f0000;
pub const ARM_SYS_SET_TLS: u32 = ARM_SYSCALL_BASE + 5;

pub const EINVAL: u32 = (-22i32) as u32;
pub const EPERM: u32 = (-1i32) as u32;
pub const EFAULT: u32 = (-14i32) as u32;
pub const EBADF: u32 = (-9i32) as u32;
pub const ENOENT: u32 = (-2i32) as u32;
pub const ENOMEM: u32 = (-12i32) as u32;
pub const ECHILD: u32 = (-10i32) as u32;
pub const EAGAIN: u32 = (-11i32) as u32;
pub const EISDIR: u32 = (-21i32) as u32;
#[allow(dead_code)]
pub const ENOTDIR: u32 = (-20i32) as u32;
pub const ENOEXEC: u32 = (-8i32) as u32;
pub const EMFILE: u32 = (-24i32) as u32;
pub const EEXIST: u32 = (-17i32) as u32;
pub const ENOSPC: u32 = (-28i32) as u32;
pub const EROFS: u32 = (-30i32) as u32;
pub const ENOTEMPTY: u32 = (-39i32) as u32;
pub const ESPIPE: u32 = (-29i32) as u32;
pub const ENOSYS: u32 = (-38i32) as u32;
pub const EACCES: u32 = (-13i32) as u32;
pub const ENOTTY: u32 = (-25i32) as u32;
pub const E2BIG: u32 = (-7i32) as u32;
pub const EINTR: u32 = (-4i32) as u32;
pub const EOVERFLOW: u32 = (-75i32) as u32;
pub const ERANGE: u32 = (-34i32) as u32;
pub const EIO: u32 = (-5i32) as u32;
pub const EAFNOSUPPORT: u32 = (-97i32) as u32;
pub const ESRCH: u32 = (-3i32) as u32;
#[cfg(feature = "mmu")]
pub const PROT_READ: u32 = 1 << 0;
#[cfg(feature = "mmu")]
pub const PROT_WRITE: u32 = 1 << 1;
#[cfg(feature = "mmu")]
pub const PROT_EXEC: u32 = 1 << 2;

#[allow(dead_code)]
pub const MAP_PRIVATE: u32 = 1 << 1;
#[cfg(feature = "mmu")]
pub const MAP_FIXED: u32 = 1 << 4;
#[cfg(feature = "mmu")]
pub const MAP_ANONYMOUS: u32 = 1 << 5;

#[allow(dead_code)]
pub const O_RDONLY: u32 = 0;
#[allow(dead_code)]
pub const O_WRONLY: u32 = 1;
#[allow(dead_code)]
pub const O_RDWR: u32 = 2;
#[allow(dead_code)]
pub const O_ACCMODE: u32 = 3;
pub const O_CREAT: u32 = 0o100;
pub const O_EXCL: u32 = 0o200;
pub const O_TRUNC: u32 = 0o1000;
pub const O_APPEND: u32 = 0o2000;
pub const O_NONBLOCK: u32 = 0o4000;
pub const O_DIRECTORY: u32 = 0o40000;
pub const O_NOFOLLOW: u32 = 0o100000;
pub const O_LARGEFILE: u32 = 0o400000;
pub const O_CLOEXEC: u32 = 0o2000000;

pub const F_DUPFD: u32 = 0;
pub const F_GETFD: u32 = 1;
pub const F_SETFD: u32 = 2;
pub const F_GETFL: u32 = 3;
pub const F_SETFL: u32 = 4;
pub const F_DUPFD_CLOEXEC: u32 = 1030;
pub const FD_CLOEXEC_FLAG: u32 = 1;

pub const AT_FDCWD: i32 = -100;
pub const AT_SYMLINK_NOFOLLOW: u32 = 0x100;
pub const AT_SYMLINK_FOLLOW: u32 = 0x400;
pub const AT_EMPTY_PATH: u32 = 0x1000;
pub const AT_REMOVEDIR: u32 = 0x200;

pub const R_OK: u32 = 4;
pub const W_OK: u32 = 2;
pub const X_OK: u32 = 1;
pub const F_OK: u32 = 0;

pub const CLOCK_REALTIME: u32 = 0;
pub const CLOCK_MONOTONIC: u32 = 1;

pub const RLIMIT_STACK: u32 = 3;
pub const RLIMIT_NOFILE: u32 = 7;
pub const RLIM_INFINITY: u32 = 0xffff_ffff;

pub const MADV_NORMAL: u32 = 0;
pub const MADV_DONTNEED: u32 = 4;
pub const MADV_FREE: u32 = 8;

pub const FUTEX_WAIT: u32 = 0;
pub const FUTEX_WAKE: u32 = 1;
pub const FUTEX_PRIVATE_FLAG: u32 = 128;

pub const TIOCGWINSZ: u32 = 0x5413;
pub const TCGETS: u32 = 0x5401;
pub const TCSETS: u32 = 0x5402;
pub const TCSETSW: u32 = 0x5403;
pub const TCSETSF: u32 = 0x5404;

pub const POLLIN: i16 = 0x0001;
pub const POLLOUT: i16 = 0x0004;
pub const POLLERR: i16 = 0x0008;
pub const POLLHUP: i16 = 0x0010;
pub const POLLNVAL: i16 = 0x0020;

pub const SEEK_SET: u32 = 0;
pub const SEEK_CUR: u32 = 1;
pub const SEEK_END: u32 = 2;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UserIovec {
    pub base: u32,
    pub len: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PollFd {
    pub fd: i32,
    pub events: i16,
    pub revents: i16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FdSet32 {
    pub bits: [u32; 32],
}

pub const DT_UNKNOWN: u8 = 0;
pub const DT_REG: u8 = 8;
pub const DT_DIR: u8 = 4;
pub const DT_CHR: u8 = 2;
pub const DT_LNK: u8 = 10;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UtsName {
    pub sysname: [u8; 65],
    pub nodename: [u8; 65],
    pub release: [u8; 65],
    pub version: [u8; 65],
    pub machine: [u8; 65],
    pub domainname: [u8; 65],
}

impl UtsName {
    pub const fn empty() -> Self {
        Self {
            sysname: [0; 65],
            nodename: [0; 65],
            release: [0; 65],
            version: [0; 65],
            machine: [0; 65],
            domainname: [0; 65],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TimeVal {
    pub tv_sec: i32,
    pub tv_usec: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TimeSpec {
    pub tv_sec: i32,
    pub tv_nsec: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TimeSpec64 {
    pub tv_sec: i64,
    pub tv_nsec: i64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RLimit {
    pub rlim_cur: u32,
    pub rlim_max: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Tms {
    pub tms_utime: i32,
    pub tms_stime: i32,
    pub tms_cutime: i32,
    pub tms_cstime: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SysInfo {
    pub uptime: i32,
    pub loads: [u32; 3],
    pub totalram: u32,
    pub freeram: u32,
    pub sharedram: u32,
    pub bufferram: u32,
    pub totalswap: u32,
    pub freeswap: u32,
    pub procs: u16,
    pub pad: u16,
    pub totalhigh: u32,
    pub freehigh: u32,
    pub mem_unit: u32,
    pub reserved: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct StatFs64 {
    pub f_type: u32,
    pub f_bsize: u32,
    pub f_blocks: u64,
    pub f_bfree: u64,
    pub f_bavail: u64,
    pub f_files: u64,
    pub f_ffree: u64,
    pub f_fsid: [i32; 2],
    pub f_namelen: u32,
    pub f_frsize: u32,
    pub f_flags: u32,
    pub f_spare: [u32; 4],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct WinSize {
    pub ws_row: u16,
    pub ws_col: u16,
    pub ws_xpixel: u16,
    pub ws_ypixel: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Termios {
    pub iflag: u32,
    pub oflag: u32,
    pub cflag: u32,
    pub lflag: u32,
    pub line: u8,
    pub cc: [u8; 19],
}

#[inline]
pub fn yield_now() {
    unsafe {
        syscall1(SYS_YIELD, 0);
    }
}

#[inline]
pub fn sleep(ticks: u32) {
    unsafe {
        syscall1(SYS_SLEEP, ticks);
    }
}

#[inline]
pub fn block(channel: u32) {
    unsafe {
        syscall1(SYS_BLOCK, channel);
    }
}

#[inline]
pub fn wake(channel: u32) -> usize {
    unsafe { syscall1(SYS_WAKE, channel) as usize }
}

#[inline]
#[allow(dead_code)]
pub fn write(fd: u32, bytes: &[u8]) -> isize {
    unsafe { syscall3(SYS_WRITE, fd, bytes.as_ptr() as u32, bytes.len() as u32) as i32 as isize }
}

#[inline]
#[allow(dead_code)]
pub fn read(fd: u32, bytes: &mut [u8]) -> isize {
    unsafe { syscall3(SYS_READ, fd, bytes.as_mut_ptr() as u32, bytes.len() as u32) as i32 as isize }
}

#[inline]
#[allow(dead_code)]
pub fn open(path: &[u8], flags: u32) -> isize {
    unsafe { syscall3(SYS_OPEN, path.as_ptr() as u32, path.len() as u32, flags) as i32 as isize }
}

#[inline]
#[allow(dead_code)]
pub fn close(fd: u32) -> isize {
    unsafe { syscall1(SYS_CLOSE, fd) as i32 as isize }
}

#[inline]
#[allow(dead_code)]
pub fn wait(pid: u32) -> isize {
    unsafe { syscall1(SYS_WAIT, pid) as i32 as isize }
}

#[inline]
#[allow(dead_code)]
pub fn spawn(path: &[u8]) -> isize {
    unsafe { syscall3(SYS_SPAWN, path.as_ptr() as u32, path.len() as u32, 0) as i32 as isize }
}

#[inline]
#[allow(dead_code)]
pub fn exec(path: &[u8]) -> isize {
    unsafe { syscall3(SYS_EXEC, path.as_ptr() as u32, path.len() as u32, 0) as i32 as isize }
}

#[inline]
#[allow(dead_code)]
pub fn pipe(fds: &mut [u32; 2]) -> isize {
    unsafe { syscall1(SYS_PIPE, fds.as_mut_ptr() as u32) as i32 as isize }
}

#[inline]
#[allow(dead_code)]
pub fn brk(addr: usize) -> isize {
    unsafe { syscall1(SYS_BRK, addr as u32) as i32 as isize }
}

#[inline]
#[allow(dead_code)]
pub fn mmap(addr: usize, len: usize, prot: u32, flags: u32, fd: i32, offset: usize) -> isize {
    unsafe {
        syscall6(
            SYS_MMAP,
            addr as u32,
            len as u32,
            prot,
            flags,
            fd as u32,
            offset as u32,
        ) as i32 as isize
    }
}

#[inline]
#[allow(dead_code)]
pub fn munmap(addr: usize, len: usize) -> isize {
    unsafe { syscall3(SYS_MUNMAP, addr as u32, len as u32, 0) as i32 as isize }
}

#[inline]
#[allow(dead_code)]
pub fn mprotect(addr: usize, len: usize, prot: u32) -> isize {
    unsafe { syscall3(SYS_MPROTECT, addr as u32, len as u32, prot) as i32 as isize }
}

#[inline]
#[allow(dead_code)]
pub fn exit() -> ! {
    exit_with(0)
}

#[inline]
pub fn exit_with(code: i32) -> ! {
    unsafe {
        syscall1(SYS_EXIT, code as u32);
    }

    loop {
        aarch32_cpu::asm::wfi();
    }
}

#[inline]
unsafe fn syscall1(number: u32, arg0: u32) -> u32 {
    let ret: u32;
    unsafe {
        core::arch::asm!(
            "svc #0",
            inlateout("r0") number => ret,
            in("r1") arg0,
            in("r7") KERNEL_ABI_MAGIC,
            options(nostack)
        );
    }
    ret
}

#[inline]
#[allow(dead_code)]
unsafe fn syscall3(number: u32, arg0: u32, arg1: u32, arg2: u32) -> u32 {
    let ret: u32;
    unsafe {
        core::arch::asm!(
            "svc #0",
            inlateout("r0") number => ret,
            in("r1") arg0,
            in("r2") arg1,
            in("r3") arg2,
            in("r7") KERNEL_ABI_MAGIC,
            options(nostack)
        );
    }
    ret
}

#[inline]
#[allow(dead_code)]
unsafe fn syscall6(
    number: u32,
    arg0: u32,
    arg1: u32,
    arg2: u32,
    arg3: u32,
    arg4: u32,
    arg5: u32,
) -> u32 {
    let ret: u32;
    unsafe {
        core::arch::asm!(
            "svc #0",
            inlateout("r0") number => ret,
            in("r1") arg0,
            in("r2") arg1,
            in("r3") arg2,
            in("r4") arg3,
            in("r5") arg4,
            in("r7") KERNEL_ABI_MAGIC,
            in("r12") arg5,
            options(nostack)
        );
    }
    ret
}
