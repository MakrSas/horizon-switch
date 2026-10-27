#![cfg(target_os = "horizon")]
//! Nintendo Switch (libnx) backend: one fixed full-screen window on the default `NWindow`,
//! the touch screen as a mouse, the applet's focus and exit requests as window events.

use std::fmt::{self, Display, Formatter};

use smol_str::SmolStr;

use crate::dpi::{PhysicalPosition, PhysicalSize};
use crate::keyboard::Key;

pub(crate) use self::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy, OwnedDisplayHandle};
mod event_loop;

pub use self::window::Window;
mod window;

/// The framebuffer the game renders into, in both handheld and docked mode (the system scales
/// it to the TV).
pub(crate) const WIDTH: u32 = 1280;
pub(crate) const HEIGHT: u32 = 720;

pub(crate) mod ffi {
    use std::ffi::c_void;

    pub const FOCUS_IN_FOCUS: u8 = 1;
    pub const OPERATION_MODE_CONSOLE: u8 = 1;

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct HidTouchState {
        pub delta_time: u64,
        pub attributes: u32,
        pub finger_id: u32,
        pub x: u32,
        pub y: u32,
        pub diameter_x: u32,
        pub diameter_y: u32,
        pub rotation_angle: u32,
        pub reserved: u32,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct HidTouchScreenState {
        pub sampling_number: u64,
        pub count: i32,
        pub reserved: u32,
        pub touches: [HidTouchState; 16],
    }

    extern "C" {
        pub fn nwindowGetDefault() -> *mut c_void;
        pub fn appletMainLoop() -> bool;
        pub fn appletGetFocusState() -> u8;
        pub fn appletGetOperationMode() -> u8;
        pub fn hidInitialize() -> u32;
        pub fn hidInitializeTouchScreen();
        pub fn hidGetTouchScreenStates(states: *mut HidTouchScreenState, count: usize) -> usize;
    }
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) struct PlatformSpecificEventLoopAttributes {}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WindowId(u64);

impl WindowId {
    pub const fn dummy() -> Self {
        WindowId(0)
    }
}

impl From<WindowId> for u64 {
    fn from(id: WindowId) -> Self {
        id.0
    }
}

impl From<u64> for WindowId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DeviceId;

impl DeviceId {
    pub const fn dummy() -> Self {
        DeviceId
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PlatformSpecificWindowAttributes;

#[derive(Clone, Debug)]
pub struct OsError(&'static str);

impl Display for OsError {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        fmt.write_str(self.0)
    }
}

pub(crate) use crate::cursor::{
    NoCustomCursor as PlatformCustomCursor, NoCustomCursor as PlatformCustomCursorSource,
};
pub(crate) use crate::icon::NoIcon as PlatformIcon;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MonitorHandle;

impl MonitorHandle {
    pub fn name(&self) -> Option<String> {
        // SAFETY: a plain applet query.
        let docked = unsafe { ffi::appletGetOperationMode() } == ffi::OPERATION_MODE_CONSOLE;
        Some(if docked { "Nintendo Switch (TV)" } else { "Nintendo Switch" }.to_owned())
    }

    pub fn size(&self) -> PhysicalSize<u32> {
        PhysicalSize::new(WIDTH, HEIGHT)
    }

    pub fn position(&self) -> PhysicalPosition<i32> {
        (0, 0).into()
    }

    pub fn scale_factor(&self) -> f64 {
        1.0
    }

    pub fn refresh_rate_millihertz(&self) -> Option<u32> {
        Some(60000)
    }

    pub fn video_modes(&self) -> impl Iterator<Item = VideoModeHandle> {
        std::iter::once(VideoModeHandle { monitor: self.clone() })
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct VideoModeHandle {
    monitor: MonitorHandle,
}

impl VideoModeHandle {
    pub fn size(&self) -> PhysicalSize<u32> {
        self.monitor.size()
    }

    pub fn bit_depth(&self) -> u16 {
        32
    }

    pub fn refresh_rate_millihertz(&self) -> u32 {
        60000
    }

    pub fn monitor(&self) -> MonitorHandle {
        self.monitor.clone()
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct KeyEventExtra {
    pub key_without_modifiers: Key,
    pub text_with_all_modifiers: Option<SmolStr>,
}
