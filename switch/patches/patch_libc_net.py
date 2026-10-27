"""The socket constants of libnx's headers (FreeBSD's) for the Switch, where libc's horizon
module has the 3DS's. Applied to the vendored libc and recorded in patch_libc.py."""
import pathlib
import sys

ROOT = pathlib.Path(sys.argv[1]) / "src/unix/newlib"
SW = 'all(target_os = "horizon", target_arch = "aarch64")'

EDITS = [
    ("horizon/mod.rs", "pub const MSG_OOB: c_int = 1;\npub const MSG_PEEK: c_int = 2;\npub const MSG_DONTWAIT: c_int = 4;\npub const MSG_DONTROUTE: c_int = 0;\npub const MSG_WAITALL: c_int = 0;\npub const MSG_MORE: c_int = 0;\npub const MSG_NOSIGNAL: c_int = 0;",
     f"""pub const MSG_OOB: c_int = 1;
pub const MSG_PEEK: c_int = 2;
pub const MSG_MORE: c_int = 0;
// The Switch: libnx's sys/socket.h (FreeBSD's values).
cfg_if! {{
    if #[cfg({SW})] {{
        pub const MSG_DONTWAIT: c_int = 0x80;
        pub const MSG_DONTROUTE: c_int = 0x4;
        pub const MSG_WAITALL: c_int = 0x40;
        pub const MSG_NOSIGNAL: c_int = 0x20000;
    }} else {{
        pub const MSG_DONTWAIT: c_int = 4;
        pub const MSG_DONTROUTE: c_int = 0;
        pub const MSG_WAITALL: c_int = 0;
        pub const MSG_NOSIGNAL: c_int = 0;
    }}
}}"""),
    ("horizon/mod.rs", "pub const POLLOUT: c_int = 0x0008;",
     f"#[cfg({SW})]\npub const POLLOUT: c_int = 0x0004;\n#[cfg(not({SW}))]\npub const POLLOUT: c_int = 0x0008;"),
    ("horizon/mod.rs", "pub const AF_INET6: c_int = 23;",
     f"#[cfg({SW})]\npub const AF_INET6: c_int = 28;\n#[cfg(not({SW}))]\npub const AF_INET6: c_int = 23;"),
    ("horizon/mod.rs", "pub const FIONBIO: c_ulong = 1;",
     f"// libnx's sys/filio.h: _IOW('f', 126, int).\n#[cfg({SW})]\npub const FIONBIO: c_ulong = 0x8004_667e;\n#[cfg(not({SW}))]\npub const FIONBIO: c_ulong = 1;"),
    ("mod.rs", '    if #[cfg(target_os = "horizon")] {\n        pub const SO_ERROR: c_int = 0x1009;',
     f'    if #[cfg({SW})] {{\n        pub const SO_ERROR: c_int = 0x1007;\n    }} else if #[cfg(target_os = "horizon")] {{\n        pub const SO_ERROR: c_int = 0x1009;'),
    ("mod.rs", '    } else if #[cfg(target_os = "rtems")] {\n        pub const EAI_FAMILY: c_int = 5;',
     f'    }} else if #[cfg(any(target_os = "rtems", {SW}))] {{\n        pub const EAI_FAMILY: c_int = 5;'),
]

for path, old, new in EDITS:
    p = ROOT / path
    s = p.read_text()
    if new in s:
        continue
    if s.count(old) != 1:
        sys.exit(f"pattern not found in {path}: {old[:60]!r}")
    p.write_text(s.replace(old, new))
print("libc socket constants patched")
