use std::cell::Cell;
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use crate::error::EventLoopError;
use crate::event::{self, ElementState, MouseButton, StartCause, TouchPhase};
use crate::event_loop::{self, ControlFlow, DeviceEvents};
use crate::window::{
    CustomCursor as RootCustomCursor, CustomCursorSource, Theme, WindowId as RootWindowId,
};

use super::{ffi, DeviceId, MonitorHandle, PlatformSpecificEventLoopAttributes, WindowId};

/// State the window and the loop share (the window may live on another thread).
#[derive(Default)]
pub(crate) struct Shared {
    pub(super) window_open: AtomicBool,
    pub(super) pending_create: AtomicBool,
    pub(super) pending_destroy: AtomicBool,
    pub(super) redraw: AtomicBool,
    pub(super) focused: AtomicBool,
    wake: AtomicBool,
}

pub struct EventLoop<T> {
    window_target: event_loop::ActiveEventLoop,
    user_events_sender: mpsc::Sender<T>,
    user_events_receiver: mpsc::Receiver<T>,
}

/// The touch screen as the mouse: the first finger moves the cursor and holds the left button.
#[derive(Default)]
struct Touch {
    sampling: u64,
    finger: Option<u32>,
}

impl<T: 'static> EventLoop<T> {
    pub(crate) fn new(_: &PlatformSpecificEventLoopAttributes) -> Result<Self, EventLoopError> {
        let (user_events_sender, user_events_receiver) = mpsc::channel();
        // SAFETY: plain libnx service setup. libnx does not open hid on its own; every input
        // read (touch here, the controllers in the game) needs it.
        let rc = unsafe { ffi::hidInitialize() };
        if rc != 0 {
            return Err(EventLoopError::Os(os_error!(super::OsError("hidInitialize failed"))));
        }
        unsafe { ffi::hidInitializeTouchScreen() };
        Ok(Self {
            window_target: event_loop::ActiveEventLoop {
                p: ActiveEventLoop {
                    control_flow: Cell::new(ControlFlow::default()),
                    exit: Cell::new(false),
                    shared: Arc::new(Shared::default()),
                },
                _marker: PhantomData,
            },
            user_events_sender,
            user_events_receiver,
        })
    }

    pub fn run<F>(self, mut handler: F) -> Result<(), EventLoopError>
    where
        F: FnMut(event::Event<T>, &event_loop::ActiveEventLoop),
    {
        let target = &self.window_target;
        let shared = target.p.shared.clone();
        let window_id = RootWindowId(WindowId(1));
        let device_id = event::DeviceId(DeviceId);
        let window_event =
            |handler: &mut F, event: event::WindowEvent| {
                handler(event::Event::WindowEvent { window_id, event }, target)
            };
        let mut start_cause = StartCause::Init;
        let mut touch = Touch::default();
        let mut focused = false;
        let mut queued: VecDeque<event::WindowEvent> = VecDeque::new();

        loop {
            handler(event::Event::NewEvents(start_cause), target);
            if start_cause == StartCause::Init {
                handler(event::Event::Resumed, target);
            }

            // SAFETY: the applet loop pumps system messages; false once the system asks the
            // program to exit.
            let running = unsafe { ffi::appletMainLoop() };

            if shared.pending_create.swap(false, Ordering::SeqCst) {
                window_event(&mut handler, event::WindowEvent::Resized(MonitorHandle.size()));
                window_event(&mut handler, event::WindowEvent::Moved((0, 0).into()));
                shared.redraw.store(true, Ordering::SeqCst);
            }
            if shared.pending_destroy.swap(false, Ordering::SeqCst) {
                window_event(&mut handler, event::WindowEvent::Destroyed);
            }

            if shared.window_open.load(Ordering::SeqCst) {
                // SAFETY: a plain applet query.
                let now_focused =
                    unsafe { ffi::appletGetFocusState() } == ffi::FOCUS_IN_FOCUS;
                if now_focused != focused {
                    focused = now_focused;
                    shared.focused.store(focused, Ordering::SeqCst);
                    window_event(&mut handler, event::WindowEvent::Focused(focused));
                }
                poll_touch(&mut touch, device_id, &mut queued);
                while let Some(event) = queued.pop_front() {
                    window_event(&mut handler, event);
                }
                if !running {
                    window_event(&mut handler, event::WindowEvent::CloseRequested);
                }
            }

            while let Ok(event) = self.user_events_receiver.try_recv() {
                handler(event::Event::UserEvent(event), target);
            }

            if shared.window_open.load(Ordering::SeqCst)
                && shared.redraw.swap(false, Ordering::SeqCst)
            {
                window_event(&mut handler, event::WindowEvent::RedrawRequested);
            }

            handler(event::Event::AboutToWait, target);

            if target.p.exiting() || !running {
                break;
            }

            start_cause = match target.p.control_flow() {
                ControlFlow::Poll => StartCause::Poll,
                ControlFlow::Wait => wait(&shared, None),
                ControlFlow::WaitUntil(instant) => wait(&shared, Some(instant)),
            };
        }

        handler(event::Event::LoopExiting, target);
        Ok(())
    }

    pub fn window_target(&self) -> &event_loop::ActiveEventLoop {
        &self.window_target
    }

    pub fn create_proxy(&self) -> EventLoopProxy<T> {
        EventLoopProxy {
            user_events_sender: self.user_events_sender.clone(),
            shared: self.window_target.p.shared.clone(),
        }
    }
}

/// Sleeps in short steps until the deadline, a redraw request or a proxy event, polling the
/// system at least every few milliseconds so touch and applet messages stay live.
fn wait(shared: &Shared, until: Option<Instant>) -> StartCause {
    const STEP: Duration = Duration::from_millis(4);
    let start = Instant::now();
    let limit = start + Duration::from_millis(16);
    loop {
        if shared.redraw.load(Ordering::SeqCst) || shared.wake.swap(false, Ordering::SeqCst) {
            return StartCause::WaitCancelled { start, requested_resume: until };
        }
        let now = Instant::now();
        if let Some(requested_resume) = until {
            if now >= requested_resume {
                return StartCause::ResumeTimeReached { start, requested_resume };
            }
        }
        if now >= limit {
            return StartCause::WaitCancelled { start, requested_resume: until };
        }
        std::thread::sleep(STEP);
    }
}

fn poll_touch(touch: &mut Touch, device_id: event::DeviceId, out: &mut VecDeque<event::WindowEvent>) {
    let mut state = ffi::HidTouchScreenState::default();
    // SAFETY: one state into a valid buffer.
    if unsafe { ffi::hidGetTouchScreenStates(&mut state, 1) } == 0
        || state.sampling_number == touch.sampling
    {
        return;
    }
    touch.sampling = state.sampling_number;
    let count = state.count.clamp(0, 16) as usize;
    let touches = &state.touches[..count];
    let current = touch.finger.and_then(|id| touches.iter().find(|t| t.finger_id == id));
    match (touch.finger, current, touches.first()) {
        (Some(_), Some(t), _) => {
            out.push_back(cursor(device_id, t));
            out.push_back(event::WindowEvent::Touch(finger(device_id, t, TouchPhase::Moved)));
        },
        (Some(id), None, _) => {
            touch.finger = None;
            out.push_back(event::WindowEvent::MouseInput {
                device_id,
                state: ElementState::Released,
                button: MouseButton::Left,
            });
            let ended = ffi::HidTouchState { finger_id: id, ..Default::default() };
            out.push_back(event::WindowEvent::Touch(finger(device_id, &ended, TouchPhase::Ended)));
        },
        (None, _, Some(t)) => {
            touch.finger = Some(t.finger_id);
            out.push_back(event::WindowEvent::CursorEntered { device_id });
            out.push_back(cursor(device_id, t));
            out.push_back(event::WindowEvent::MouseInput {
                device_id,
                state: ElementState::Pressed,
                button: MouseButton::Left,
            });
            out.push_back(event::WindowEvent::Touch(finger(device_id, t, TouchPhase::Started)));
        },
        (None, _, None) => {},
    }
}

fn cursor(device_id: event::DeviceId, t: &ffi::HidTouchState) -> event::WindowEvent {
    event::WindowEvent::CursorMoved { device_id, position: (t.x as f64, t.y as f64).into() }
}

fn finger(device_id: event::DeviceId, t: &ffi::HidTouchState, phase: TouchPhase) -> event::Touch {
    event::Touch {
        device_id,
        phase,
        location: (t.x as f64, t.y as f64).into(),
        force: None,
        id: t.finger_id as u64,
    }
}

pub struct EventLoopProxy<T: 'static> {
    user_events_sender: mpsc::Sender<T>,
    shared: Arc<Shared>,
}

impl<T> EventLoopProxy<T> {
    pub fn send_event(&self, event: T) -> Result<(), event_loop::EventLoopClosed<T>> {
        self.user_events_sender
            .send(event)
            .map_err(|mpsc::SendError(x)| event_loop::EventLoopClosed(x))?;
        self.shared.wake.store(true, Ordering::SeqCst);
        Ok(())
    }
}

impl<T> Clone for EventLoopProxy<T> {
    fn clone(&self) -> Self {
        Self { user_events_sender: self.user_events_sender.clone(), shared: self.shared.clone() }
    }
}

impl<T> Unpin for EventLoopProxy<T> {}

pub struct ActiveEventLoop {
    control_flow: Cell<ControlFlow>,
    exit: Cell<bool>,
    pub(super) shared: Arc<Shared>,
}

impl ActiveEventLoop {
    pub fn create_custom_cursor(&self, source: CustomCursorSource) -> RootCustomCursor {
        let _ = source.inner;
        RootCustomCursor { inner: super::PlatformCustomCursor }
    }

    pub fn primary_monitor(&self) -> Option<MonitorHandle> {
        Some(MonitorHandle)
    }

    pub fn available_monitors(&self) -> VecDeque<MonitorHandle> {
        VecDeque::from([MonitorHandle])
    }

    #[inline]
    pub fn listen_device_events(&self, _allowed: DeviceEvents) {}

    #[cfg(feature = "rwh_05")]
    #[inline]
    pub fn raw_display_handle_rwh_05(&self) -> rwh_05::RawDisplayHandle {
        rwh_05::RawDisplayHandle::Android(rwh_05::AndroidDisplayHandle::empty())
    }

    #[inline]
    pub fn system_theme(&self) -> Option<Theme> {
        Some(Theme::Dark)
    }

    #[cfg(feature = "rwh_06")]
    #[inline]
    pub fn raw_display_handle_rwh_06(
        &self,
    ) -> Result<rwh_06::RawDisplayHandle, rwh_06::HandleError> {
        Ok(rwh_06::RawDisplayHandle::Android(rwh_06::AndroidDisplayHandle::new()))
    }

    pub fn set_control_flow(&self, control_flow: ControlFlow) {
        self.control_flow.set(control_flow)
    }

    pub fn control_flow(&self) -> ControlFlow {
        self.control_flow.get()
    }

    pub(crate) fn exit(&self) {
        self.exit.set(true);
    }

    pub(crate) fn exiting(&self) -> bool {
        self.exit.get()
    }

    pub(crate) fn owned_display_handle(&self) -> OwnedDisplayHandle {
        OwnedDisplayHandle
    }
}

#[derive(Clone)]
pub(crate) struct OwnedDisplayHandle;

impl OwnedDisplayHandle {
    #[cfg(feature = "rwh_05")]
    #[inline]
    pub fn raw_display_handle_rwh_05(&self) -> rwh_05::RawDisplayHandle {
        rwh_05::AndroidDisplayHandle::empty().into()
    }

    #[cfg(feature = "rwh_06")]
    #[inline]
    pub fn raw_display_handle_rwh_06(
        &self,
    ) -> Result<rwh_06::RawDisplayHandle, rwh_06::HandleError> {
        Ok(rwh_06::AndroidDisplayHandle::new().into())
    }
}
