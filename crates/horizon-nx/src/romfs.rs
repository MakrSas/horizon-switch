//! The prepared resources as a RomFS image (`resources.romfs`): one file on the SD card instead of
//! 28,000. Every open, stat and read of a loose file is a call to the system's file service; in an
//! image the lookups are tables in memory and the data comes through one open file.

use std::ffi::{CString, c_char};
use std::path::{Path, PathBuf};

unsafe extern "C" {
    fn nxshim_mount_romfs(path: *const c_char, name: *const c_char) -> i32;
}

/// Mounts the image at `image` as `name:/` and returns that root, or None.
pub fn mount(image: &Path, name: &str) -> Option<PathBuf> {
    if !image.is_file() {
        return None;
    }
    let path = CString::new(image.to_str()?).ok()?;
    let device = CString::new(name).ok()?;
    // SAFETY: two valid C strings; libnx copies what it keeps.
    let rc = unsafe { nxshim_mount_romfs(path.as_ptr(), device.as_ptr()) };
    (rc == 0).then(|| PathBuf::from(format!("{name}:/")))
}
