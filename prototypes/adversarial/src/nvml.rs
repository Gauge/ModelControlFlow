#![allow(unsafe_code)]

use core::ffi::{c_char, c_int, c_uint, c_void};

use mcf_core::attested::Attested;
use mcf_core::failure::{Attribution, Category, Disposition, Failure, Result, Subsystem};

use crate::accelerator::Reading;

const WHERE: Subsystem = Subsystem::new("mcf-prototype::nvml");

const CANDIDATES: [&str; 2] = ["libnvidia-ml.so.1\0", "libnvidia-ml.so\0"];

const RTLD_NOW: c_int = 2;

unsafe extern "C" {
    fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> c_int;
}

const NAME_BUFFER: usize = 96;

pub(crate) fn probe() -> Result<Reading> {
    let library = Library::open()?;

    let init: InitFn = library.symbol(b"nvmlInit_v2\0")?;
    let shutdown: ShutdownFn = library.symbol(b"nvmlShutdown\0")?;
    let device_count: CountFn = library.symbol(b"nvmlDeviceGetCount_v2\0")?;
    let by_index: HandleFn = library.symbol(b"nvmlDeviceGetHandleByIndex_v2\0")?;
    let name_of: NameFn = library.symbol(b"nvmlDeviceGetName\0")?;
    let memory_of: MemoryFn = library.symbol(b"nvmlDeviceGetMemoryInfo\0")?;
    let temperature_of: TemperatureFn = library.symbol(b"nvmlDeviceGetTemperature\0")?;
    let driver_version: VersionFn = library.symbol(b"nvmlSystemGetDriverVersion\0")?;

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
    let status = unsafe { memory_of(device, &raw mut memory) };
    if status == 0 {
        reading.memory_bytes = Attested::Known(memory.total);
    }

    let mut celsius: c_uint = 0;
    let status = unsafe { temperature_of(device, 0, &raw mut celsius) };
    if status == 0 {
        reading.temperature_c = Attested::Known(celsius);
    }

    let mut version = [0_u8; NAME_BUFFER];
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

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Memory {
    total: u64,
    free: u64,
    used: u64,
}

struct Library {
    handle: *mut c_void,
}

impl Library {
    fn open() -> Result<Self> {
        for candidate in CANDIDATES {
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
        Ok(unsafe { core::mem::transmute_copy::<*mut c_void, T>(&symbol) })
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
