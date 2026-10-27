"""The Switch's pollfd (libnx poll.h: short events and revents, 8 bytes), where libc's horizon
module has the 3DS's (ints, 12 bytes): poll wrote revents where Rust did not read them.
Appended to switch/patches/patch_libc_net.py's edits."""
import pathlib
import sys

ROOT = pathlib.Path(sys.argv[1]) / "src/unix/newlib/horizon/mod.rs"
SW = 'all(target_os = "horizon", target_arch = "aarch64")'
s = ROOT.read_text()

old_struct = """    pub struct pollfd {
        pub fd: c_int,
        pub events: c_int,
        pub revents: c_int,
    }"""
new_struct = f"""    pub struct pollfd {{
        pub fd: c_int,
        pub events: PollFlags,
        pub revents: PollFlags,
    }}"""
if new_struct not in s:
    assert s.count(old_struct) == 1
    s = s.replace(old_struct, new_struct)

old_flags = """pub const POLLIN: c_int = 0x0001;
pub const POLLPRI: c_int = 0x0002;
#[cfg(all(target_os = "horizon", target_arch = "aarch64"))]
pub const POLLOUT: c_int = 0x0004;
#[cfg(not(all(target_os = "horizon", target_arch = "aarch64")))]
pub const POLLOUT: c_int = 0x0008;
pub const POLLRDNORM: c_int = 0x0040;
pub const POLLWRNORM: c_int = POLLOUT;
pub const POLLRDBAND: c_int = 0x0080;
pub const POLLWRBAND: c_int = 0x0100;
/// POLLERR behavior on 3DS+HorizonOS is unclear, and it may be unsupported.
pub const POLLERR: c_int = 0x0008;
/// POLLHUP behavior on 3DS+HorizonOS is unclear, and it may be unsupported.
pub const POLLHUP: c_int = 0x0010;
pub const POLLNVAL: c_int = 0x0020;"""
new_flags = f"""// The Switch's poll.h: short flags, POLLOUT 0x4, and POLLERR 0x8 apart from it.
#[cfg({SW})]
pub type PollFlags = crate::c_short;
#[cfg(not({SW}))]
pub type PollFlags = c_int;
pub const POLLIN: PollFlags = 0x0001;
pub const POLLPRI: PollFlags = 0x0002;
#[cfg({SW})]
pub const POLLOUT: PollFlags = 0x0004;
#[cfg(not({SW}))]
pub const POLLOUT: PollFlags = 0x0008;
pub const POLLRDNORM: PollFlags = 0x0040;
pub const POLLWRNORM: PollFlags = POLLOUT;
pub const POLLRDBAND: PollFlags = 0x0080;
pub const POLLWRBAND: PollFlags = 0x0100;
/// POLLERR behavior on 3DS+HorizonOS is unclear, and it may be unsupported.
pub const POLLERR: PollFlags = 0x0008;
/// POLLHUP behavior on 3DS+HorizonOS is unclear, and it may be unsupported.
pub const POLLHUP: PollFlags = 0x0010;
pub const POLLNVAL: PollFlags = 0x0020;"""
if new_flags not in s:
    assert s.count(old_flags) == 1, "flags block not found"
    s = s.replace(old_flags, new_flags)
ROOT.write_text(s)
print("libc pollfd patched")
