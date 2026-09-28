//! 工作集（Working Set）整理与上限。
//!
//! 常驻监控应用运行中会不断触达各种内存页（渲染缓存、传感器快照解析、
//! 窗口创建/销毁的峰值分配……）。其中很大一部分用完之后长期不再访问，
//! 却仍然占用着物理内存 —— 任务管理器里看到的「工作集 / 内存」数字
//! 因此只增不减。
//!
//! 本模块用两条手段把数字稳定在低位，而不是让它在「深整理回落后再
//! 爬升」的循环里来回波动：
//!
//! 1. **一次性整理**（[`trim_now`]）：调用 `EmptyWorkingSet` 把不再访问的
//!    页面立即交还系统。用于启动预热完成后、以及详情面板关闭后
//!    （等 DWM / DirectComposition 释放窗口资源）。
//! 2. **工作集硬上限**（[`set_cap`]）：给进程设置硬性工作集上限。达到上限
//!    后，系统会平滑地把超出的闲置页剔出驻留集，数字**贴着上限稳定**，
//!    不会长期缓慢爬升，也不会周期性地大幅回落。
//!
//! 打开详情面板时临时解除上限（面板需要更多渲染资源），关闭后回落并恢复。
//!
//! 以上操作不丢失数据、不改变提交（commit）计数，页面之后按需换回
//! （软缺页，代价极低）。非 Windows 平台全部为 no-op，可跨平台编译。

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// 空闲状态下的工作集上限。超出部分被系统平滑回收，
/// 实测表现为长时间稳定在 10MB 以内（9.9–10.0MB）。
/// 调大 → 缺页更少（16MB 时约降低至 1/8）；调小 → 更省但回收更频繁。
const IDLE_CAP_BYTES: usize = 10 * 1024 * 1024;
/// 详情面板打开期间的上限（足够宽松，等同解除，不再触发回收）。
const PANEL_CAP_BYTES: usize = 512 * 1024 * 1024;
/// `SetProcessWorkingSetSizeEx` 的 min 参数：硬上限要求 min 为有效值
/// 且 min ≤ max；这里给一个很小的值，且不启用硬下限。
const MIN_BYTES: usize = 1024 * 1024;

/// 两次 [`trim_now`] 之间的最小间隔，用于合并短时间内的重复请求。
#[cfg(target_os = "windows")]
const MIN_INTERVAL: Duration = Duration::from_secs(30);

/// 上次实际执行整理的毫秒时间戳（0 = 从未执行）。
#[cfg(target_os = "windows")]
static LAST_TRIM_MS: AtomicU64 = AtomicU64::new(0);

/// 0 表示当前没有排定的延迟收尾；非 0 是应执行的毫秒时间戳。
static SCHEDULED_DUE_MS: AtomicU64 = AtomicU64::new(0);

/// 详情面板是否处于打开状态（打开期间不整理、不收紧上限）。
static PANEL_OPEN: AtomicBool = AtomicBool::new(false);

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

/// 给当前进程设置工作集硬上限（best-effort，失败静默）。
#[cfg(target_os = "windows")]
fn set_cap(max_bytes: usize) {
    use windows::Win32::System::Memory::{
        QUOTA_LIMITS_HARDWS_MAX_ENABLE, SetProcessWorkingSetSizeEx,
    };
    use windows::Win32::System::Threading::GetCurrentProcess;
    unsafe {
        let _ = SetProcessWorkingSetSizeEx(
            GetCurrentProcess(),
            MIN_BYTES,
            max_bytes,
            QUOTA_LIMITS_HARDWS_MAX_ENABLE,
        );
    }
}

#[cfg(not(target_os = "windows"))]
fn set_cap(_max_bytes: usize) {}

/// 立刻整理工作集（受 [`MIN_INTERVAL`] 合并窗口约束）。返回是否真正执行。
#[cfg(target_os = "windows")]
pub(crate) fn trim_now() -> bool {
    let now = now_ms();
    let last = LAST_TRIM_MS.load(Ordering::Relaxed);
    if now.saturating_sub(last) < MIN_INTERVAL.as_millis() as u64 {
        return false;
    }
    LAST_TRIM_MS.store(now, Ordering::Relaxed);
    unsafe {
        let process = windows::Win32::System::Threading::GetCurrentProcess();
        let _ = windows::Win32::System::ProcessStatus::EmptyWorkingSet(process);
    }
    true
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn trim_now() -> bool {
    false
}

/// 回到空闲状态：整理一次并收紧工作集上限。
pub(crate) fn settle_idle() {
    trim_now();
    set_cap(IDLE_CAP_BYTES);
}

/// 详情面板打开：解除上限（面板期间不做回收）。
pub(crate) fn panel_opened() {
    PANEL_OPEN.store(true, Ordering::Relaxed);
    set_cap(PANEL_CAP_BYTES);
}

/// 详情面板开始关闭：稍后（等窗口资源释放）整理并恢复上限。
pub(crate) fn panel_closing() {
    PANEL_OPEN.store(false, Ordering::Relaxed);
    schedule_in(Duration::from_secs(5));
}

/// 详情面板当前是否打开。
pub(crate) fn panel_is_open() -> bool {
    PANEL_OPEN.load(Ordering::Relaxed)
}

/// 安排一次延迟收尾（与已排定的请求合并，取更早的时间）。
pub(crate) fn schedule_in(delay: Duration) {
    let due = now_ms() + delay.as_millis() as u64;
    let previous = SCHEDULED_DUE_MS.load(Ordering::Relaxed);
    if previous == 0 || due < previous {
        SCHEDULED_DUE_MS.store(due, Ordering::Relaxed);
    }
}

/// 由守护循环每秒调用：到点则执行排定的收尾（面板已重新打开则跳过）。
pub(crate) fn poll() {
    let due = SCHEDULED_DUE_MS.load(Ordering::Relaxed);
    if due != 0 && now_ms() >= due {
        SCHEDULED_DUE_MS.store(0, Ordering::Relaxed);
        if !PANEL_OPEN.load(Ordering::Relaxed) {
            settle_idle();
        }
    }
}
