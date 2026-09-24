use std::sync::Arc;
use std::time::Duration;

use crate::internal::pulse_monit::PulseMonit;
use crate::ui::ExpandPanel;
use gpui_kit::assets::IconName;
use gpui_kit::component::{ActiveTheme, Icon, Sizable, Theme, h_flex, v_flex};
use gpui_kit::*;

pub(crate) struct TaskbarWidget {
    monitor: Arc<PulseMonit>,
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
        let taskbar_hwnd = crate::internal::taskbar::embed(window).or_else(|| {
            cx.quit();
            None
        });

        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                if this
                    .update(cx, |this, cx| {
                        #[cfg(target_os = "windows")]
                        if let Some(hwnd) = this.taskbar_hwnd {
                            crate::internal::taskbar::maintain(hwnd);
                        }
                        cx.notify();
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
            panel: None,
            _appearance_subscription: appearance_subscription,
            #[cfg(target_os = "windows")]
            taskbar_hwnd,
        }
    }

    fn toggle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(panel) = self.panel.take() {
            if panel
                .update(cx, |_, window, _| window.remove_window())
                .is_ok()
            {
                cx.notify();
                return;
            }
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
                                format!("{:.0}%", m.cpu_usage),
                                cpu_usage,
                                muted,
                            ))
                            .child(metric(
                                IconName::MemoryStick,
                                format!("{:.0}%", m.memory_usage),
                                memory_usage,
                                muted,
                            ))
                            .child(metric(
                                IconName::HardDrive,
                                format!("{:.0}%", m.disk_activity),
                                disk_activity,
                                muted,
                            )),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(metric(
                                IconName::Thermometer,
                                fmt_temp(m.cpu_temperature),
                                cpu_temp,
                                muted,
                            ))
                            .child(metric(
                                IconName::ThermometerSnowflake,
                                fmt_temp(m.memory_temperature),
                                memory_temp,
                                muted,
                            ))
                            .child(metric(
                                IconName::Thermometer,
                                fmt_temp(m.disk_temperature),
                                disk_temp,
                                muted,
                            )),
                    ),
            )
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

fn fmt_temp(celsius: f64) -> String {
    if celsius > 0.0 {
        format!("{celsius:.0}°")
    } else {
        "—".to_string()
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
