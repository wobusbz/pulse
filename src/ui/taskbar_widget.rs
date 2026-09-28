use std::sync::Arc;
use std::time::Duration;

use crate::internal::pulse_monit::PulseMonit;
use crate::ui::ExpandPanel;
use gpui_kit::assets::IconName;
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::{ActiveTheme, Icon, Sizable, Theme, h_flex, v_flex};
use gpui_kit::*;

pub(crate) struct TaskbarWidget {
    monitor: Arc<PulseMonit>,
    /// 上次触发重绘的传感器数据版本号；数据没变化时跳过每秒重绘。
    last_revision: Option<u64>,
    panel: Option<WindowHandle<ExpandPanel>>,
    _appearance_subscription: Subscription,
    #[cfg(target_os = "windows")]
    taskbar_hwnd: Option<isize>,
}

impl TaskbarWidget {
    pub(crate) fn new(
        monitor: Arc<PulseMonit>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let appearance_subscription = window.observe_window_appearance(|window, cx| {
            Theme::sync_system_appearance(Some(window), cx);
        });

        #[cfg(target_os = "windows")]
        let taskbar_hwnd = crate::internal::taskbar::attach(window);

        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                if this
                    .update(cx, |this, cx| {
                        #[cfg(target_os = "windows")]
                        if let Some(hwnd) = this.taskbar_hwnd {
                            crate::internal::taskbar::maintain(hwnd);
                        }
                        // 只在传感器数据变化、或还在初始化（转圈动画）时重绘，
                        // 避免每秒一次的无效渲染。
                        let revision = this.monitor.revision();
                        if !this.monitor.snapshot().ready || this.last_revision != Some(revision) {
                            this.last_revision = Some(revision);
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();

        Self {
            monitor,
            last_revision: None,
            panel: None,
            _appearance_subscription: appearance_subscription,
            #[cfg(target_os = "windows")]
            taskbar_hwnd,
        }
    }

    fn toggle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // 传感器还没初始化完之前，不展开详情面板。
        if !self.monitor.snapshot().ready {
            return;
        }

        if let Some(panel) = self.panel.take()
            && panel
                .update(cx, |_, window, _| window.remove_window())
                .is_ok()
        {
            // 面板关闭：安排收尾（窗口资源释放后整理并恢复工作集上限）。
            crate::internal::memtrim::panel_closing();
            cx.notify();
            return;
        }

        #[cfg(target_os = "windows")]
        let widget = self
            .taskbar_hwnd
            .and_then(|hwnd| crate::internal::taskbar::screen_bounds(hwnd, window.scale_factor()))
            .unwrap_or_else(|| window.bounds());
        #[cfg(not(target_os = "windows"))]
        let widget = window.bounds();
        let origin = point(
            widget.right() - panel_size().width,
            widget.origin.y - panel_size().height - px(PANEL_GAP),
        );
        let options = panel_window_options(Bounds::new(origin, panel_size()));

        let monitor = Arc::clone(&self.monitor);
        let panel = cx
            .open_window(options, move |window, cx| {
                cx.new(|cx| ExpandPanel::new(Arc::clone(&monitor), window, cx))
            })
            .expect("failed to open panel window");
        self.panel = Some(panel);
        crate::internal::memtrim::panel_opened();
        cx.notify();
    }
}

impl Render for TaskbarWidget {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let m = self.monitor.snapshot();

        let muted = cx.theme().muted_foreground;
        let danger = cx.theme().danger;
        let foreground = cx.theme().foreground;
        let cpu_usage = if m.cpu_usage >= 80.0 {
            danger
        } else {
            foreground
        };
        let cpu_temp = if m.cpu_temperature >= 80.0 {
            danger
        } else {
            foreground
        };
        let memory_usage = if m.memory_usage >= 90.0 {
            danger
        } else {
            foreground
        };
        let memory_temp = if m.memory_temperature >= 80.0 {
            danger
        } else {
            foreground
        };
        let disk_activity = if m.disk_activity >= 90.0 {
            danger
        } else {
            foreground
        };
        let disk_temp = if m.disk_temperature >= 60.0 {
            danger
        } else {
            foreground
        };
        let rest_bg = cx.theme().popover.opacity(0.55);
        let hover_bg = cx.theme().popover.opacity(0.95);

        // LHM 初始化要几秒；这期间只显示初始化状态，不显示指标，也不响应点击。
        if !m.ready {
            return h_flex()
                .items_center()
                .gap_2()
                .px_3()
                .rounded(cx.theme().radius_lg)
                .bg(rest_bg)
                .child(Spinner::new().small().color(muted))
                .child(div().text_sm().text_color(muted).child("正在初始化…"))
                .into_any_element();
        }

        h_flex()
            .id("taskbar-monitor")
            .items_center()
            .gap_2()
            .px_3()
            .rounded(cx.theme().radius_lg)
            .bg(rest_bg)
            .hover(move |style| style.bg(hover_bg))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, window, cx| this.toggle(window, cx)),
            )
            .child(
                v_flex()
                    .flex_1()
                    .gap_0()
                    .child(
                        h_flex()
                            .gap_2()
                            .child(metric(
                                IconName::Cpu,
                                percent(m.ready, m.cpu_usage),
                                cpu_usage,
                                muted,
                            ))
                            .child(metric(
                                IconName::MemoryStick,
                                percent(m.ready, m.memory_usage),
                                memory_usage,
                                muted,
                            ))
                            .child(metric(
                                IconName::HardDrive,
                                percent(m.ready, m.disk_activity),
                                disk_activity,
                                muted,
                            )),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(metric(
                                IconName::Thermometer,
                                fmt_temp(m.ready, m.cpu_temperature),
                                cpu_temp,
                                muted,
                            ))
                            .child(metric(
                                IconName::ThermometerSnowflake,
                                fmt_temp(m.ready, m.memory_temperature),
                                memory_temp,
                                muted,
                            ))
                            .child(metric(
                                IconName::Thermometer,
                                fmt_temp(m.ready, m.disk_temperature),
                                disk_temp,
                                muted,
                            )),
                    ),
            )
            .into_any_element()
    }
}

fn metric(icon: IconName, value: String, value_color: Hsla, icon_color: Hsla) -> impl IntoElement {
    h_flex()
        .flex_1()
        .gap_1()
        .items_center()
        .child(Icon::new(icon).small().text_color(icon_color))
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .text_color(value_color)
                .child(value),
        )
}

fn fmt_temp(ready: bool, celsius: f64) -> String {
    if !ready {
        return "…".to_string();
    }
    if celsius > 0.0 {
        format!("{celsius:.0}°")
    } else {
        "—".to_string()
    }
}

/// Percentage cell; `…` until the first sensor snapshot arrives.
fn percent(ready: bool, value: f64) -> String {
    if ready {
        format!("{value:.0}%")
    } else {
        "…".to_string()
    }
}

const PANEL_GAP: f32 = 8.0;

fn panel_size() -> Size<Pixels> {
    size(px(300.), px(545.))
}

fn panel_window_options(bounds: Bounds<Pixels>) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: None,
        kind: WindowKind::PopUp,
        focus: true,
        show: true,
        is_movable: true,
        is_resizable: false,
        is_minimizable: false,
        window_background: WindowBackgroundAppearance::Transparent,
        ..WindowOptions::default()
    }
}
