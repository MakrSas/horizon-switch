//! The memory the game may use: about 3.2 GB when started over a game (holding R), about
//! 400 MB from the album.

unsafe extern "C" {
    fn nxshim_memory(total: *mut u64, used: *mut u64);
}

/// (total, used) bytes for this process.
pub fn usage() -> (u64, u64) {
    let (mut total, mut used) = (0, 0);
    // SAFETY: two plain svcGetInfo queries into valid pointers.
    unsafe { nxshim_memory(&mut total, &mut used) };
    (total, used)
}
