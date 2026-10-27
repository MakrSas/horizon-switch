//! libnx's text console on the default window, for work before the renderer starts (the first
//! start's unpacking): a line of progress instead of a black screen. It must be closed before
//! EGL takes the window.

use std::ffi::c_void;
use std::io::Write;

unsafe extern "C" {
    fn consoleInit(console: *mut c_void) -> *mut c_void;
    fn consoleUpdate(console: *mut c_void);
    fn consoleExit(console: *mut c_void);
    fn appletMainLoop() -> bool;
}

/// The console while it is open.
pub struct Console(());

impl Console {
    pub fn open() -> Self {
        // SAFETY: the default console on the default window; no renderer holds it yet.
        unsafe { consoleInit(std::ptr::null_mut()) };
        Self(())
    }

    /// Writes `text` (ANSI escapes work) and shows it. Also pumps the applet's messages, so
    /// HOME and sleep keep working: returns false once the system asks the program to exit.
    pub fn show(&mut self, text: &str) -> bool {
        let mut out = std::io::stdout();
        let _ = out.write_all(text.as_bytes());
        let _ = out.flush();
        // SAFETY: as above.
        unsafe {
            consoleUpdate(std::ptr::null_mut());
            appletMainLoop()
        }
    }
}

impl Drop for Console {
    fn drop(&mut self) {
        // SAFETY: closes the console `open` opened.
        unsafe { consoleExit(std::ptr::null_mut()) };
    }
}

