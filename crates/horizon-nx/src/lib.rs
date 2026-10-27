//! The libnx services Horizon uses on the Nintendo Switch, behind safe wrappers: the game's
//! crates keep `#![forbid(unsafe_code)]`. Builds on every target; off the Switch it is empty.
#![cfg(target_os = "horizon")]

pub mod alloc;
pub mod console;
pub mod memory;
pub mod pad;
pub mod romfs;

unsafe extern "C" {
    fn nxshim_exit_now() -> !;
    fn nxshim_net_init() -> i32;
}

/// Starts the network (BSD sockets and DNS). Whether it did.
pub fn net_init() -> bool {
    // SAFETY: libnx's socket setup, once at startup.
    unsafe { nxshim_net_init() == 0 }
}

/// Ends the process now, without libnx exit handlers or joining threads.
pub fn exit_now() -> ! {
    // SAFETY: svcExitProcess; nothing runs after it.
    unsafe { nxshim_exit_now() }
}
