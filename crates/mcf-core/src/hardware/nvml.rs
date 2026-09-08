#![allow(unsafe_code)]

use core::ffi::{c_char, c_int, c_uint, c_void};

use crate::attested::Attested;
use crate::measurement::Bytes;

use super::accelerator::{Missing, Reading, Route};

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

const CANDIDATES: [&str; 2] = ["libnvidia-ml.so.1\0", "libnvidia-ml.so\0"];
const RTLD_NOW: c_int = 2;
const NAME_BUFFER: usize = 96;
const SENSOR_GPU: c_uint = 0;

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

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct DeviceMemory {
    total: u64,
    free: u64,
    used: u64,
}

fn read_devices() -> Option<Vec<Reading>> {
    let library = Library::open()?;

    let init: InitFn = library.symbol(b"nvmlInit_v2\0")?;
    let shutdown: ShutdownFn = library.symbol(b"nvmlShutdown\0")?;
    let device_count: CountFn = library.symbol(b"nvmlDeviceGetCount_v2\0")?;
    let by_index: HandleFn = library.symbol(b"nvmlDeviceGetHandleByIndex_v2\0")?;
    let name_of: NameFn = library.symbol(b"nvmlDeviceGetName\0")?;
    let memory_of: MemoryFn = library.symbol(b"nvmlDeviceGetMemoryInfo\0")?;
    let temperature_of: TemperatureFn = library.symbol(b"nvmlDeviceGetTemperature\0")?;
    let driver_of: VersionFn = library.symbol(b"nvmlSystemGetDriverVersion\0")?;
    let runtime_of: VersionFn = library.symbol(b"nvmlSystemGetNVMLVersion\0")?;

    if unsafe { init() } != 0 {
        return None;
    }

    let driver = attest(text_from(driver_of));
    let runtime = attest(text_from(runtime_of));

    let mut count: c_uint = 0;
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
    if unsafe { by_index(index, &raw mut device) } != 0 || device.is_null() {
        return None;
    }

    let mut reading = Reading {
        vendor: Attested::Known("NVIDIA".to_owned()),
        ..Reading::default()
    };

    let mut name = [0_u8; NAME_BUFFER];
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
    if unsafe { memory_of(device, &raw mut memory) } == 0 {
        reading.memory_total = Attested::Known(Bytes(memory.total));
        reading.memory_available = Attested::Known(Bytes(memory.free));
    }

    let mut celsius: c_uint = 0;
    if unsafe { temperature_of(device, SENSOR_GPU, &raw mut celsius) } == 0 {
        reading.temperature_c = Attested::Known(celsius);
    }

    Some(reading)
}

fn text_from(query: VersionFn) -> Option<String> {
    let mut buffer = [0_u8; NAME_BUFFER];
    let status = unsafe {
        query(
            buffer.as_mut_ptr().cast::<c_char>(),
            c_uint::try_from(NAME_BUFFER).unwrap_or(c_uint::MAX),
        )
    };
    (status == 0).then(|| text(&buffer)).flatten()
}

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

struct Library {
    handle: *mut c_void,
}

impl Library {
    fn open() -> Option<Self> {
        for candidate in CANDIDATES {
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
        let symbol = unsafe { dlsym(self.handle, name.as_ptr().cast::<c_char>()) };
        if symbol.is_null() {
            return None;
        }
        Some(unsafe { core::mem::transmute_copy::<*mut c_void, T>(&symbol) })
    }
}

impl Drop for Library {
    fn drop(&mut self) {
        let _status = unsafe { dlclose(self.handle) };
    }
}

impl core::fmt::Debug for Library {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Library(<vendor management library>)")
    }
}
