use gpui_kit::Window;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::MapWindowPoints;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, FindWindowW, GWL_EXSTYLE, GWL_STYLE, GetClientRect, GetParent,
    GetWindowLongPtrW, GetWindowRect, HWND_TOP, IsWindow, SWP_FRAMECHANGED, SWP_NOACTIVATE,
    SWP_NOZORDER, SWP_SHOWWINDOW, SetParent, SetWindowLongPtrW, SetWindowPos, WS_CAPTION,
    WS_CHILD, WS_EX_CLIENTEDGE, WS_EX_DLGMODALFRAME, WS_EX_STATICEDGE, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_EX_WINDOWEDGE, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_POPUP, WS_SYSMENU,
    WS_THICKFRAME,
};
use windows::core::w;

const WIDTH: f32 = 205.0;
const HEIGHT: f32 = 40.0;
const TASKBAR_GAP: i32 = 4;
const TASKBAR_V_MARGIN: i32 = 3;

/// Window styles that create non-client chrome (caption / border / frame).
/// They must be removed so the client area fills the whole widget window,
/// otherwise GPUI only draws inside the inset client area and the widget is
/// left clipped and top-aligned.
const NON_CLIENT_STYLE: u32 = WS_POPUP.0
    | WS_CAPTION.0
    | WS_THICKFRAME.0
    | WS_SYSMENU.0
    | WS_MINIMIZEBOX.0
    | WS_MAXIMIZEBOX.0;

/// Extended window styles that add a non-client edge or resize frame.
const NON_CLIENT_EX_STYLE: u32 = WS_EX_TOPMOST.0
    | WS_EX_WINDOWEDGE.0
    | WS_EX_CLIENTEDGE.0
    | WS_EX_STATICEDGE.0
    | WS_EX_DLGMODALFRAME.0;

fn center_offset(container: i32, child: i32) -> i32 {
    (container - child).max(0) / 2
}

/// Shrinks the desired widget height so it always leaves a small margin inside
/// the taskbar, keeping the widget visibly centered vertically.
fn fitted_height(desired: i32, taskbar_height: i32) -> i32 {
    desired.min((taskbar_height - 2 * TASKBAR_V_MARGIN).max(1))
}

pub(crate) fn taskbar_height() -> Option<f32> {
    let taskbar = unsafe { FindWindowW(w!("Shell_TrayWnd"), None) };
    if taskbar.0 == 0 {
        return None;
    }

    let dpi = unsafe { windows::Win32::UI::HiDpi::GetDpiForWindow(taskbar) };
    (dpi > 0).then_some(HEIGHT)
}

pub(crate) fn embed(window: &mut Window) -> Option<isize> {
    let raw = match HasWindowHandle::window_handle(window) {
        Ok(handle) => handle.as_raw(),
        Err(error) => {
            eprintln!("Unable to obtain the GPUI native window handle: {error}");
            return None;
        }
    };
    let RawWindowHandle::Win32(handle) = raw else {
        eprintln!("GPUI returned a non-Win32 handle on Windows");
        return None;
    };
    let widget = HWND(handle.hwnd.get());
    match set_taskbar_parent(widget) {
        Ok(()) => Some(widget.0),
        Err(error) => {
            eprintln!("Unable to embed the monitor into the Windows taskbar: {error}");
            None
        }
    }
}

pub(crate) fn maintain(widget_handle: isize) {
    let widget = HWND(widget_handle);
    if !unsafe { IsWindow(widget).as_bool() } {
        return;
    }
    if let Err(error) = set_taskbar_parent(widget) {
        eprintln!("Unable to maintain the monitor taskbar child window: {error}");
    }
}

pub(crate) fn screen_bounds(
    widget_handle: isize,
    scale: f32,
) -> Option<gpui_kit::Bounds<gpui_kit::Pixels>> {
    let mut rect = RECT::default();
    unsafe { GetWindowRect(HWND(widget_handle), &mut rect) }.ok()?;
    let scale = scale.max(f32::EPSILON);
    Some(gpui_kit::Bounds::new(
        gpui_kit::point(
            gpui_kit::px(rect.left as f32 / scale),
            gpui_kit::px(rect.top as f32 / scale),
        ),
        gpui_kit::size(
            gpui_kit::px((rect.right - rect.left) as f32 / scale),
            gpui_kit::px((rect.bottom - rect.top) as f32 / scale),
        ),
    ))
}

fn set_taskbar_parent(widget: HWND) -> Result<(), String> {
    let taskbar = unsafe { FindWindowW(w!("Shell_TrayWnd"), None) };
    if taskbar.0 == 0 {
        return Err("Explorer taskbar HWND (Shell_TrayWnd) was not found".into());
    }

    let parent = unsafe { GetParent(widget) };
    if parent != taskbar {
        let style = unsafe { GetWindowLongPtrW(widget, GWL_STYLE) } as u32;
        let child_style = (style & !NON_CLIENT_STYLE) | WS_CHILD.0;
        unsafe {
            SetWindowLongPtrW(widget, GWL_STYLE, child_style as isize);
            SetParent(widget, taskbar);
            let extended_style = GetWindowLongPtrW(widget, GWL_EXSTYLE) as u32;
            SetWindowLongPtrW(
                widget,
                GWL_EXSTYLE,
                ((extended_style & !NON_CLIENT_EX_STYLE) | WS_EX_TOOLWINDOW.0) as isize,
            );
            SetWindowPos(
                widget,
                HWND_TOP,
                0,
                0,
                0,
                0,
                SWP_FRAMECHANGED | SWP_NOACTIVATE | SWP_NOZORDER,
            )
            .map_err(|error| format!("failed to apply child-window styles: {error}"))?;
        }
        if unsafe { GetParent(widget) } != taskbar {
            return Err("SetParent did not attach the widget to Shell_TrayWnd".into());
        }
        let applied_style = unsafe { GetWindowLongPtrW(widget, GWL_STYLE) } as u32;
        if applied_style & WS_CHILD.0 == 0 || applied_style & NON_CLIENT_STYLE != 0 {
            return Err("the GPUI HWND did not accept the required child-window styles".into());
        }
    }

    position_in_taskbar(widget, taskbar)
}

fn position_in_taskbar(widget: HWND, taskbar: HWND) -> Result<(), String> {
    let mut client = RECT::default();
    unsafe { GetClientRect(taskbar, &mut client) }
        .map_err(|error| format!("failed to read taskbar client bounds: {error}"))?;

    let scale = unsafe { windows::Win32::UI::HiDpi::GetDpiForWindow(widget) } as f32 / 96.0;
    let width = (WIDTH * scale).round() as i32;
    let client_height = client.bottom - client.top;
    let horizontal = client.right - client.left >= client_height;

    // Keep the widget slightly shorter than the taskbar so it stays visibly
    // centered vertically instead of filling the whole taskbar height.
    let desired_height = (HEIGHT * scale).round() as i32;
    let height = fitted_height(desired_height, client_height);
    let y = client.top + center_offset(client_height, height);

    let x = if horizontal {
        let notify = unsafe { FindWindowExW(taskbar, None, w!("TrayNotifyWnd"), None) };
        if notify.0 != 0 {
            let mut notify_rect = RECT::default();
            unsafe { GetWindowRect(notify, &mut notify_rect) }
                .map_err(|error| format!("failed to read taskbar notification area: {error}"))?;
            let mut point = [POINT {
                x: notify_rect.left - TASKBAR_GAP,
                y: 0,
            }];
            unsafe { MapWindowPoints(HWND::default(), taskbar, &mut point) };
            (point[0].x - width).max(client.left)
        } else {
            (client.right - width).max(client.left)
        }
    } else {
        client.left + center_offset(client.right - client.left, width)
    };

    unsafe {
        SetWindowPos(
            widget,
            HWND_TOP,
            x,
            y,
            width,
            height,
            SWP_NOACTIVATE | SWP_NOZORDER | SWP_SHOWWINDOW,
        )
    }
    .map_err(|error| format!("failed to position the monitor in the taskbar: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{center_offset, fitted_height};

    #[test]
    fn widget_is_centered_when_taskbar_has_extra_height() {
        assert_eq!(center_offset(48, 40), 4);
    }

    #[test]
    fn widget_starts_at_parent_edge_when_taskbar_is_too_short() {
        assert_eq!(center_offset(36, 40), 0);
    }

    #[test]
    fn widget_keeps_desired_height_when_taskbar_is_tall_enough() {
        assert_eq!(fitted_height(40, 48), 40);
    }

    #[test]
    fn widget_is_shrunk_to_leave_vertical_margins_on_a_short_taskbar() {
        assert_eq!(fitted_height(40, 40), 34);
    }

    #[test]
    fn widget_never_collapses_to_zero_height() {
        assert_eq!(fitted_height(40, 4), 1);
    }
}
