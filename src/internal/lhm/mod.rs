pub(crate) mod binding;
pub(crate) mod model;

pub(crate) use binding::{Lhm, flags};

use std::path::PathBuf;

/// Resolves `LhmNative.dll`.
///
/// Prefers the installed layout (`<exe dir>/lib/LhmNative.dll`, with a plain
/// `<exe dir>/LhmNative.dll` as a fallback) and otherwise falls back to the
/// repository's `lib/` folder, which is where `cargo run` and the tests load
/// it from.
pub(crate) fn dll_path() -> PathBuf {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.to_path_buf()));

    if let Some(dir) = exe_dir {
        for candidate in [
            dir.join("lib").join("LhmNative.dll"),
            dir.join("LhmNative.dll"),
        ] {
            if candidate.exists() {
                return candidate;
            }
        }
    }

    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("lib")
        .join("LhmNative.dll")
}
