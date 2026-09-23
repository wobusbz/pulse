use std::collections::HashMap;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use sysinfo::{
    CpuRefreshKind, DiskRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System,
};

/// A snapshot of the system metrics the widget displays.
///
/// The UI layer only ever reads this — all collection happens on
/// [`PulseMonit`]'s background thread.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Metrics {
    /// Global CPU usage percentage (0..=100).
    pub cpu_usage: f64,
    /// CPU temperature in °C (0 when unavailable).
    pub cpu_temperature: f64,
    /// GPU usage percentage (0..=100).
    pub gpu_usage: f64,
    /// GPU temperature in °C (0 when unavailable).
    pub gpu_temperature: f64,
    /// Video memory currently in use, in bytes.
    pub vram_used: u64,
    /// Total video memory in bytes (0 when unavailable).
    pub vram_total: u64,
    /// Memory usage percentage (0..=100).
    pub memory_usage: f64,
    /// Memory / thermal-zone temperature in °C (0 when unavailable).
    pub memory_temperature: f64,
    /// Disk activity percentage (0..=100), i.e. how busy the disks were.
    pub disk_activity: f64,
    /// Disk temperature in °C (0 when unavailable).
    pub disk_temperature: f64,
    /// Bytes read since the previous sample (per second at the 1s cadence).
    pub disk_read_bytes: u64,
    /// Bytes written since the previous sample (per second at the 1s cadence).
    pub disk_write_bytes: u64,
}

impl Metrics {
    /// Video memory usage as a percentage of the total, or `None` when the
    /// total is not known.
    pub(crate) fn vram_usage(&self) -> Option<f64> {
        (self.vram_total > 0)
            .then(|| self.vram_used as f64 / self.vram_total as f64 * 100.0)
    }
}

/// Collects system metrics on a background thread and shares the latest
/// snapshot with the UI. Data fetching is deliberately kept out of the UI.
pub(crate) struct PulseMonit {
    metrics: Arc<RwLock<Metrics>>,
    force_refresh: Arc<AtomicBool>,
}

impl PulseMonit {
    pub(crate) fn new() -> Self {
        let metrics = Arc::new(RwLock::new(Metrics::default()));
        let force_refresh = Arc::new(AtomicBool::new(false));

        let shared = Arc::clone(&metrics);
        let force = Arc::clone(&force_refresh);
        std::thread::Builder::new()
            .name("pulse-monit".to_string())
            .spawn(move || collect_loop(shared, force))
            .expect("failed to spawn monitor thread");

        Self {
            metrics,
            force_refresh,
        }
    }

    /// Returns the most recently collected metrics.
    pub(crate) fn snapshot(&self) -> Metrics {
        *self.metrics.read().expect("monitor metrics poisoned")
    }

    /// Asks the collector to refresh immediately instead of waiting for the
    /// next tick. Used by the panel's "立即刷新" button.
    pub(crate) fn refresh_now(&self) {
        self.force_refresh.store(true, Ordering::Relaxed);
    }
}

/// How often the cheap metrics (CPU, memory, disk I/O) are collected.
const COLLECT_INTERVAL: Duration = Duration::from_secs(1);
/// Temperatures are expensive to read (WMI / powershell) — refresh every N ticks.
const TEMP_TICKS: u64 = 5;
/// `nvidia-smi` spawns a process — refresh the GPU every N ticks.
const GPU_TICKS: u64 = 2;
/// Granularity of the wait loop so an explicit refresh wakes the thread promptly.
const WAIT_SLICE: Duration = Duration::from_millis(100);

fn collect_loop(metrics: Arc<RwLock<Metrics>>, force_refresh: Arc<AtomicBool>) {
    // Only ask for what the widget shows: without this, `System::new_all()
    // would allocate the whole process/user/component list we never read.
    let mut sys = System::new_with_specifics(
        RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
            .with_memory(MemoryRefreshKind::everything()),
    );
    // Prime the CPU counters so the first usage reading is meaningful.
    sys.refresh_cpu_usage();

    let total_memory = sys.total_memory() as f64;
    let mut disks = Disks::new_with_refreshed_list();
    // Prime the disk counters so the first delta is not the whole boot total.
    disks.refresh_specifics(true, DiskRefreshKind::nothing().with_io_usage());
    let disk_activity_sampler = DiskActivity::new();
    let mut last_sample = Instant::now();
    let mut tick: u64 = 0;

    loop {
        // Wait for the next interval, waking early on an explicit refresh.
        let mut waited = Duration::ZERO;
        while waited < COLLECT_INTERVAL {
            std::thread::sleep(WAIT_SLICE);
            waited += WAIT_SLICE;
            if force_refresh.swap(false, Ordering::Relaxed) {
                break;
            }
        }

        sys.refresh_memory();
        sys.refresh_cpu_usage();
        disks.refresh_specifics(true, DiskRefreshKind::nothing().with_io_usage());

        // The disk counters report bytes since the previous refresh, so divide
        // by the real elapsed time to keep the rate correct even when an
        // explicit refresh shortens the interval.
        let now = Instant::now();
        let elapsed = now.duration_since(last_sample).as_secs_f64();
        last_sample = now;

        let cpu_usage = sys.global_cpu_usage() as f64;
        let memory_usage = if total_memory > 0.0 {
            (sys.used_memory() as f64 / total_memory * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        };
        let (read_delta, write_delta) = total_disk_io(&disks);
        let disk_read_bytes = to_per_second(read_delta, elapsed);
        let disk_write_bytes = to_per_second(write_delta, elapsed);
        let disk_activity = disk_activity_sampler.sample();

        // Sample the expensive sources on their own cadence, and only then take
        // the write lock, so the UI never blocks behind a helper process.
        let gpu = (tick % GPU_TICKS == 0).then(gpu_metrics);
        let temps = (tick % TEMP_TICKS == 0).then(temperatures);

        {
            let mut m = metrics.write().expect("monitor metrics poisoned");
            m.cpu_usage = cpu_usage;
            m.memory_usage = memory_usage;
            m.disk_read_bytes = disk_read_bytes;
            m.disk_write_bytes = disk_write_bytes;
            m.disk_activity = disk_activity;

            if let Some(gpu) = gpu {
                m.gpu_usage = gpu.usage;
                m.gpu_temperature = gpu.temperature;
                m.vram_used = gpu.vram_used;
                m.vram_total = gpu.vram_total;
            }
            if let Some((cpu, memory, disk)) = temps {
                m.cpu_temperature = cpu;
                m.memory_temperature = memory;
                m.disk_temperature = disk;
            }
        }

        tick = tick.wrapping_add(1);
    }
}

/// Converts a byte delta measured over `elapsed` seconds into a per-second rate.
fn to_per_second(bytes: u64, elapsed: f64) -> u64 {
    if elapsed > 0.0 {
        (bytes as f64 / elapsed).round() as u64
    } else {
        bytes
    }
}

/// Samples overall disk activity from the Windows performance counter
/// `\PhysicalDisk(_Total)\% Idle Time`, without spawning a helper process.
#[cfg(target_os = "windows")]
struct DiskActivity {
    query: isize,
    counter: isize,
}

#[cfg(target_os = "windows")]
impl DiskActivity {
    fn new() -> Self {
        use windows::Win32::System::Performance::{
            PdhAddEnglishCounterW, PdhCloseQuery, PdhOpenQueryW,
        };
        use windows::core::{PCWSTR, w};

        unsafe {
            let mut query = 0isize;
            if PdhOpenQueryW(PCWSTR::null(), 0, &mut query) != 0 {
                return Self { query: 0, counter: 0 };
            }
            let mut counter = 0isize;
            if PdhAddEnglishCounterW(
                query,
                w!("\\PhysicalDisk(_Total)\\% Idle Time"),
                0,
                &mut counter,
            ) != 0
            {
                let _ = PdhCloseQuery(query);
                return Self { query: 0, counter: 0 };
            }
            Self { query, counter }
        }
    }

    fn sample(&self) -> f64 {
        use windows::Win32::System::Performance::{
            PDH_FMT_COUNTERVALUE, PDH_FMT_DOUBLE, PdhCollectQueryData,
            PdhGetFormattedCounterValue,
        };

        if self.query == 0 {
            return 0.0;
        }
        unsafe {
            if PdhCollectQueryData(self.query) != 0 {
                return 0.0;
            }
            let mut value = PDH_FMT_COUNTERVALUE::default();
            if PdhGetFormattedCounterValue(self.counter, PDH_FMT_DOUBLE, None, &mut value) != 0 {
                return 0.0;
            }
            let idle = value.Anonymous.doubleValue;
            (100.0 - idle).clamp(0.0, 100.0)
        }
    }
}

/// Disk activity is only sourced on Windows; elsewhere it reports zero.
#[cfg(not(target_os = "windows"))]
struct DiskActivity;

#[cfg(not(target_os = "windows"))]
impl DiskActivity {
    fn new() -> Self {
        Self
    }

    fn sample(&self) -> f64 {
        0.0
    }
}

/// Total bytes read/written since the previous refresh across the machine's
/// physical disks.
///
/// `IOCTL_DISK_PERFORMANCE` reports a physical disk's counters for every volume
/// on it, so two volumes on one disk return identical cumulative totals. Group
/// by those totals so each physical disk is counted once.
fn total_disk_io(disks: &Disks) -> (u64, u64) {
    let mut physical: HashMap<(u64, u64), (u64, u64)> = HashMap::new();
    for disk in disks.list() {
        let usage = disk.usage();
        physical
            .entry((usage.total_read_bytes, usage.total_written_bytes))
            .or_insert((usage.read_bytes, usage.written_bytes));
    }
    physical
        .into_values()
        .fold((0, 0), |(read, write), (r, w)| (read + r, write + w))
}

#[derive(Clone, Copy, Default)]
struct GpuMetrics {
    usage: f64,
    temperature: f64,
    vram_used: u64,
    vram_total: u64,
}

/// Builds a command whose child console window is suppressed on Windows, so the
/// helper processes (`nvidia-smi`, `powershell`) do not flash a terminal window
/// when the app runs as a GUI-subsystem binary.
fn hidden_command(program: &str) -> Command {
    let mut command = Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

/// Reads GPU utilisation, temperature and VRAM from `nvidia-smi`. Returns zeros
/// when it is unavailable (no NVIDIA driver), so the UI degrades gracefully.
fn gpu_metrics() -> GpuMetrics {
    let Ok(output) = hidden_command("nvidia-smi")
        .args([
            "--query-gpu=utilization.gpu,temperature.gpu,memory.used,memory.total",
            "--format=csv,noheader,nounits",
        ])
        .output()
    else {
        return GpuMetrics::default();
    };

    if !output.status.success() {
        return GpuMetrics::default();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let Some(line) = stdout.lines().next() else {
        return GpuMetrics::default();
    };
    let fields: Vec<&str> = line.split(',').map(str::trim).collect();
    if fields.len() < 4 {
        return GpuMetrics::default();
    }

    const MIB: u64 = 1024 * 1024;
    GpuMetrics {
        usage: fields[0].parse().unwrap_or(0.0),
        temperature: fields[1].parse().unwrap_or(0.0),
        vram_used: fields[2]
            .parse::<u64>()
            .unwrap_or(0)
            .saturating_mul(MIB),
        vram_total: fields[3]
            .parse::<u64>()
            .unwrap_or(0)
            .saturating_mul(MIB),
    }
}

/// CPU, thermal-zone, and disk temperatures in °C.
///
/// Windows exposes the CPU package and board through the same ACPI thermal
/// zone, so `cpu` and `memory` share a value. All three come from one hidden
/// PowerShell call to keep helper-process spawns (and their console windows)
/// minimal.
#[cfg(target_os = "windows")]
fn temperatures() -> (f64, f64, f64) {
    let script = r#"
$ErrorActionPreference = 'SilentlyContinue'
$tz = Get-CimInstance -Namespace root/wmi -ClassName MSAcpi_ThermalZoneTemperature |
    Select-Object -First 1 -ExpandProperty CurrentTemperature
if ($tz) { $thermal = [math]::Round($tz / 10 - 273.15, 1) } else { $thermal = 0 }
$disk = Get-PhysicalDisk | Get-StorageReliabilityCounter |
    Select-Object -First 1 -ExpandProperty Temperature
if ($null -eq $disk) { $disk = 0 }
Write-Output "thermal=$thermal"
Write-Output "disk=$disk"
"#;

    let Ok(output) = hidden_command("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
    else {
        return (0.0, 0.0, 0.0);
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut thermal = 0.0;
    let mut disk = 0.0;
    for line in stdout.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("thermal=") {
            thermal = value.trim().parse().unwrap_or(0.0);
        } else if let Some(value) = line.strip_prefix("disk=") {
            disk = value.trim().parse().unwrap_or(0.0);
        }
    }
    (thermal, thermal, disk)
}

#[cfg(not(target_os = "windows"))]
fn temperatures() -> (f64, f64, f64) {
    (0.0, 0.0, 0.0)
}
