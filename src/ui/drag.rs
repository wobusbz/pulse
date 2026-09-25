use gpui_kit::Window;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture;
use windows::Win32::UI::WindowsAndMessaging::{HTCAPTION, PostMessageW, WM_NCLBUTTONDOWN};

/// 让 Windows 用原生标题栏拖动来移动窗口。
///
/// # 为什么必须是 `PostMessageW`，不能是 `SendMessageW`
///
/// `WM_NCLBUTTONDOWN` + `HTCAPTION` 会让 `DefWindowProc` 进入一个**模态消息循环**
/// （它自己抽消息、直到用户松开鼠标才返回）。
///
/// `SendMessageW` 是**同步**调用：那个模态循环会在**当前 GPUI 事件回调还没返回、
/// `App` 仍处于可变借用状态**时就开始抽消息。抽到的消息里包含 GPUI 的定时器任务
/// （例如展开面板每秒刷新一次的 `Entity::update`），于是它去
/// `AppCell::borrow_mut()` —— 撞上外层未释放的借用，panic
/// `RefCell already borrowed`。
///
/// release 构建是 `panic = "abort"`，所以这个 panic 直接变成**整个进程静默崩溃**
/// （Windows 事件日志里是 `0xc0000409` / `FAST_FAIL_FATAL_APP_EXIT`），调用栈为：
///
/// ```text
/// ExpandPanel 的 1s 刷新 -> WeakEntity::update -> AppCell::borrow_mut
///   <- WindowsPlatformInner::handle_msg <- window_procedure
/// ```
///
/// `PostMessageW` 只把消息**放进队列**，等当前回调返回、借用释放之后才由消息循环
/// 处理，因此不会重入。
///
/// # 给后续维护者的警告
///
/// 同样的重入问题适用于**任何**在主线程启动原生模态循环的 API —— 例如
/// `TrackPopupMenu`（右键菜单）、`MessageBoxW`、`DragQueryFile` 等。
/// 如果将来要加右键菜单，必须从消息队列进入，或在 GPUI 回调之外调用。
pub(super) fn start_window_drag(window: &Window) {
    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return;
    };

    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return;
    };
    let hwnd = HWND(handle.hwnd.get());

    unsafe {
        let _ = ReleaseCapture();
        let _ = PostMessageW(
            hwnd,
            WM_NCLBUTTONDOWN,
            WPARAM(HTCAPTION as usize),
            LPARAM(0),
        );
    }
}
