use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use crate::internal::lhm::model::Summary;
use crate::internal::lhm::{Lhm, dll_path, flags};

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Metrics {
    /// CPU 的使用率 (0..=100).
    pub cpu_usage: f64,
    /// CPU 的温度 °C
    pub cpu_temperature: f64,
    /// GPU 的使用率 (0..=100).
    pub gpu_usage: f64,
    /// GPU 的温度 °C
    pub gpu_temperature: f64,
    /// 当前正在使用的视频内存
    pub vram_used: u64,
    ///总视频内存（字节）（不可用时为0）。
    pub vram_total: u64,
    /// 内存的使用率 (0..=100).
    pub memory_usage: f64,
    /// 内存的温度，这个值是看硬件的是否支持，是根据品牌决定 °C
    pub memory_temperature: f64,
    /// 硬盘的温度 °C
    pub disk_temperature: f64,
    /// 硬盘当前的状态负载 (0..=100).
    pub disk_activity: f64,
    /// 硬盘的读取
    pub disk_read_bytes: u64,
    /// 硬盘的写入
    pub disk_write_bytes: u64,
    /// 是否已经拿到第一份传感器快照。LHM 初始化需要几秒，在此之前界面应显示加载中。
    pub ready: bool,
}

impl Metrics {
    pub(crate) fn vram_usage(&self) -> Option<f64> {
        (self.vram_total > 0 && self.vram_used > 0)
            .then(|| self.vram_used as f64 / self.vram_total as f64 * 100.0)
    }

    fn from_summary(summary: &Summary) -> Self {
        let gpu = summary.gpus.iter().max_by(|a, b| {
            a.usage_percent
                .unwrap_or(0.0)
                .total_cmp(&b.usage_percent.unwrap_or(0.0))
        });

        let disk_temperature = summary
            .disks
            .iter()
            .filter_map(|disk| disk.temperature_c)
            .fold(0.0_f32, f32::max);
        let disk_activity = summary
            .disks
            .iter()
            .filter_map(|disk| disk.activity_percent)
            .fold(0.0_f32, f32::max);
        let disk_read = summary
            .disks
            .iter()
            .map(|disk| disk.read_bytes_per_sec)
            .sum::<f64>();
        let disk_write = summary
            .disks
            .iter()
            .map(|disk| disk.write_bytes_per_sec)
            .sum::<f64>();

        let memory_temperature = summary
            .memory
            .temperature_c
            .or_else(|| summary.memory.dimm_temperatures_c.first().copied())
            .unwrap_or(0.0);

        Self {
            cpu_usage: f64::from(summary.cpu.usage_percent.unwrap_or(0.0)),
            cpu_temperature: f64::from(summary.cpu.temperature_c.unwrap_or(0.0)),
            gpu_usage: f64::from(gpu.and_then(|gpu| gpu.usage_percent).unwrap_or(0.0)),
            gpu_temperature: f64::from(gpu.and_then(|gpu| gpu.temperature_c).unwrap_or(0.0)),
            vram_used: 0,
            vram_total: gpu.map_or(0, |gpu| gpu.vram_total_bytes),
            memory_usage: f64::from(summary.memory.usage_percent.unwrap_or(0.0)),
            memory_temperature: f64::from(memory_temperature),
            disk_temperature: f64::from(disk_temperature),
            disk_activity: f64::from(disk_activity),
            disk_read_bytes: disk_read.max(0.0) as u64,
            disk_write_bytes: disk_write.max(0.0) as u64,
            ready: true,
        }
    }
}

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

    pub(crate) fn snapshot(&self) -> Metrics {
        *self.metrics.read().expect("monitor metrics poisoned")
    }

    pub(crate) fn refresh_now(&self) {
        self.force_refresh.store(true, Ordering::Relaxed);
    }
}

/// 多久刷新一次新的LHM快照
const COLLECT_INTERVAL: Duration = Duration::from_secs(2);
/// 等待循环刷新获取新的快照的间隔时间
const WAIT_SLICE: Duration = Duration::from_millis(100);
/// 要监控硬件的信息
const SENSOR_FLAGS: u32 =
    flags::CPU | flags::GPU | flags::MEMORY | flags::STORAGE | flags::MOTHERBOARD;

fn collect_loop(metrics: Arc<RwLock<Metrics>>, force_refresh: Arc<AtomicBool>) {
    let lhm = match Lhm::load_with_flags(dll_path(), SENSOR_FLAGS) {
        Ok(lhm) => lhm,
        Err(err) => {
            eprintln!("failed to initialise LibreHardwareMonitor: {err}");
            return;
        }
    };

    // 启动先刷新一次
    force_refresh.store(true, Ordering::Relaxed);

    loop {
        let mut waited = Duration::ZERO;
        while waited < COLLECT_INTERVAL {
            std::thread::sleep(WAIT_SLICE);
            waited += WAIT_SLICE;
            if force_refresh.swap(false, Ordering::Relaxed) {
                break;
            }
        }

        match lhm.snapshot() {
            Ok(summary) => {
                let snapshot = Metrics::from_summary(&summary);
                *metrics.write().expect("monitor metrics poisoned") = snapshot;
            }
            Err(err) => eprintln!("failed to read LHM snapshot: {err}"),
        }
    }
}
