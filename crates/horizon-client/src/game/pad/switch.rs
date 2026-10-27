//! The Switch's own controllers (`horizon_nx::pad`): the Joy-Con, attached or held, and the Pro
//! Controller, read once per poll into the same `PadState` the other platforms fill from gilrs.
//! Buttons map by name, as the Switch's own games use them: A confirms (Horizon's `South`), B goes
//! back (`East`), X and Y are the `West` and `North` actions. The glyphs then read A, B, X, Y.
use super::{super::Game, Pads};
use horizon_client::gamepad::{Button, GlyphStyle, PadState, Pulse, Rumble};
use horizon_nx::pad::{Pad, Snapshot, button as nx};
use serde_json::{Value, json};

const BUTTONS: [(Button, u64); 14] = [
    (Button::South, nx::A),
    (Button::East, nx::B),
    (Button::West, nx::X),
    (Button::North, nx::Y),
    (Button::LeftStick, nx::STICK_L),
    (Button::RightStick, nx::STICK_R),
    (Button::LeftBumper, nx::L),
    (Button::RightBumper, nx::R),
    (Button::Start, nx::PLUS),
    (Button::Select, nx::MINUS),
    (Button::DPadLeft, nx::LEFT),
    (Button::DPadUp, nx::UP),
    (Button::DPadRight, nx::RIGHT),
    (Button::DPadDown, nx::DOWN),
];

/// Player 1's controllers, opened at the first poll.
#[derive(Default)]
pub(super) struct Devices {
    pad: Option<Pad>,
    last: Snapshot,
    /// When the vibration playing now ends.
    rumble_until: Option<std::time::Instant>,
}

impl Pads {
    pub(super) fn read(&mut self) -> PadState {
        let snapshot = self.devices.pad.get_or_insert_with(Pad::new).update();
        self.devices.last = snapshot;
        // Labels by name (A, B, X, Y): the Xbox set of names matches this mapping.
        self.style = GlyphStyle::Xbox;
        if !snapshot.connected {
            return PadState::default();
        }
        let mut state = PadState::default();
        for (button, bit) in BUTTONS {
            state.set(button, snapshot.buttons & bit != 0);
        }
        state.left = snapshot.sticks[0];
        state.right = snapshot.sticks[1];
        // ZL and ZR are digital on every Switch controller.
        let pulled = |bit: u64| if snapshot.buttons & bit != 0 { 1.0 } else { 0.0 };
        state.triggers = [pulled(nx::ZL), pulled(nx::ZR)];
        state
    }
}

impl Game<'_> {
    pub(super) fn pad_output_off(&mut self) {
        self.pad_stop_rumble();
    }
    /// Ends a vibration whose time is up.
    pub(super) fn pad_output_frame(&mut self) {
        if self
            .pad
            .devices
            .rumble_until
            .is_some_and(|until| std::time::Instant::now() >= until)
        {
            self.pad_stop_rumble();
        }
    }
    pub(in crate::game) fn pad_close_output(&mut self) {
        self.pad_stop_rumble();
    }
    /// HD Rumble: the heavy motor on the low band, the light one on the high band.
    pub(super) fn pad_vibrate(&mut self, _: Rumble, pulse: Pulse) {
        let last = self.pad.devices.last;
        if !last.connected {
            return;
        }
        horizon_nx::pad::rumble(last.handheld, last.styles, pulse.strong, pulse.weak);
        self.pad.devices.rumble_until = Some(
            std::time::Instant::now() + std::time::Duration::from_millis(u64::from(pulse.millis)),
        );
    }
    fn pad_stop_rumble(&mut self) {
        if self.pad.devices.rumble_until.take().is_some() {
            let last = self.pad.devices.last;
            horizon_nx::pad::rumble(last.handheld, last.styles, 0.0, 0.0);
        }
    }
    pub(super) fn pad_devices_status(&self) -> Value {
        let last = &self.pad.devices.last;
        json!({
            "backend": "libnx",
            "controllers": [{
                "name": if last.handheld { "Joy-Con (attached)" } else { "Player 1" },
                "styles": last.styles,
                "active": last.connected,
            }],
        })
    }
}
