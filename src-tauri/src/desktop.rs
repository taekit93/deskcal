//! 위젯을 바탕화면 층에 고정한다.
//! - 기본: 바탕화면 창(Progman)의 자식 창으로 붙인다. 다른 창보다 항상 뒤에 있고,
//!   Win+D로 바탕화면을 보면 바탕화면과 함께 보인다.
//!   (Windows 11은 Win+D 때 바탕화면을 최상위 창보다도 위로 올리므로, 최상위로 올리는 방식은 통하지 않는다.)
//! - 대체: 붙이기에 실패하면 HWND_BOTTOM 고정 + 바탕화면이 앞에 오면 잠시 최상위로 올리는 방식.

pub fn pin(window: &tauri::WebviewWindow) {
    #[cfg(windows)]
    match window.hwnd() {
        Ok(hwnd) => {
            if !imp::attach_to_desktop(hwnd.0 as isize) {
                imp::pin(hwnd.0 as isize)
            }
        }
        Err(e) => log::error!("HWND를 얻지 못했습니다: {e}"),
    }
    #[cfg(not(windows))]
    let _ = window;
}

#[cfg(windows)]
mod imp {
    use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};

    use windows::core::w;
    use windows::Win32::Foundation::{HMODULE, HWND, LPARAM, LRESULT, RECT, WPARAM};
    use windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
    use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
    use windows::Win32::UI::WindowsAndMessaging::{
        FindWindowW, GetClassNameW, GetWindowLongPtrW, GetWindowRect, SetParent,
        SetWindowLongPtrW, SetWindowPos, GWL_STYLE, HWND_TOP, SWP_FRAMECHANGED, SWP_SHOWWINDOW, WS_CHILD, WS_POPUP, EVENT_SYSTEM_FOREGROUND, HWND_BOTTOM, HWND_NOTOPMOST, HWND_TOPMOST,
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, WINDOWPOS, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS,
        WM_WINDOWPOSCHANGING,
    };

    static WIDGET: AtomicIsize = AtomicIsize::new(0);
    static RAISED: AtomicBool = AtomicBool::new(false);

    fn widget() -> HWND {
        HWND(WIDGET.load(Ordering::Relaxed) as *mut _)
    }

    /// 위젯을 바탕화면 창(Progman)의 자식 창으로 만든다. Win+D에도 바탕화면과 함께 보인다.
    pub fn attach_to_desktop(raw: isize) -> bool {
        WIDGET.store(raw, Ordering::Relaxed);
        let hwnd = widget();
        unsafe {
            let progman = match FindWindowW(w!("Progman"), None) {
                Ok(h) if !h.is_invalid() => h,
                _ => {
                    log::warn!("Progman을 찾지 못함");
                    return false;
                }
            };
            let mut r = RECT::default();
            let mut pr = RECT::default();
            let _ = GetWindowRect(hwnd, &mut r);
            let _ = GetWindowRect(progman, &mut pr);
            let style = GetWindowLongPtrW(hwnd, GWL_STYLE);
            SetWindowLongPtrW(hwnd, GWL_STYLE, (style & !(WS_POPUP.0 as isize)) | WS_CHILD.0 as isize);
            if let Err(e) = SetParent(hwnd, progman) {
                log::warn!("SetParent 실패: {e}");
                SetWindowLongPtrW(hwnd, GWL_STYLE, style);
                return false;
            }
            let _ = SetWindowPos(
                hwnd,
                HWND_TOP,
                r.left - pr.left,
                r.top - pr.top,
                r.right - r.left,
                r.bottom - r.top,
                SWP_NOACTIVATE | SWP_SHOWWINDOW | SWP_FRAMECHANGED,
            );
            log::info!("바탕화면 창에 붙임");
        }
        true
    }

    pub fn pin(raw: isize) {
        WIDGET.store(raw, Ordering::Relaxed);
        let hwnd = widget();
        unsafe {
            if !SetWindowSubclass(hwnd, Some(subclass_proc), 1, 0).as_bool() {
                log::warn!("SetWindowSubclass 실패");
            }
            let hook = SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                HMODULE::default(),
                Some(on_foreground),
                0,
                0,
                WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
            );
            if hook.is_invalid() {
                log::warn!("SetWinEventHook 실패");
            }
            send_to_bottom(hwnd);
        }
    }

    unsafe fn send_to_bottom(hwnd: HWND) {
        let _ = SetWindowPos(hwnd, HWND_BOTTOM, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
    }

    unsafe extern "system" fn subclass_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _id: usize,
        _data: usize,
    ) -> LRESULT {
        if msg == WM_WINDOWPOSCHANGING && !RAISED.load(Ordering::Relaxed) {
            let pos = &mut *(lparam.0 as *mut WINDOWPOS);
            pos.hwndInsertAfter = HWND_BOTTOM;
        }
        DefSubclassProc(hwnd, msg, wparam, lparam)
    }

    unsafe fn is_desktop(hwnd: HWND) -> bool {
        let mut buf = [0u16; 64];
        let n = GetClassNameW(hwnd, &mut buf);
        if n <= 0 {
            return false;
        }
        let class = String::from_utf16_lossy(&buf[..n as usize]);
        class == "Progman" || class == "WorkerW"
    }

    unsafe extern "system" fn on_foreground(
        _hook: HWINEVENTHOOK,
        _event: u32,
        foreground: HWND,
        _id_object: i32,
        _id_child: i32,
        _thread: u32,
        _time: u32,
    ) {
        let hwnd = widget();
        if is_desktop(foreground) {
            RAISED.store(true, Ordering::Relaxed);
            let _ = SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        } else if RAISED.swap(false, Ordering::Relaxed) {
            let _ = SetWindowPos(hwnd, HWND_NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            send_to_bottom(hwnd);
        }
    }
}
