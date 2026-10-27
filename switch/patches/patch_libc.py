#!/usr/bin/env python3
# Patch libc 0.2.191's newlib/horizon module for aarch64 Nintendo Switch (libnx + devkitA64 newlib).
# Sizes measured with aarch64-none-elf-gcc against libnx headers.
import sys, pathlib
root = pathlib.Path(sys.argv[1]) / "src/unix/newlib"
SW = 'all(target_os = "horizon", target_arch = "aarch64")'

def sub(path, old, new, count=1):
    p = root / path
    s = p.read_text()
    if new in s:
        return
    if s.count(old) < count:
        sys.exit(f"pattern not found in {path}: {old[:60]!r}")
    p.write_text(s.replace(old, new, count))

m = "mod.rs"
sub(m, "pub type blkcnt_t = i32;\npub type blksize_t = i32;",
    f"cfg_if! {{\n    if #[cfg({SW})] {{\n        pub type blkcnt_t = c_long;\n        pub type blksize_t = c_long;\n    }} else {{\n        pub type blkcnt_t = i32;\n        pub type blksize_t = i32;\n    }}\n}}")
sub(m, '    } else if #[cfg(any(target_arch = "arm", target_arch = "powerpc"))] {',
    f'    }} else if #[cfg({SW})] {{\n        pub type dev_t = c_short;\n        pub type ino_t = c_ushort;\n        pub type off_t = i64;\n    }} else if #[cfg(any(target_arch = "arm", target_arch = "powerpc"))] {{')
sub(m, '    if #[cfg(target_os = "horizon")] {\n        pub type sa_family_t = u16;',
    f'    if #[cfg({SW})] {{\n        pub type sa_family_t = u8;\n    }} else if #[cfg(target_os = "horizon")] {{\n        pub type sa_family_t = u16;')
sub(m, "pub type suseconds_t = i32;",
    f"#[cfg({SW})]\npub type suseconds_t = c_long;\n#[cfg(not({SW}))]\npub type suseconds_t = i32;")
sub(m, '    } else if #[cfg(target_os = "vita")] {\n        const __PTHREAD_INITIALIZER_BYTE',
    f'''    }} else if #[cfg({SW})] {{
        const __PTHREAD_INITIALIZER_BYTE: u8 = 0;
        pub const __SIZEOF_PTHREAD_ATTR_T: usize = 24;
        pub const __SIZEOF_PTHREAD_MUTEX_T: usize = 12;
        pub const __SIZEOF_PTHREAD_MUTEXATTR_T: usize = 4;
        pub const __SIZEOF_PTHREAD_COND_T: usize = 16;
        pub const __SIZEOF_PTHREAD_CONDATTR_T: usize = 8;
        pub const __SIZEOF_PTHREAD_RWLOCK_T: usize = 16;
        pub const __SIZEOF_PTHREAD_RWLOCKATTR_T: usize = 4;
        pub const __SIZEOF_PTHREAD_BARRIER_T: usize = 20;
    }} else if #[cfg(target_os = "vita")] {{
        const __PTHREAD_INITIALIZER_BYTE''')
# condattr is 8-aligned in libnx
sub(m, "    #[repr(align(4))]\n    pub struct pthread_condattr_t {",
    f"    #[cfg_attr({SW}, repr(align(8)))]\n    #[cfg_attr(not({SW}), repr(align(4)))]\n    pub struct pthread_condattr_t {{")

h = "horizon/mod.rs"
sub(h, """    pub struct sockaddr {
        pub sa_family: crate::sa_family_t,
        pub sa_data: [c_char; 26usize],
    }

    pub struct sockaddr_storage {
        pub ss_family: crate::sa_family_t,
        __ss_padding: Padding<[c_char; 26usize]>,
    }

    pub struct sockaddr_in {
        pub sin_family: crate::sa_family_t,""",
f"""    pub struct sockaddr {{
        #[cfg({SW})]
        pub sa_len: u8,
        pub sa_family: crate::sa_family_t,
        #[cfg({SW})]
        pub sa_data: [c_char; 14usize],
        #[cfg(not({SW}))]
        pub sa_data: [c_char; 26usize],
    }}

    #[cfg_attr({SW}, repr(align(8)))]
    pub struct sockaddr_storage {{
        #[cfg({SW})]
        pub ss_len: u8,
        pub ss_family: crate::sa_family_t,
        #[cfg({SW})]
        __ss_padding: Padding<[c_char; 126usize]>,
        #[cfg(not({SW}))]
        __ss_padding: Padding<[c_char; 26usize]>,
    }}

    pub struct sockaddr_in {{
        #[cfg({SW})]
        pub sin_len: u8,
        pub sin_family: crate::sa_family_t,""")
sub(h, "    pub struct sockaddr_in6 {\n        pub sin6_family",
    f"    pub struct sockaddr_in6 {{\n        #[cfg({SW})]\n        pub sin6_len: u8,\n        pub sin6_family")
print("patched")
