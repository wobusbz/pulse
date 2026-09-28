#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

/// 可选堆分配剖析（`cargo build --release --features profiling` 后，
/// 以环境变量 `PULSE_PROFILE=1` 运行生效；退出时写出 `pulse-dhat-heap.json`）。
#[cfg(feature = "profiling")]
#[global_allocator]
static HEAP_PROFILER: dhat::Alloc = dhat::Alloc;

fn main() {
    #[cfg(feature = "profiling")]
    let _heap_profiler = std::env::var_os("PULSE_PROFILE").map(|_| {
        dhat::Profiler::builder()
            .file_name("pulse-dhat-heap.json")
            .build()
    });

    pulse::app::run();
}
