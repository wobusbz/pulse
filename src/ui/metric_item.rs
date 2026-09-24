use gpui_kit::assets::IconName;
use gpui_kit::component::progress::Progress;
use gpui_kit::component::{ActiveTheme, Icon, Sizable, h_flex, v_flex};
use gpui_kit::*;

pub(crate) fn metric_item(
    id: &'static str,
    icon: IconName,
    label: &str,
    value: String,
    percent: Option<f64>,
    warn: bool,
    cx: &App,
) -> impl IntoElement {
    let accent = if warn {
        cx.theme().danger
    } else {
        cx.theme().primary
    };
    let value_color = if warn {
        cx.theme().danger
    } else {
        cx.theme().foreground
    };
    let icon_color = if warn {
        cx.theme().danger
    } else {
        cx.theme().muted_foreground
    };

    let mut item = v_flex().gap_1().child(
        h_flex()
            .justify_between()
            .items_center()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(Icon::new(icon).small().text_color(icon_color))
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(label.to_string()),
                    ),
            )
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(value_color)
                    .child(value),
            ),
    );

    if let Some(percent) = percent {
        item = item.child(
            Progress::new(id)
                .value(percent.clamp(0.0, 100.0) as f32)
                .xsmall()
                .color(accent),
        );
    }

    item
}
