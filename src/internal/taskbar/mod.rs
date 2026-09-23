#[cfg(target_os = "windows")]
pub(crate) mod windows_taskbar;

#[cfg(target_os = "windows")]
pub(crate) use windows_taskbar::{embed, maintain, screen_bounds, taskbar_height};
