use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use crate::internal::pulse_monit::PulseMonit;
use crate::ui::metric_item::metric_item;
use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{ActiveTheme, Sizable, h_flex, v_flex};
use gpui_kit::*;

/// The flyout panel shown above the taskbar widget.
pub(crate) struct ExpandPanel {
    monitor: Arc<PulseMonit>,
    autostart: Rc<Cell<bool>>,
    _activation_subscription: Subscription,
}

impl ExpandPanel {
    pub(crate) fn new(
        monitor: Arc<PulseMonit>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        // Re-render the panel once per second to reflect fresh metrics.
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                if this.update(cx, |_, cx| cx.notify()).is_err() {
                    break;
                }
            }
        })
        .detach();

        // Dismiss the flyout when it loses activation, i.e. the user clicked
        // somewhere else. Defer the close briefly so a click on the widget can
        // close the panel itself first — otherwise the two race and the widget
        // click reopens the panel it just dismissed.
        let mut was_active = false;
        let activation_subscription = cx.observe_window_activation(window, move |_, window, cx| {
            let active = window.is_window_active();
            if was_active && !active {
                cx.spawn_in(window, async move |this, cx| {
                    cx.background_executor()
                        .timer(Duration::from_millis(100))
                        .await;
                    let _ = this.update_in(cx, |_, window, _| {
                        if !window.is_window_active() {
                            window.remove_window();
                        }
                    });
                })
                .detach();
            }
            was_active = active;
        });

        Self {
            monitor,
            autostart: Rc::new(Cell::new(crate::internal::autostart::is_enabled())),
            _activation_subscription: activation_subscription,
        }
    }
}

impl Render for ExpandPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let m = self.monitor.snapshot();
        let vram_usage = m.vram_usage();

        v_flex()
            .size_full()
            .p_4()
            .gap_4()
            .rounded(cx.theme().radius_lg)
            .bg(cx.theme().popover)
            .border_1()
            .border_color(cx.theme().border)
            .shadow_lg()
            .child(
                div()
                    .text_base()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(cx.theme().foreground)
                    .child("系统监控"),
            )
            .child(
                v_flex()
                    .gap_2()
                    .child(metric_item(
                        "cpu-usage",
                        IconName::Cpu,
                        "CPU 使用率",
                        fmt_percent(m.cpu_usage),
                        Some(m.cpu_usage),
                        m.cpu_usage >= 80.0,
                        cx,
                    ))
                    .child(metric_item(
                        "cpu-temp",
                        IconName::Thermometer,
                        "CPU 温度",
                        fmt_temp(m.cpu_temperature),
                        Some(m.cpu_temperature),
                        m.cpu_temperature >= 80.0,
                        cx,
                    ))
                    .child(metric_item(
                        "gpu-usage",
                        IconName::Gpu,
                        "GPU 使用率",
                        fmt_percent(m.gpu_usage),
                        Some(m.gpu_usage),
                        m.gpu_usage >= 90.0,
                        cx,
                    ))
                    .child(metric_item(
                        "gpu-temp",
                        IconName::Thermometer,
                        "GPU 温度",
                        fmt_temp(m.gpu_temperature),
                        Some(m.gpu_temperature),
                        m.gpu_temperature >= 80.0,
                        cx,
                    ))
                    .child(metric_item(
                        "vram",
                        IconName::Layers,
                        "显存",
                        fmt_vram(m.vram_used, m.vram_total),
                        vram_usage,
                        vram_usage.is_some_and(|usage| usage >= 90.0),
                        cx,
                    ))
                    .child(metric_item(
                        "memory-usage",
                        IconName::MemoryStick,
                        "内存使用率",
                        fmt_percent(m.memory_usage),
                        Some(m.memory_usage),
                        m.memory_usage >= 90.0,
                        cx,
                    ))
                    .child(metric_item(
                        "memory-temp",
                        IconName::ThermometerSnowflake,
                        "内存温度",
                        fmt_temp(m.memory_temperature),
                        Some(m.memory_temperature),
                        m.memory_temperature >= 80.0,
                        cx,
                    ))
                    .child(metric_item(
                        "disk-temp",
                        IconName::HardDrive,
                        "硬盘温度",
                        fmt_temp(m.disk_temperature),
                        Some(m.disk_temperature),
                        m.disk_temperature >= 60.0,
                        cx,
                    ))
                    .child(metric_item(
                        "disk-read",
                        IconName::HardDriveDownload,
                        "硬盘读取",
                        fmt_rate(m.disk_read_bytes),
                        None,
                        false,
                        cx,
                    ))
                    .child(metric_item(
                        "disk-write",
                        IconName::HardDriveUpload,
                        "硬盘写入",
                        fmt_rate(m.disk_write_bytes),
                        None,
                        false,
                        cx,
                    )),
            )
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .pt_3()
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child("开机自启动"),
                    )
                    .child({
                        let autostart = Rc::clone(&self.autostart);
                        let panel = cx.weak_entity();
                        Switch::new("autostart")
                            .checked(self.autostart.get())
                            .on_click(move |&checked, _window, cx| {
                                autostart.set(checked);
                                crate::internal::autostart::set_enabled(checked);
                                let _ = panel.update(cx, |_, cx| cx.notify());
                            })
                    }),
            )
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .pt_3()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("刷新间隔 1s"),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child({
                                let monitor = Arc::clone(&self.monitor);
                                Button::new("refresh-now")
                                    .label("立即刷新")
                                    .small()
                                    .on_click(move |_, _, _| monitor.refresh_now())
                            })
                            .child(
                                Button::new("quit")
                                    .label("退出")
                                    .small()
                                    .ghost()
                                    .on_click(|_, _, cx| cx.quit()),
                            ),
                    ),
            )
    }
}

fn fmt_percent(value: f64) -> String {
    format!("{value:.0}%")
}

fn fmt_temp(celsius: f64) -> String {
    if celsius > 0.0 {
        format!("{celsius:.1}°")
    } else {
        "—".to_string()
    }
}

fn fmt_vram(used: u64, total: u64) -> String {
    if total == 0 {
        return "—".to_string();
    }
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    format!("{:.1} / {:.0} GB", used as f64 / GIB, total as f64 / GIB)
}

fn fmt_rate(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    let bytes = bytes as f64;
    if bytes >= GIB {
        format!("{:.1} GB/s", bytes / GIB)
    } else if bytes >= MIB {
        format!("{:.1} MB/s", bytes / MIB)
    } else if bytes >= KIB {
        format!("{:.0} KB/s", bytes / KIB)
    } else {
        format!("{bytes:.0} B/s")
    }
}
