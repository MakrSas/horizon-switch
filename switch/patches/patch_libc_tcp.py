"""The Switch's TCP and IP option numbers (libnx netinet/in.h and netinet/tcp.h, FreeBSD's), where
libc's newlib module has others: TCP_NODELAY was 8193, so every set_nodelay failed."""
import pathlib
import sys

ROOT = pathlib.Path(sys.argv[1]) / "src/unix/newlib/mod.rs"
SW = 'all(target_os = "horizon", target_arch = "aarch64")'
s = ROOT.read_text()
EDITS = [
    ('    if #[cfg(any(target_os = "vita", target_os = "rtems"))] {\n        pub const TCP_NODELAY: c_int = 1;',
     f'    if #[cfg(any(target_os = "vita", target_os = "rtems", {SW}))] {{\n        pub const TCP_NODELAY: c_int = 1;'),
    ('    if #[cfg(any(target_os = "vita", target_os = "rtems"))] {\n        pub const IP_TTL: c_int = 4;',
     f'    if #[cfg(any(target_os = "vita", target_os = "rtems", {SW}))] {{\n        pub const IP_TTL: c_int = 4;'),
    ('    if #[cfg(any(target_os = "vita", target_os = "rtems"))] {\n        pub const IP_ADD_MEMBERSHIP: c_int = 12;',
     f'    if #[cfg(any(target_os = "vita", target_os = "rtems", {SW}))] {{\n        pub const IP_ADD_MEMBERSHIP: c_int = 12;'),
    ('    if #[cfg(target_os = "horizon")] {\n        pub const IP_TOS: c_int = 7;',
     f'    if #[cfg({SW})] {{\n        pub const IP_TOS: c_int = 3;\n    }} else if #[cfg(target_os = "horizon")] {{\n        pub const IP_TOS: c_int = 7;'),
]
for old, new in EDITS:
    if new in s:
        continue
    assert s.count(old) == 1, old[:70]
    s = s.replace(old, new)
ROOT.write_text(s)
print("libc TCP/IP options patched")
