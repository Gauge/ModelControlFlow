//! The C-ABI boundary, which is the half of D4 that needed testing.
//!
//! D4 argues that "hardware probing, accelerator interrogation and driving
//! inference engines are constant C-ABI work, and Rust pays no tax at that
//! boundary", and that this is where garbage-collected alternatives lose *for
//! this project specifically*. §7.19 asks for that to be validated rather than
//! asserted, so this module is the validation: a real vendor library, loaded
//! at runtime, called over the C ABI, with every failure classified against
//! the taxonomy.
//!
//! **Why `dlopen` rather than linking.** Linking against the vendor library
//! would make MCF unbuildable on a machine that does not have it, and
//! unrunnable on one whose driver is a different version — B36's "a missing
//! prerequisite is never the user's errand", inverted. Loading it at runtime
//! means its absence is a *reading* (`accel.driver.absent`) rather than a
//! link error, which is what §3.2 asks for: degrade, and say so.
//!
//! **Why `unsafe` is admitted here.** The workspace denies `unsafe_code` and
//! says so as a `deny` rather than a `forbid` precisely for this: the C-ABI
//! work D4 anticipates opts in per module, with the reason written at the
//! opt-in site. This is that site. The unsafety is confined to four things:
//! the three `extern "C"` declarations of the loader, the symbol casts, and
//! the calls themselves. Everything above this module sees safe Rust and a
//! classified [`Failure`].
//!
//! **What is checked before each call, because the compiler cannot.**
//! Every pointer returned by the loader is checked for null before it is used;
//! every call's status code is checked before its out-parameter is read; and
//! the buffer handed to the name query is sized from the vendor's own
//! documented maximum and is not read past the status check. The library is
//! closed on every exit path.
//!
//! **Nothing measured passes through this library.** It reports the device's
//! own state; it does not compute anything MCF publishes. That is why loading
//! it does not reopen intent v23's decision to vendor the whole *inference*
//! stack — a distinction recorded for DEC-008 rather than assumed.

// The one module in the workspace that opts in, and the reason is above.
#![allow(unsafe_code)]

use core::ffi::{c_char, c_int, c_uint, c_void};

use mcf_core::attested::Attested;
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};

use crate::accelerator::Reading;

const WHERE: Subsystem = Subsystem::new("mcf-prototype::nvml");

/// The candidate names, in the order they are tried.
///
/// The versioned soname first, because it is the one a driver installation is
/// guaranteed to provide; the development symlink second, because a machine
/// with the headers installed has it and a machine without them does not.
const CANDIDATES: [&str; 2] = ["libnvidia-ml.so.1\0", "libnvidia-ml.so\0"];

const RTLD_NOW: c_int = 2;

// The three functions the loader needs. Declared here rather than taken from a
// dependency: they are in libc, which every Rust program on this platform
// already links, and B15 admits weight only against a stated cost.
unsafe extern "C" {
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> c_int;
}

/// The vendor's own documented maximum for a device name, plus room.
const NAME_BUFFER: usize = 96;

/// Asks the vendor library what the first device is and what it is doing.
///
/// # Errors
///
/// Every way this can fail is a taxonomy category: the library absent
/// (`accel.driver.absent`), a symbol missing (`accel.driver.version_mismatch`),
/// initialization refused or a query rejected (`accel.driver.query_failed`), no
/// device present (`accel.absent`).
pub(crate) fn probe() -> Result<Reading> {
    let library = Library::open()?;

    // Every symbol is resolved before any is called, so a partially-usable
    // library is a single classified failure rather than a half-finished
    // reading (A4 is about outcomes, not about giving up halfway through one).
    let init: InitFn = library.symbol(b"nvmlInit_v2\0")?;
    let shutdown: ShutdownFn = library.symbol(b"nvmlShutdown\0")?;
    let device_count: CountFn = library.symbol(b"nvmlDeviceGetCount_v2\0")?;
    let by_index: HandleFn = library.symbol(b"nvmlDeviceGetHandleByIndex_v2\0")?;
    let name_of: NameFn = library.symbol(b"nvmlDeviceGetName\0")?;
    let memory_of: MemoryFn = library.symbol(b"nvmlDeviceGetMemoryInfo\0")?;
    let temperature_of: TemperatureFn = library.symbol(b"nvmlDeviceGetTemperature\0")?;
    let driver_version: VersionFn = library.symbol(b"nvmlSystemGetDriverVersion\0")?;

    // SAFETY: `init` came from `dlsym` on a successfully opened library and was
    // checked non-null; it takes no arguments and returns a status code.
    let status = unsafe { init() };
    if status != 0 {
        return Err(query_failed("nvmlInit_v2", status));
    }

    let reading = read_first_device(
        device_count,
        by_index,
        name_of,
        memory_of,
        temperature_of,
        driver_version,
    );

    // A27's habit: what this opened, it closes, on every path out — including
    // the one where the reading failed.
    // SAFETY: `shutdown` came from `dlsym` on the same open library and was
    // checked non-null; it takes no arguments.
    let _status = unsafe { shutdown() };
    reading
}

fn read_first_device(
    device_count: CountFn,
    by_index: HandleFn,
    name_of: NameFn,
    memory_of: MemoryFn,
    temperature_of: TemperatureFn,
    driver_version: VersionFn,
) -> Result<Reading> {
    let mut count: c_uint = 0;
    // SAFETY: `count` is a live, aligned `c_uint` this frame owns, and the
    // vendor's contract is that the callee writes to it only on success.
    let status = unsafe { device_count(&raw mut count) };
    if status != 0 {
        return Err(query_failed("nvmlDeviceGetCount_v2", status));
    }
    if count == 0 {
        return Err(Failure::new(
            Category::AccelAbsent,
            Attribution::Machine,
            Disposition::Degraded,
            WHERE,
            "the vendor library initialized and reports no devices",
        )
        .with_context("route", "vendor-library"));
    }

    let mut device: *mut c_void = core::ptr::null_mut();
    // SAFETY: `device` is a live pointer slot this frame owns; the callee
    // writes a handle into it only when it returns success.
    let status = unsafe { by_index(0, &raw mut device) };
    if status != 0 {
        return Err(query_failed("nvmlDeviceGetHandleByIndex_v2", status));
    }
    if device.is_null() {
        return Err(query_failed("nvmlDeviceGetHandleByIndex_v2", -1));
    }

    let mut reading = Reading {
        route: "vendor-library",
        vendor: Attested::Known("NVIDIA".to_owned()),
        model: Attested::Unknown,
        driver: Attested::Unknown,
        memory_bytes: Attested::Unknown,
        temperature_c: Attested::Unknown,
    };

    let mut name = [0_u8; NAME_BUFFER];
    // SAFETY: the buffer is `NAME_BUFFER` bytes and that same length is passed,
    // so the callee cannot write past it. `device` was checked non-null.
    let status = unsafe {
        name_of(
            device,
            name.as_mut_ptr().cast::<c_char>(),
            c_uint::try_from(NAME_BUFFER).unwrap_or(c_uint::MAX),
        )
    };
    if status == 0 {
        reading.model = Attested::Known(from_c_bytes(&name));
    }

    let mut memory = Memory {
        total: 0,
        free: 0,
        used: 0,
    };
    // SAFETY: `memory` is a live, correctly-laid-out `nvmlMemory_t` this frame
    // owns; the callee fills it only on success.
    let status = unsafe { memory_of(device, &raw mut memory) };
    if status == 0 {
        reading.memory_bytes = Attested::Known(memory.total);
    }

    let mut celsius: c_uint = 0;
    // SAFETY: sensor 0 is the vendor's documented GPU sensor; `celsius` is a
    // live slot this frame owns.
    let status = unsafe { temperature_of(device, 0, &raw mut celsius) };
    if status == 0 {
        reading.temperature_c = Attested::Known(celsius);
    }

    let mut version = [0_u8; NAME_BUFFER];
    // SAFETY: as for the name query.
    let status = unsafe {
        driver_version(
            version.as_mut_ptr().cast::<c_char>(),
            c_uint::try_from(NAME_BUFFER).unwrap_or(c_uint::MAX),
        )
    };
    if status == 0 {
        reading.driver = Attested::Known(from_c_bytes(&version));
    }

    Ok(reading)
}

/// The bytes before the first NUL, as text.
///
/// A truncated or non-UTF-8 name yields what was readable rather than an
/// error: A4, at the smallest possible scale.
fn from_c_bytes(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(bytes.split_at(end).0).into_owned()
}

fn query_failed(call: &'static str, status: c_int) -> Failure {
    Failure::new(
        Category::AccelDriverQueryFailed,
        Attribution::Machine,
        Disposition::Degraded,
        WHERE,
        "the vendor library refused a query",
    )
    .with_context("call", call)
    .with_context("status", status.to_string())
}

type InitFn = unsafe extern "C" fn() -> c_int;
type ShutdownFn = unsafe extern "C" fn() -> c_int;
type CountFn = unsafe extern "C" fn(*mut c_uint) -> c_int;
type HandleFn = unsafe extern "C" fn(c_uint, *mut *mut c_void) -> c_int;
type NameFn = unsafe extern "C" fn(*mut c_void, *mut c_char, c_uint) -> c_int;
type MemoryFn = unsafe extern "C" fn(*mut c_void, *mut Memory) -> c_int;
type TemperatureFn = unsafe extern "C" fn(*mut c_void, c_uint, *mut c_uint) -> c_int;
type VersionFn = unsafe extern "C" fn(*mut c_char, c_uint) -> c_int;

/// The vendor's `nvmlMemory_t`: three 64-bit byte counts, in this order.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Memory {
    total: u64,
    free: u64,
    used: u64,
}

/// An open handle to the vendor library, closed when it goes out of scope.
struct Library {
    handle: *mut c_void,
}

impl Library {
    fn open() -> Result<Self> {
        for candidate in CANDIDATES {
            // SAFETY: the candidate is a `&'static str` with an explicit
            // trailing NUL, so it is a valid C string for the duration of the
            // call.
            let handle = unsafe { dlopen(candidate.as_ptr().cast::<c_char>(), RTLD_NOW) };
            if !handle.is_null() {
                return Ok(Self { handle });
            }
        }
        Err(Failure::new(
            Category::AccelDriverAbsent,
            Attribution::Machine,
            Disposition::Degraded,
            WHERE,
            "the vendor management library is not present on this machine",
        )
        .with_context("route", "vendor-library")
        .with_context("tried", CANDIDATES.join(", ").replace('\0', "")))
    }

    fn symbol<T: Copy>(&self, name: &'static [u8]) -> Result<T> {
        assert!(
            core::mem::size_of::<T>() == core::mem::size_of::<*mut c_void>(),
            "a symbol is transmuted to a function pointer, which is pointer-sized"
        );
        // SAFETY: `name` is a byte string with an explicit trailing NUL, and
        // `self.handle` is non-null by construction.
        let symbol = unsafe { dlsym(self.handle, name.as_ptr().cast::<c_char>()) };
        if symbol.is_null() {
            return Err(Failure::new(
                Category::AccelDriverVersionMismatch,
                Attribution::Machine,
                Disposition::Degraded,
                WHERE,
                "the vendor library does not export a symbol this probe needs",
            )
            .with_context("symbol", String::from_utf8_lossy(name).replace('\0', "")));
        }
        // SAFETY: the size assertion above establishes that `T` is
        // pointer-sized, and every `T` this is instantiated with is an
        // `extern "C"` function pointer whose signature matches the vendor's
        // documented one for `name`.
        Ok(unsafe { core::mem::transmute_copy::<*mut c_void, T>(&symbol) })
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
