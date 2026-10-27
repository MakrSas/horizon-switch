//! The game's global allocator on the Switch: newlib's malloc behind a cache of small blocks.
//!
//! newlib's malloc has one lock for the whole heap, and every thread (render, server, chunk
//! generation and meshing) queued on it. Blocks of up to 8 KiB come from per-thread magazines
//! instead ([`small_blocks`], from bevy-horizon, MIT or Apache-2.0); larger ones go straight to
//! malloc. What is allocated does not change, only how fast.
use core::alloc::{GlobalAlloc, Layout};
use core::ffi::c_void;

#[allow(dead_code)]
mod small_blocks;

use small_blocks::{Backend, Cache, MAX_ALIGN};

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn calloc(count: usize, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn malloc_usable_size(ptr: *mut c_void) -> usize;
    fn memalign(align: usize, size: usize) -> *mut c_void;
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
}

struct Newlib;

impl Backend for Newlib {
    #[inline]
    unsafe fn alloc(size: usize, zeroed: bool) -> *mut u8 {
        unsafe { if zeroed { calloc(1, size) } else { malloc(size) }.cast() }
    }

    #[inline]
    unsafe fn free(ptr: *mut u8) {
        unsafe { free(ptr.cast()) }
    }

    #[inline]
    unsafe fn usable_size(ptr: *mut u8) -> usize {
        unsafe { malloc_usable_size(ptr.cast()) }
    }

    /// The thread's local region (`tpidrro_el0`), 0x200 bytes apart per thread.
    #[inline]
    fn shard_hint() -> usize {
        let tlr: usize;
        unsafe {
            core::arch::asm!("mrs {}, tpidrro_el0", out(reg) tlr, options(nomem, nostack, preserves_flags))
        };
        tlr >> 9
    }
}

static CACHE: Cache<Newlib> = Cache::new();

/// The Switch build's `#[global_allocator]`.
pub struct Allocator;

unsafe fn malloc_alloc(layout: Layout, zeroed: bool) -> *mut u8 {
    unsafe {
        if layout.align() <= MAX_ALIGN {
            Newlib::alloc(layout.size(), zeroed)
        } else {
            let ptr = memalign(layout.align(), layout.size()).cast::<u8>();
            if zeroed && !ptr.is_null() {
                core::ptr::write_bytes(ptr, 0, layout.size());
            }
            ptr
        }
    }
}

unsafe fn allocate(layout: Layout, zeroed: bool) -> *mut u8 {
    let once = || match CACHE.class_for(layout.size(), layout.align()) {
        Some(class) => unsafe { CACHE.alloc(class, layout.size(), zeroed) },
        None => unsafe { malloc_alloc(layout, zeroed) },
    };
    let ptr = once();
    // malloc is full: give it the cached blocks and ask again.
    if ptr.is_null() && CACHE.drain() { once() } else { ptr }
}

unsafe fn deallocate(ptr: *mut u8, layout: Layout) {
    match CACHE.class_for(layout.size(), layout.align()) {
        Some(class) => unsafe { CACHE.free(ptr, class) },
        None => unsafe { free(ptr.cast()) },
    }
}

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { allocate(layout, false) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        unsafe { allocate(layout, true) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { deallocate(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: GlobalAlloc::realloc's caller guarantees the new layout is valid.
        let new_layout = unsafe { Layout::from_size_align_unchecked(new_size, layout.align()) };
        let old_cached = CACHE.class_for(layout.size(), layout.align());
        let new_cached = CACHE.class_for(new_size, layout.align());
        // Both sides plain malloc blocks: malloc's own realloc.
        if old_cached.is_none() && new_cached.is_none() && layout.align() <= MAX_ALIGN {
            let new = unsafe { realloc(ptr.cast(), new_size) }.cast::<u8>();
            if !new.is_null() || !CACHE.drain() {
                return new;
            }
            return unsafe { realloc(ptr.cast(), new_size) }.cast();
        }
        // Same class: the block already holds the new size.
        if old_cached.is_some() && old_cached == new_cached {
            return ptr;
        }
        let new = unsafe { allocate(new_layout, false) };
        if !new.is_null() {
            unsafe {
                core::ptr::copy_nonoverlapping(ptr, new, layout.size().min(new_size));
                deallocate(ptr, layout);
            }
        }
        new
    }
}
