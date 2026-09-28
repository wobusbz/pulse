use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui_kit::component::Theme;
use gpui_kit::{actions, *};

use crate::internal::diagnostics::diagnostics::install_panic_logger;
use crate::internal::pulse_monit::PulseMonit;
use crate::ui::TaskbarWidget;

actions!(system_monitor, [Quit]);

const WIDGET_WIDTH: f32 = 205.0;
const WIDGET_HEIGHT: f32 = 40.0;
const SCREEN_MARGIN: f32 = 8.0;

/// 检查部件是否还活着、需要时把它重建出来的间隔。
const WIDGET_RECOVERY_INTERVAL: Duration = Duration::from_secs(1);

/// 传感器就绪后，再等这么久做第一次工作集整理并收紧上限
/// （首帧渲染完成即可，尽早让启动期堆积的闲置页归位）。
const WARMUP_TRIM_DELAY: Duration = Duration::from_secs(45);

pub fn run() {
    // 先装 panic 日志：release 是 panic = "abort" 且无控制台，
    // 没有它的话用户只能报「它崩了」。
    install_panic_logger();

    gpui_kit::application()
        .with_assets(gpui_kit::assets::AllAssets)
        // 任务栏部件是 `Shell_TrayWnd` 的**子窗口**，而任务栏会被销毁重建
        // （Explorer 重启、切换「自动隐藏任务栏」、改 DPI / 分辨率、插拔显示器、
        // Windows 更新…）。父窗口一销毁，子窗口会被连带销毁。
        //
        // gpui 的默认策略 `LastWindowClosed` 会让**整个进程**在最后一个窗口关闭时
        // 退出 —— 用户看到的就是「跑了一段时间，进程静默消失」（exit code 0，
        // 没有崩溃事件、也没有内存耗尽事件，所以之前无从定位）。
        //
        // 改成 `Explicit`：只有显式的 `App::quit`（退出按钮 / Alt+F4）才结束进程，
        // 好让下面的守护任务把部件重建出来。
        .with_quit_mode(QuitMode::Explicit)
        .run(move |cx| {
            gpui_kit::init(cx);
            cx.bind_keys([
                #[cfg(target_os = "macos")]
                KeyBinding::new("cmd-q", Quit, None),
                #[cfg(not(target_os = "macos"))]
                KeyBinding::new("alt-f4", Quit, None),
            ]);
            cx.on_action(|_: &Quit, cx: &mut App| cx.quit());

            // 调试辅助：`PULSE_QUIT_AFTER_SECS=N` 时 N 秒后自动退出
            // （供剖析采集完成后让数据落盘，例如 dhat 堆剖析）。
            if let Some(seconds) = std::env::var("PULSE_QUIT_AFTER_SECS")
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
            {
                cx.spawn(async move |cx| {
                    cx.background_executor()
                        .timer(Duration::from_secs(seconds))
                        .await;
                    cx.update(|cx| cx.quit());
                })
                .detach();
            }

            spawn_widget_supervisor(Arc::new(PulseMonit::new()), cx);
        });
}

/// 部件窗口的守护任务。
///
/// 关键在于它是**应用级任务**（`App::spawn`），不挂在任何窗口上：所以部件窗口被
/// 任务栏连带销毁之后，这个循环依然存活，每秒检查一次并把部件重新建出来。
///
/// 它依赖 `QuitMode::Explicit` —— 否则窗口销毁的瞬间 gpui 就已经退出进程，
/// 这里根本没有机会运行。
///
/// 传感器线程由 `Arc<PulseMonit>` 持有，重建部件时复用同一个实例，
/// 所以恢复是即时的，不会重新等 LHM 初始化。
fn spawn_widget_supervisor(monitor: Arc<PulseMonit>, cx: &mut App) {
    cx.spawn(async move |cx| {
        let mut widget: Option<WindowHandle<TaskbarWidget>> = None;
        let mut ready_since: Option<Instant> = None;
        let mut idle_settled = false;
        loop {
            // `update` 在实体或窗口已销毁时返回 Err，正好用来判断部件还在不在。
            let alive = widget
                .as_ref()
                .is_some_and(|handle| handle.update(cx, |_, _, _| ()).is_ok());
            if !alive {
                widget = open_widget_window(Arc::clone(&monitor), cx);
            }

            // 工作集：传感器就绪且预热一小段后，整理一次并收紧上限，
            // 使数字稳定在低位（面板打开期间顺延）；面板关闭的收尾
            // 通过 `memtrim::panel_closing()` 排定、这里每秒驱动执行。
            if !idle_settled {
                if ready_since.is_none() && monitor.snapshot().ready {
                    ready_since = Some(Instant::now());
                }
                if let Some(since) = ready_since
                    && since.elapsed() >= WARMUP_TRIM_DELAY
                    && !crate::internal::memtrim::panel_is_open()
                {
                    crate::internal::memtrim::settle_idle();
                    idle_settled = true;
                }
            }
            crate::internal::memtrim::poll();

            cx.background_executor()
                .timer(WIDGET_RECOVERY_INTERVAL)
                .await;
        }
    })
    .detach();
}

fn open_widget_window(
    monitor: Arc<PulseMonit>,
    cx: &mut AsyncApp,
) -> Option<WindowHandle<TaskbarWidget>> {
    cx.update(|cx| {
        let bounds = widget_bounds(cx);
        cx.open_window(widget_window_options(bounds), move |window, cx| {
            Theme::sync_system_appearance(Some(window), cx);
            cx.new(move |cx| TaskbarWidget::new(monitor, window, cx))
        })
    })
    .ok()
}

fn widget_bounds(cx: &App) -> Bounds<Pixels> {
    let display_bounds = cx
        .primary_display()
        .map(|display| display.visible_bounds())
        .unwrap_or_else(|| Bounds::new(point(px(0.), px(0.)), size(px(1920.), px(1080.))));
    #[cfg(target_os = "windows")]
    let height = crate::internal::taskbar::taskbar_height().unwrap_or(WIDGET_HEIGHT);
    #[cfg(not(target_os = "windows"))]
    let height = WIDGET_HEIGHT;
    let origin = point(
        display_bounds.right() - px(WIDGET_WIDTH) - px(SCREEN_MARGIN),
        display_bounds.bottom() - px(height),
    );
    Bounds::new(origin, size(px(WIDGET_WIDTH), px(height)))
}

fn widget_window_options(bounds: Bounds<Pixels>) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: None,
        kind: WindowKind::PopUp,
        focus: true,
        show: true,
        is_movable: false,
        is_resizable: false,
        is_minimizable: false,
        window_background: WindowBackgroundAppearance::Transparent,
        ..WindowOptions::default()
    }
}
