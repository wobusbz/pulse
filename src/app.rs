use std::sync::Arc;

use gpui_kit::component::Theme;
use gpui_kit::{actions, *};

use crate::internal::pulse_monit::PulseMonit;
use crate::ui::TaskbarWidget;

actions!(system_monitor, [Quit]);

const WIDGET_WIDTH: f32 = 205.0;
const WIDGET_HEIGHT: f32 = 40.0;
const SCREEN_MARGIN: f32 = 8.0;

pub fn run() {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::AllAssets)
        .run(move |cx| {
            gpui_kit::init(cx);
            cx.bind_keys([
                #[cfg(target_os = "macos")]
                KeyBinding::new("cmd-q", Quit, None),
                #[cfg(not(target_os = "macos"))]
                KeyBinding::new("alt-f4", Quit, None),
            ]);
            cx.on_action(|_: &Quit, cx: &mut App| cx.quit());

            let monitor = Arc::new(PulseMonit::new());
            let bounds = widget_bounds(cx);
            cx.open_window(widget_window_options(bounds), move |window, cx| {
                Theme::sync_system_appearance(Some(window), cx);
                cx.new(move |cx| TaskbarWidget::new(Arc::clone(&monitor), window, cx))
            })
            .expect("failed to open taskbar widget");
        });
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
