//! Stereo PCM output on the Switch through libnx's default AudioOut service.
use std::io;
use std::ptr;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const CHANNELS: usize = 2;
const FRAMES: usize = 1024;
const SAMPLES: usize = FRAMES * CHANNELS;
const PCM_INT16: i32 = 2;
static OUTPUT_IN_USE: AtomicBool = AtomicBool::new(false);

#[repr(C, align(4096))]
struct Samples([i16; SAMPLES]);

// AudioOut returns the address of the submitted descriptor as the buffer token.
#[repr(C)]
struct AudioOutBuffer {
    next: *mut AudioOutBuffer,
    buffer: *mut i16,
    buffer_size: u64,
    data_size: u64,
    data_offset: u64,
}

unsafe extern "C" {
    fn audoutInitialize() -> u32;
    fn audoutGetSampleRate() -> u32;
    fn audoutGetChannelCount() -> u32;
    fn audoutGetPcmFormat() -> i32;
    fn audoutStartAudioOut() -> u32;
    fn audoutStopAudioOut() -> u32;
    fn audoutAppendAudioOutBuffer(buffer: *mut AudioOutBuffer) -> u32;
    fn audoutGetReleasedAudioOutBuffer(buffer: *mut *mut AudioOutBuffer, count: *mut u32) -> u32;
    fn audoutExit();
}

fn check(result: u32, operation: &str) -> io::Result<()> {
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "{operation} failed (0x{result:08x})"
        )))
    }
}

struct Playback;
impl Playback {
    fn open() -> io::Result<Self> {
        if OUTPUT_IN_USE.swap(true, Ordering::AcqRel) {
            return Err(io::Error::other("Switch audio output is already open"));
        }
        // SAFETY: this worker exclusively owns the process-wide audout service.
        let result = unsafe { audoutInitialize() };
        if let Err(error) = check(result, "audoutInitialize") {
            OUTPUT_IN_USE.store(false, Ordering::Release);
            return Err(error);
        }
        Ok(Self)
    }
}

impl Drop for Playback {
    fn drop(&mut self) {
        // SAFETY: the service belongs to this worker, and is closed before its buffers are freed.
        unsafe {
            audoutStopAudioOut();
            audoutExit();
        }
        OUTPUT_IN_USE.store(false, Ordering::Release);
    }
}

pub struct Stream {
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl Stream {
    pub fn open(
        render: impl FnMut(&mut [f32], usize, u32) + Send + 'static,
        on_error: impl FnOnce(io::Error) + Send + 'static,
    ) -> io::Result<Self> {
        let (ready, initialized) = mpsc::sync_channel(0);
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let worker = thread::Builder::new()
            .name("Switch audio".into())
            .spawn(move || {
                let mut ready = Some(ready);
                if let Err(error) = play(&worker_stop, &mut ready, render) {
                    if let Some(ready) = ready {
                        let _ = ready.send(Err(error));
                    } else {
                        on_error(error);
                    }
                }
            })?;
        let stream = Self {
            stop,
            worker: Some(worker),
        };
        initialized
            .recv()
            .map_err(|_| io::Error::other("the Switch audio worker exited during startup"))??;
        Ok(stream)
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn play(
    stop: &AtomicBool,
    ready: &mut Option<mpsc::SyncSender<io::Result<()>>>,
    mut render: impl FnMut(&mut [f32], usize, u32),
) -> io::Result<()> {
    let mut pcm = Box::new([Samples([0; SAMPLES]), Samples([0; SAMPLES])]);
    let mut buffers = pcm.each_mut().map(|samples| AudioOutBuffer {
        next: ptr::null_mut(),
        buffer: samples.0.as_mut_ptr(),
        buffer_size: size_of::<Samples>() as u64,
        data_size: size_of::<[i16; SAMPLES]>() as u64,
        data_offset: 0,
    });
    // Drop the service before the descriptors and aligned PCM storage go out of scope.
    let _playback = Playback::open()?;
    // SAFETY: the worker has initialized audout and exclusively owns the service.
    let (rate, channels, format) = unsafe {
        (
            audoutGetSampleRate(),
            audoutGetChannelCount(),
            audoutGetPcmFormat(),
        )
    };
    if rate == 0 || channels as usize != CHANNELS || format != PCM_INT16 {
        return Err(io::Error::other(format!(
            "unsupported Switch audio format ({rate} Hz, {channels} channels, PCM {format})"
        )));
    }

    let tokens = buffers
        .each_mut()
        .map(|buffer| buffer as *mut AudioOutBuffer);
    for buffer in tokens {
        // SAFETY: the descriptor and its aligned data stay alive until audout is closed.
        check(
            unsafe { audoutAppendAudioOutBuffer(buffer) },
            "audoutAppendAudioOutBuffer",
        )?;
    }
    // SAFETY: the worker owns the initialized service and has queued both buffers.
    check(unsafe { audoutStartAudioOut() }, "audoutStartAudioOut")?;
    let Some(ready) = ready.take() else {
        return Err(io::Error::other("missing Switch audio startup channel"));
    };
    if ready.send(Ok(())).is_err() {
        return Ok(());
    }

    let mut scratch = [0.0; SAMPLES];
    while !stop.load(Ordering::Acquire) {
        let mut released = ptr::null_mut();
        let mut count = 0;
        // SAFETY: both outputs are writable; libnx returns the token for one released buffer.
        check(
            unsafe { audoutGetReleasedAudioOutBuffer(&mut released, &mut count) },
            "audoutGetReleasedAudioOutBuffer",
        )?;
        if count == 0 {
            thread::sleep(Duration::from_millis(4));
            continue;
        }
        let index = tokens
            .iter()
            .position(|&token| token == released)
            .ok_or_else(|| io::Error::other("unexpected Switch audio buffer"))?;
        render(&mut scratch, CHANNELS, rate);
        for (sample, mixed) in pcm[index].0.iter_mut().zip(scratch.iter()) {
            *sample = (mixed.clamp(-1.0, 1.0) * f32::from(i16::MAX)).round() as i16;
        }
        // SAFETY: audout has released this buffer and the worker finished updating it.
        check(
            unsafe { audoutAppendAudioOutBuffer(released) },
            "audoutAppendAudioOutBuffer",
        )?;
    }
    Ok(())
}
