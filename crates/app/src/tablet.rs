//! Pen tablets through Wintab ([decision 0184](../../../docs/decisions/0184-brush-feel.md)):
//! Wacom's interface, which Wacom tablets (and most others) offer on
//! Windows, read for the pen's pressure, tilt and eraser end.
//!
//! The app polls the tablet's packet queue once a frame (winit doesn't
//! pass Wintab's window messages on), and hands the latest pen state to
//! the workbench. When `Wintab32.dll` isn't there (no tablet driver), or
//! on other systems, there is no tablet: pens then reach the app through
//! the system as touch with force, which the workbench reads itself.

use ui::PenState;

/// The tablet, once opened.
#[derive(Default)]
pub struct Tablet {
    /// Whether opening was tried (only once: a missing driver stays
    /// missing).
    tried: bool,
    #[cfg(windows)]
    context: Option<wintab::Context>,
}

impl Tablet {
    /// The pen's state now, if a tablet pen is near the tablet. Opens the
    /// tablet for the window of `frame` the first time.
    pub fn poll(&mut self, frame: &eframe::Frame) -> Option<PenState> {
        #[cfg(windows)]
        {
            if !self.tried {
                self.tried = true;
                self.context = wintab::window_of(frame).and_then(wintab::Context::open);
            }
            self.context.as_mut()?.poll()
        }
        #[cfg(not(windows))]
        {
            let _ = frame;
            self.tried = true;
            None
        }
    }
}

#[cfg(windows)]
mod wintab {
    use libloading::Library;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use std::ffi::c_void;
    use ui::PenState;
    use wintab_lite::{AXIS, CXO, DVC, LOGCONTEXT, Packet, TPS, WTI, WTPKT};

    type InfoFn = unsafe extern "system" fn(u32, u32, *mut c_void) -> u32;
    type OpenFn = unsafe extern "system" fn(*mut c_void, *mut LOGCONTEXT, i32) -> *mut c_void;
    type CloseFn = unsafe extern "system" fn(*mut c_void) -> i32;
    type QueueFn = unsafe extern "system" fn(*mut c_void, *mut u32, *mut u32) -> i32;
    type DataGetFn =
        unsafe extern "system" fn(*mut c_void, u32, u32, i32, *mut c_void, *mut i32) -> i32;

    /// The most packets read in one frame.
    const PACKETS: usize = 64;

    /// The window's handle, for opening the tablet.
    pub(super) fn window_of(frame: &eframe::Frame) -> Option<*mut c_void> {
        match frame.window_handle().ok()?.as_raw() {
            RawWindowHandle::Win32(handle) => Some(handle.hwnd.get() as *mut c_void),
            _ => None,
        }
    }

    /// An open Wintab context.
    pub(super) struct Context {
        // Kept so the functions stay loaded.
        _library: Library,
        close: CloseFn,
        queue: QueueFn,
        data_get: DataGetFn,
        handle: *mut c_void,
        /// The pressure's top reading.
        most_pressure: f32,
        /// The latest state, kept between packets.
        latest: Option<PenState>,
    }

    impl Context {
        /// Opens the tablet for window `hwnd`, if Wintab and a tablet are
        /// there.
        pub(super) fn open(hwnd: *mut c_void) -> Option<Self> {
            // SAFETY: loading the system's Wintab library, whose functions
            // have the signatures declared above (Wintab's documented C
            // interface); every buffer passed is as large as Wintab writes.
            unsafe {
                let library = Library::new("Wintab32.dll").ok()?;
                let info: InfoFn = *library.get::<InfoFn>(b"WTInfoA\0").ok()?;
                let open: OpenFn = *library.get::<OpenFn>(b"WTOpenA\0").ok()?;
                let close: CloseFn = *library.get::<CloseFn>(b"WTClose\0").ok()?;
                let queue: QueueFn = *library.get::<QueueFn>(b"WTQueuePacketsEx\0").ok()?;
                let data_get: DataGetFn = *library.get::<DataGetFn>(b"WTDataGet\0").ok()?;
                let mut context = LOGCONTEXT::default();
                if info(
                    WTI::DEFSYSCTX as u32,
                    0,
                    (&mut context as *mut LOGCONTEXT).cast(),
                ) == 0
                {
                    return None;
                }
                let mut pressure = AXIS::default();
                let size = info(
                    WTI::DEVICES as u32,
                    DVC::NPRESSURE as u32,
                    (&mut pressure as *mut AXIS).cast(),
                );
                if size == 0 || pressure.axMax <= 0 {
                    return None;
                }
                // Every packet item, in absolute numbers, as `Packet`
                // expects; the system cursor still follows the pen.
                context.lcPktData = WTPKT::all();
                context.lcPktMode = WTPKT::empty();
                context.lcMoveMask = WTPKT::all();
                context.lcOptions |= CXO::SYSTEM;
                let handle = open(hwnd, &mut context, 1);
                if handle.is_null() {
                    return None;
                }
                Some(Self {
                    _library: library,
                    close,
                    queue,
                    data_get,
                    handle,
                    most_pressure: pressure.axMax as f32,
                    latest: None,
                })
            }
        }

        /// The pen's latest state, reading the packets since last time.
        pub(super) fn poll(&mut self) -> Option<PenState> {
            let (mut oldest, mut newest) = (0u32, 0u32);
            // SAFETY: the context is open; the buffer holds `PACKETS`
            // packets and Wintab is told so.
            let packets = unsafe {
                if (self.queue)(self.handle, &mut oldest, &mut newest) == 0 {
                    return self.latest;
                }
                let mut buffer: Vec<Packet> = (0..PACKETS).map(|_| Packet::default()).collect();
                let mut got = 0i32;
                (self.data_get)(
                    self.handle,
                    oldest,
                    newest,
                    PACKETS as i32,
                    buffer.as_mut_ptr().cast(),
                    &mut got,
                );
                buffer.truncate(got.clamp(0, PACKETS as i32) as usize);
                buffer
            };
            if let Some(last) = packets.last() {
                let status = last.pkStatus;
                self.latest = (!status.contains(TPS::PROXIMITY)).then(|| {
                    let orientation = last.pkOrientation;
                    PenState {
                        pressure: (last.pkNormalPressure as f32 / self.most_pressure)
                            .clamp(0.0, 1.0),
                        tilt: tilt(orientation.orAzimuth, orientation.orAltitude),
                        eraser: status.contains(TPS::INVERT),
                    }
                });
            }
            self.latest
        }
    }

    impl Drop for Context {
        fn drop(&mut self) {
            // SAFETY: the context was opened and is closed once.
            unsafe {
                (self.close)(self.handle);
            }
        }
    }

    /// The pen's lean from Wintab's azimuth and altitude (tenths of a
    /// degree; altitude 900 is upright): its direction across the
    /// tablet, as long as it leans (0 upright, 1 flat). `None` when the
    /// tablet doesn't report it.
    fn tilt(azimuth: i32, altitude: i32) -> Option<[f32; 2]> {
        if altitude == 0 && azimuth == 0 {
            return None;
        }
        let lean = 1.0 - (altitude.unsigned_abs() as f32 / 900.0).clamp(0.0, 1.0);
        let angle = (azimuth as f32 / 10.0).to_radians();
        Some([angle.sin() * lean, -angle.cos() * lean])
    }
}
