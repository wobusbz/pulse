//! Launch-at-sign-in support, backed by the per-user `Run` registry key.
//!
//! This is the same mechanism the installer uses, so the app can also turn it
//! on or off from its own UI.

#[cfg(target_os = "windows")]
mod windows_impl {
    use std::os::windows::ffi::OsStrExt;

    use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
    use windows::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SAM_FLAGS, REG_SZ, RegCloseKey,
        RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    };
    use windows::core::{PCWSTR, w};

    const RUN_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
    const VALUE_NAME: PCWSTR = w!("Pulse");

    fn open(access: REG_SAM_FLAGS) -> Option<HKEY> {
        let mut key = HKEY::default();
        let status = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, RUN_KEY, 0, access, &mut key) };
        (status == ERROR_SUCCESS).then_some(key)
    }

    pub(crate) fn is_enabled() -> bool {
        let Some(key) = open(KEY_QUERY_VALUE) else {
            return false;
        };
        let mut length = 0u32;
        let status =
            unsafe { RegQueryValueExW(key, VALUE_NAME, None, None, None, Some(&mut length)) };
        let _ = unsafe { RegCloseKey(key) };
        status == ERROR_SUCCESS
    }

    pub(crate) fn set_enabled(enabled: bool) -> bool {
        let Some(key) = open(KEY_SET_VALUE) else {
            return false;
        };

        let ok = if enabled {
            match std::env::current_exe() {
                Ok(path) => {
                    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
                    wide.push(0);
                    let bytes = unsafe {
                        std::slice::from_raw_parts(wide.as_ptr().cast::<u8>(), wide.len() * 2)
                    };
                    let status = unsafe { RegSetValueExW(key, VALUE_NAME, 0, REG_SZ, Some(bytes)) };
                    status == ERROR_SUCCESS
                }
                Err(_) => false,
            }
        } else {
            let status = unsafe { RegDeleteValueW(key, VALUE_NAME) };
            status == ERROR_SUCCESS || status == ERROR_FILE_NOT_FOUND
        };

        let _ = unsafe { RegCloseKey(key) };
        ok
    }
}

#[cfg(target_os = "windows")]
pub(crate) use windows_impl::{is_enabled, set_enabled};

#[cfg(not(target_os = "windows"))]
pub(crate) fn is_enabled() -> bool {
    false
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn set_enabled(_enabled: bool) -> bool {
    false
}
