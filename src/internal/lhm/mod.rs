pub(crate) mod lhm;
pub(crate) mod model;

pub(crate) use lhm::{Lhm, flags};

use std::path::PathBuf;

pub(crate) fn dll_path() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for candidate in [
                dir.join("lib").join("LhmNative.dll"),
                dir.join("LhmNative.dll"),
            ] {
                if candidate.exists() {
                    return candidate;
                }
            }
        }
    }

    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("lib")
        .join("LhmNative.dll")
}
