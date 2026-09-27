#[cfg(target_os = "windows")]
pub(crate) mod windows_taskbar;

#[cfg(target_os = "windows")]
pub(crate) use windows_taskbar::{attach, maintain, screen_bounds, taskbar_height};
