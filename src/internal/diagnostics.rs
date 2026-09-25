//! 崩溃可诊断性。
//!
//! release 构建同时带着两个「事故现场消失」的特性：
//!
//! 1. `panic = "abort"` —— panic 不展开，直接 `abort()`，进程无提示退出；
//! 2. `windows_subsystem = "windows"` —— 没有控制台，panic 写向 stderr 也没人看得到。
//!
//! 结果是用户只能报「它崩了」，开发者在 Windows 事件日志里也只拿到
//! `0xc0000409` / `FAST_FAIL_FATAL_APP_EXIT`，看不出是哪一行 panic。
//!
//! 这里装一个 panic hook，把 **panic 位置 + 消息**追加到一个日志文件里，
//! 让崩溃变得可以回报。消息与位置通常就足以定位问题（不需要符号表）。

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// 日志超过这个大小就在下次写入前清空，避免无限增长。
const MAX_LOG_BYTES: u64 = 512 * 1024;

/// 安装把 panic 记录到文件的 hook，同时保留原有的 stderr 输出。
pub(crate) fn install_panic_logger() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Some(path) = log_path() {
            let mut record = String::new();
            let _ = writeln!(
                record,
                "{}  Pulse {}  {}",
                utc_timestamp(),
                env!("CARGO_PKG_VERSION"),
                std::env::consts::OS
            );
            match info.location() {
                Some(location) => {
                    let _ = writeln!(
                        record,
                        "  at {}:{}:{}",
                        location.file(),
                        location.line(),
                        location.column()
                    );
                }
                None => {
                    let _ = writeln!(record, "  at <unknown location>");
                }
            }
            let _ = writeln!(record, "  {}", payload_message(info.payload()));
            let _ = writeln!(record);
            append(&path, &record);
        }
        // 保留默认行为：debug 构建下 stderr 仍能看到 panic。
        previous(info);
    }));
}

fn payload_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "<non-string panic payload>".to_string()
    }
}

fn append(path: &PathBuf, record: &str) {
    if std::fs::metadata(path).is_ok_and(|meta| meta.len() > MAX_LOG_BYTES) {
        let _ = std::fs::remove_file(path);
    }
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = file.write_all(record.as_bytes());
    }
}

/// `%LOCALAPPDATA%\Pulse\panic.log`，并为将来的移植预留各平台的等价位置。
fn log_path() -> Option<PathBuf> {
    let mut dir = log_dir()?;
    dir.push("Pulse");
    std::fs::create_dir_all(&dir).ok()?;
    dir.push("panic.log");
    Some(dir)
}

fn log_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Library/Application Support"))
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state"))
            })
    }
}

/// `2026-09-25 10:16:11 UTC`。手写转换，避免为一个日志时间戳引入依赖。
fn utc_timestamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);

    let days = seconds / 86_400;
    let time_of_day = seconds % 86_400;

    // Howard Hinnant 的 civil_from_days。
    let z = days as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { year + 1 } else { year };

    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02} UTC",
        time_of_day / 3_600,
        (time_of_day % 3_600) / 60,
        time_of_day % 60
    )
}

#[cfg(test)]
mod tests {
    use super::utc_timestamp;

    #[test]
    fn timestamp_has_the_expected_shape() {
        let stamp = utc_timestamp();
        assert!(stamp.ends_with(" UTC"), "{stamp}");
        assert_eq!(stamp.len(), "2026-09-25 10:16:11 UTC".len(), "{stamp}");
    }
}
