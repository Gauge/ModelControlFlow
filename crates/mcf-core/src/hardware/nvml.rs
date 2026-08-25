//! The route that asks the vendor's management library, over the C ABI.
//!
//! F1 in `doc/findings.md` established why this route exists rather than being
//! a nicety: the files a driver publishes give a device's *identity* and none
//! of its *live state*, and live state is exactly what §3.8 needs in order to
//! tell "this model is slow" from "this machine was busy". Under D25 a machine
//! with only the file route reports every device as *attempted,
//! uncharacterized* — correctly.
//!
//! **Loaded at runtime, never linked and never shipped.** Linking would make
//! MCF unbuildable on a machine without the library and unrunnable on one whose
//! driver is a different version, which inverts B36. Loading it means its
//! absence is a *reading* — the route finds nothing — rather than a link error,
//! which is §3.2's degrade-and-say-so. Nothing MCF publishes is computed
//! through this library: it reports the device's own state, so intent v23's
//! decision to vendor the whole *inference* stack is untouched (D25 states that
//! boundary).
//!
//! **Why `unsafe` is admitted here.** The workspace denies `unsafe_code` as a
//! `deny` rather than a `forbid` precisely so that the C-ABI work D4
//! anticipates can opt in per module with the reason at the site. This is that
//! site, and the unsafety is confined to three `extern "C"` loader
//! declarations, the symbol casts, and the calls. Everything above sees safe
//! Rust and an [`Attested`] reading.
//!
//! **What is checked, because the compiler cannot.** Every pointer the loader
//! returns is checked for null before use; every call's status is checked
//! before its out-parameter is read; the name buffers are sized from the
//! vendor's documented maximum and the same length is passed to the callee; and
//! the library is closed exactly once, on every path out.

// The one module in the workspace that opts in; the reason is above.
#![allow(unsafe_code)]

use core::ffi::{c_char, c_int, c_uint, c_void};

use crate::attested::Attested;
use crate::measurement::Bytes;

use super::accelerator::{Missing, Reading, Route};

/// Asks `libnvidia-ml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct VendorLibrary;

impl Route for VendorLibrary {
    fn name(&self) -> &'static str {
        "vendor-library"
    }

    fn covers(&self) -> &'static [Missing] {
        &Missing::ALL
    }

    fn probe(&self) -> Vec<Reading> {
        read_devices().unwrap_or_default()
    }
}

/// The versioned soname first, because a driver installation always provides
/// it; the development symlink second, because only a machine with the headers
/// has one.
const CANDIDATES: [&str; 2] = ["libnvidia-ml.so.1\0", "libnvidia-ml.so\0"];
const RTLD_NOW: c_int = 2;
/// The vendor's documented maximum for a name, with room.
const NAME_BUFFER: usize = 96;
/// The vendor's sensor index for the device itself.
const SENSOR_GPU: c_uint = 0;

// In libc, which every Rust program on this platform already links. Declared
// here rather than taken from a dependency, because B15 admits weight only
// against a stated cost and three declarations are cheaper than a crate.
unsafe extern "C" {
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> c_int;
}

type InitFn = unsafe extern "C" fn() -> c_int;
type ShutdownFn = unsafe extern "C" fn() -> c_int;
type CountFn = unsafe extern "C" fn(*mut c_uint) -> c_int;
type HandleFn = unsafe extern "C" fn(c_uint, *mut *mut c_void) -> c_int;
type NameFn = unsafe extern "C" fn(*mut c_void, *mut c_char, c_uint) -> c_int;
type MemoryFn = unsafe extern "C" fn(*mut c_void, *mut DeviceMemory) -> c_int;
type TemperatureFn = unsafe extern "C" fn(*mut c_void, c_uint, *mut c_uint) -> c_int;
type VersionFn = unsafe extern "C" fn(*mut c_char, c_uint) -> c_int;

/// The vendor's `nvmlMemory_t`: three 64-bit byte counts, in this order.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct DeviceMemory {
    total: u64,
    free: u64,
    used: u64,
}

/// Every device the library reports, or `None` if the route could not run.
///
/// `None` and an empty vector are different answers and both are correct: the
/// first is *this route did not work here*, the second is *it worked and there
/// are no devices*. Callers see them collapsed, because [`Route::probe`]
/// returns a list either way and a route that found nothing has found nothing
/// — but the distinction is preserved here so that a future surface can report
/// it (A9).
fn read_devices() -> Option<Vec<Reading>> {
    let library = Library::open()?;

    // Every symbol is resolved before any is called, so a partially-usable
    // library yields nothing rather than a half-taken reading.
    let init: InitFn = library.symbol(b"nvmlInit_v2\0")?;
    let shutdown: ShutdownFn = library.symbol(b"nvmlShutdown\0")?;
    let device_count: CountFn = library.symbol(b"nvmlDeviceGetCount_v2\0")?;
    let by_index: HandleFn = library.symbol(b"nvmlDeviceGetHandleByIndex_v2\0")?;
    let name_of: NameFn = library.symbol(b"nvmlDeviceGetName\0")?;
    let memory_of: MemoryFn = library.symbol(b"nvmlDeviceGetMemoryInfo\0")?;
    let temperature_of: TemperatureFn = library.symbol(b"nvmlDeviceGetTemperature\0")?;
    let driver_of: VersionFn = library.symbol(b"nvmlSystemGetDriverVersion\0")?;
    let runtime_of: VersionFn = library.symbol(b"nvmlSystemGetNVMLVersion\0")?;

    // SAFETY: `init` came from `dlsym` on an open library and was checked
    // non-null; it takes no arguments and returns a status.
    if unsafe { init() } != 0 {
        return None;
    }

    let driver = attest(text_from(driver_of));
    let runtime = attest(text_from(runtime_of));

    let mut count: c_uint = 0;
    // SAFETY: `count` is a live `c_uint` this frame owns; the callee writes it
    // only on success.
    let counted = unsafe { device_count(&raw mut count) } == 0;

    let mut readings = Vec::new();
    if counted {
        for index in 0..count {
            if let Some(reading) = read_device(index, by_index, name_of, memory_of, temperature_of)
            {
                readings.push(Reading {
                    driver: driver.clone(),
                    runtime: runtime.clone(),
                    ..reading
                });
            }
        }
    }

    // A27's habit: what this opened, it closes, on every path out.
    // SAFETY: `shutdown` came from `dlsym` on the same open library.
    let _status = unsafe { shutdown() };
    counted.then_some(readings)
}

fn read_device(
    index: c_uint,
    by_index: HandleFn,
    name_of: NameFn,
    memory_of: MemoryFn,
    temperature_of: TemperatureFn,
) -> Option<Reading> {
    let mut device: *mut c_void = core::ptr::null_mut();
    // SAFETY: `device` is a live pointer slot this frame owns; the callee
    // writes a handle into it only on success.
    if unsafe { by_index(index, &raw mut device) } != 0 || device.is_null() {
        return None;
    }

    let mut reading = Reading {
        vendor: Attested::Known("NVIDIA".to_owned()),
        ..Reading::default()
    };

    let mut name = [0_u8; NAME_BUFFER];
    // SAFETY: the buffer is `NAME_BUFFER` bytes and that length is passed, so
    // the callee cannot write past it; `device` was checked non-null.
    let status = unsafe {
        name_of(
            device,
            name.as_mut_ptr().cast::<c_char>(),
            c_uint::try_from(NAME_BUFFER).unwrap_or(c_uint::MAX),
        )
    };
    if status == 0 {
        reading.model = attest(text(&name));
    }

    let mut memory = DeviceMemory {
        total: 0,
        free: 0,
        used: 0,
    };
    // SAFETY: `memory` is a live, correctly-laid-out `nvmlMemory_t` this frame
    // owns; the callee fills it only on success.
    if unsafe { memory_of(device, &raw mut memory) } == 0 {
        reading.memory_total = Attested::Known(Bytes(memory.total));
        reading.memory_available = Attested::Known(Bytes(memory.free));
    }

    let mut celsius: c_uint = 0;
    // SAFETY: `SENSOR_GPU` is the vendor's documented sensor index for the
    // device; `celsius` is a live slot this frame owns.
    if unsafe { temperature_of(device, SENSOR_GPU, &raw mut celsius) } == 0 {
        reading.temperature_c = Attested::Known(celsius);
    }

    Some(reading)
}

fn text_from(query: VersionFn) -> Option<String> {
    let mut buffer = [0_u8; NAME_BUFFER];
    // SAFETY: the buffer is `NAME_BUFFER` bytes and that length is passed.
    let status = unsafe {
        query(
            buffer.as_mut_ptr().cast::<c_char>(),
            c_uint::try_from(NAME_BUFFER).unwrap_or(c_uint::MAX),
        )
    };
    (status == 0).then(|| text(&buffer)).flatten()
}

/// The bytes before the first NUL, as text, or `None` if there are none.
fn text(bytes: &[u8]) -> Option<String> {
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    let text = String::from_utf8_lossy(bytes.split_at(end).0).into_owned();
    (!text.is_empty()).then_some(text)
}

fn attest<T>(value: Option<T>) -> Attested<T> {
    match value {
        Some(value) => Attested::Known(value),
        None => Attested::Unknown,
    }
}

/// An open handle to the vendor library, closed when it goes out of scope.
struct Library {
    handle: *mut c_void,
}

impl Library {
    fn open() -> Option<Self> {
        for candidate in CANDIDATES {
            // SAFETY: the candidate is a `&'static str` with an explicit
            // trailing NUL, so it is a valid C string for the call's duration.
            let handle = unsafe { dlopen(candidate.as_ptr().cast::<c_char>(), RTLD_NOW) };
            if !handle.is_null() {
                return Some(Self { handle });
            }
        }
        None
    }

    fn symbol<T: Copy>(&self, name: &'static [u8]) -> Option<T> {
        assert!(
            size_of::<T>() == size_of::<*mut c_void>(),
            "a symbol is copied into a function pointer, which is pointer-sized"
        );
        // SAFETY: `name` is a byte string with an explicit trailing NUL and
        // `self.handle` is non-null by construction.
        let symbol = unsafe { dlsym(self.handle, name.as_ptr().cast::<c_char>()) };
        if symbol.is_null() {
            return None;
        }
        // SAFETY: the assertion above establishes that `T` is pointer-sized,
        // and every `T` this is instantiated with is an `extern "C"` function
        // pointer whose signature matches the vendor's documented one for
        // `name`.
        Some(unsafe { core::mem::transmute_copy::<*mut c_void, T>(&symbol) })
    }
}

impl Drop for Library {
    fn drop(&mut self) {
        // SAFETY: `self.handle` came from a successful `dlopen` and is closed
        // exactly once, here.
        let _status = unsafe { dlclose(self.handle) };
    }
}

impl core::fmt::Debug for Library {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Library(<vendor management library>)")
    }
}
