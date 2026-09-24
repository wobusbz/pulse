//!“LhmNative.dll”（LibreHardwareMonitor）封装。
//!
//!通过LhmNative.dll 导出windows的硬件信息，导出的格式JSON
#![allow(dead_code)]

use crate::internal::lhm::model::Summary;
use libloading::{Library, Symbol};
use std::{
    error::Error as StdError,
    ffi::{CStr, OsStr},
    fmt,
    os::raw::{c_char, c_void},
};

///  C ABI 函数指针
type OpenFn = unsafe extern "C" fn(u32) -> i32;
type CloseFn = unsafe extern "C" fn();
type SnapshotFn = unsafe extern "C" fn() -> *mut c_char;
type ReportFn = unsafe extern "C" fn() -> *mut c_char;
type LastErrorFn = unsafe extern "C" fn() -> *mut c_char;
type FreeFn = unsafe extern "C" fn(*mut c_void);

#[derive(Debug)]
pub enum LhmError {
    LibraryLoad(String),
    MissingSymbol(String),
    OpenFailed(String),
    NullPointer(&'static str),
    InvalidJson(String),
    DeviceError(String),
}

impl fmt::Display for LhmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LibraryLoad(err) => {
                write!(f, "failed to load LhmNative.dll: {err}")
            }
            Self::MissingSymbol(name) => {
                write!(
                    f,
                    "missing export '{name}' (LhmNative.dll version mismatch?)"
                )
            }
            Self::OpenFailed(detail) => {
                write!(
                    f,
                    "lhm_open() failed (try running as Administrator): {detail}"
                )
            }
            Self::NullPointer(func) => {
                write!(f, "{func} returned null")
            }
            Self::InvalidJson(err) => {
                write!(f, "failed to parse snapshot JSON: {err}")
            }
            Self::DeviceError(err) => {
                write!(f, "LHM error: {err}")
            }
        }
    }
}

impl StdError for LhmError {}

pub type Result<T> = std::result::Result<T, LhmError>;

/// 函数指针地址
pub mod flags {
    pub const CPU: u32 = 1 << 0;
    pub const GPU: u32 = 1 << 1;
    pub const MEMORY: u32 = 1 << 2;
    pub const MOTHERBOARD: u32 = 1 << 3;
    pub const CONTROLLER: u32 = 1 << 4;
    pub const NETWORK: u32 = 1 << 5;
    pub const STORAGE: u32 = 1 << 6;
    pub const BATTERY: u32 = 1 << 7;
    pub const PSU: u32 = 1 << 8;
    pub const POWER_MONITOR: u32 = 1 << 9;

    pub const ALL: u32 = 0;
}

pub struct Lhm {
    _library: Library,

    close: CloseFn,
    snapshot: SnapshotFn,
    report: Option<ReportFn>,
    last_error: LastErrorFn,
    free: FreeFn,
}

impl Lhm {
    pub fn load(path: impl AsRef<OsStr>) -> Result<Self> {
        Self::load_with_flags(path, flags::ALL)
    }

    pub fn load_with_flags(path: impl AsRef<OsStr>, open_flags: u32) -> Result<Self> {
        let library = unsafe { Library::new(path.as_ref()) }
            .map_err(|err| LhmError::LibraryLoad(err.to_string()))?;
        let (open, close, snapshot, report, last_error, free) = unsafe {
            (
                *get_symbol::<OpenFn>(&library, b"lhm_open")?,
                *get_symbol::<CloseFn>(&library, b"lhm_close")?,
                *get_symbol::<SnapshotFn>(&library, b"lhm_snapshot_json")?,
                library
                    .get::<ReportFn>(b"lhm_report")
                    .ok()
                    .map(|symbol| *symbol),
                *get_symbol::<LastErrorFn>(&library, b"lhm_last_error")?,
                *get_symbol::<FreeFn>(&library, b"lhm_free")?,
            )
        };
        let status = unsafe { open(open_flags) };
        if status != 0 {
            let detail = unsafe { take_string(last_error, free) }.unwrap_or_default();
            return Err(LhmError::OpenFailed(detail));
        }
        Ok(Self {
            _library: library,
            close,
            snapshot,
            report,
            last_error,
            free,
        })
    }

    pub fn snapshot_raw(&self) -> Result<String> {
        unsafe {
            take_string(self.snapshot, self.free)
                .ok_or(LhmError::NullPointer("lhm_snapshot_json()"))
        }
    }

    pub fn snapshot(&self) -> Result<Summary> {
        let json = self.snapshot_raw()?;
        let summary: Summary =
            serde_json::from_str(&json).map_err(|err| LhmError::InvalidJson(err.to_string()))?;
        if !summary.ok {
            return Err(LhmError::DeviceError(
                summary.error.clone().unwrap_or_default(),
            ));
        }
        Ok(summary)
    }

    pub fn report(&self) -> Option<String> {
        let get = self.report?;

        unsafe { take_string(get, self.free) }
    }

    pub fn last_error(&self) -> Option<String> {
        unsafe { take_string(self.last_error, self.free) }
    }
}

impl Drop for Lhm {
    fn drop(&mut self) {
        unsafe {
            (self.close)();
        }
    }
}

unsafe fn get_symbol<'a, T>(library: &'a Library, name: &'static [u8]) -> Result<Symbol<'a, T>> {
    unsafe { library.get(name) }.map_err(|_| {
        let name = CStr::from_bytes_with_nul(name)
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|_| String::from_utf8_lossy(name).into_owned());

        LhmError::MissingSymbol(name)
    })
}

unsafe fn take_string(get: unsafe extern "C" fn() -> *mut c_char, free: FreeFn) -> Option<String> {
    unsafe {
        let ptr = get();

        if ptr.is_null() {
            return None;
        }

        let value = CStr::from_ptr(ptr).to_string_lossy().into_owned();

        free(ptr.cast::<c_void>());

        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 需要本机装有 PawnIO 驱动并具备真实传感器，CI 上默认跳过。
    /// 本地验证：`cargo test test_lhm -- --ignored --nocapture`
    #[test]
    #[ignore = "needs Windows + PawnIO driver + real hardware sensors"]
    fn test_lhm() {
        let lhm = Lhm::load(crate::internal::lhm::dll_path())
            .unwrap_or_else(|err| panic!("加载 dll 失败: {err}"));
        let summary = lhm
            .snapshot()
            .unwrap_or_else(|err| panic!("读取快照失败: {err}"));
        assert!(summary.ok);
        println!("{summary:#?}");
    }
}
