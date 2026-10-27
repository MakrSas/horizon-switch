//! Controllers through libnx `pad`: player 1 (Pro Controller, Joy-Con pair or single) and the
//! Joy-Con attached to the console.

#[repr(C)]
#[derive(Default)]
struct PadState {
    id_mask: u8,
    active_id_mask: u8,
    read_handheld: bool,
    active_handheld: bool,
    style_set: u32,
    attributes: u32,
    buttons_cur: u64,
    buttons_old: u64,
    sticks: [[i32; 2]; 2],
    gc_triggers: [u32; 2],
}

unsafe extern "C" {
    fn nxshim_rumble(handheld: bool, styles: u32, low: f32, high: f32);
    fn padConfigureInput(max_players: u32, style_set: u32);
    fn padInitializeWithMask(pad: *mut PadState, mask: u64);
    fn padUpdate(pad: *mut PadState);
}

/// `HidNpadStyleSet_NpadStandard`: Pro Controller, handheld, dual, single left and right Joy-Con.
const STYLES_STANDARD: u32 = 0x1F;
/// `HidNpadIdType_No1` and `HidNpadIdType_Handheld` (0x20).
const PLAYER_ONE: u64 = (1 << 0) | (1 << 0x20);
/// `HidNpadAttribute_IsConnected`.
const CONNECTED: u32 = 1 << 0;

/// `HidNpadButton` bits.
pub mod button {
    pub const A: u64 = 1 << 0;
    pub const B: u64 = 1 << 1;
    pub const X: u64 = 1 << 2;
    pub const Y: u64 = 1 << 3;
    pub const STICK_L: u64 = 1 << 4;
    pub const STICK_R: u64 = 1 << 5;
    pub const L: u64 = 1 << 6;
    pub const R: u64 = 1 << 7;
    pub const ZL: u64 = 1 << 8;
    pub const ZR: u64 = 1 << 9;
    pub const PLUS: u64 = 1 << 10;
    pub const MINUS: u64 = 1 << 11;
    pub const LEFT: u64 = 1 << 12;
    pub const UP: u64 = 1 << 13;
    pub const RIGHT: u64 = 1 << 14;
    pub const DOWN: u64 = 1 << 15;
}

/// One reading of the controllers.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Snapshot {
    pub connected: bool,
    /// The Joy-Con attached to the console are the ones read.
    pub handheld: bool,
    /// `HidNpadStyleTag` bits of what is connected.
    pub styles: u32,
    /// `button` bits held.
    pub buttons: u64,
    /// Left and right stick, each axis in -1..=1, up positive.
    pub sticks: [[f32; 2]; 2],
}

/// Player 1's controllers.
pub struct Pad(Box<PadState>);

impl Pad {
    pub fn new() -> Self {
        let mut state = Box::<PadState>::default();
        // SAFETY: libnx's pad setup into a state that lives as long as the `Pad`.
        unsafe {
            padConfigureInput(1, STYLES_STANDARD);
            padInitializeWithMask(&mut *state, PLAYER_ONE);
        }
        Self(state)
    }

    pub fn update(&mut self) -> Snapshot {
        // SAFETY: as above.
        unsafe { padUpdate(&mut *self.0) };
        let s = &self.0;
        let axis = |v: i32| (v as f32 / 32767.0).clamp(-1.0, 1.0);
        Snapshot {
            connected: s.attributes & CONNECTED != 0,
            handheld: s.active_handheld,
            styles: s.style_set,
            buttons: s.buttons_cur,
            sticks: s.sticks.map(|[x, y]| [axis(x), axis(y)]),
        }
    }
}

impl Default for Pad {
    fn default() -> Self {
        Self::new()
    }
}

/// Vibrates player 1's controllers (HD Rumble): `low` on the low band (the heavy motor), `high`
/// on the high band (the light one), each 0..1, until called again; 0 and 0 stop it. `handheld`
/// and `styles` are the last `Snapshot`'s.
pub fn rumble(handheld: bool, styles: u32, low: f32, high: f32) {
    // SAFETY: plain values; nxshim opens the vibration devices it needs.
    unsafe { nxshim_rumble(handheld, styles, low.clamp(0.0, 1.0), high.clamp(0.0, 1.0)) }
}
