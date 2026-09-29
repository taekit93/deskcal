//! 위젯을 바탕화면 층에 고정한다.
//! - WM_WINDOWPOSCHANGING을 가로채 항상 HWND_BOTTOM에 둔다 (클릭해도 다른 창 위로 올라오지 않음).
//! - 바탕화면(Progman/WorkerW)이 전경이 되면(Win+D, 바탕화면 클릭) 잠시 최상위로 올려 보이게 한다.

pub fn pin(window: &tauri::WebviewWindow) {
    #[cfg(windows)]
    match window.hwnd() {
        Ok(hwnd) => imp::pin(hwnd.0 as isize),
        Err(e) => log::error!("HWND를 얻지 못했습니다: {e}"),
    }
    #[cfg(not(windows))]
    let _ = window;
}

#[cfg(windows)]
mod imp {
    use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};

    use windows::Win32::Foundation::{HMODULE, HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
    use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetClassNameW, SetWindowPos, EVENT_SYSTEM_FOREGROUND, HWND_BOTTOM, HWND_NOTOPMOST, HWND_TOPMOST,
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, WINDOWPOS, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS,
        WM_WINDOWPOSCHANGING,
    };

    static WIDGET: AtomicIsize = AtomicIsize::new(0);
    static RAISED: AtomicBool = AtomicBool::new(false);

    fn widget() -> HWND {
        HWND(WIDGET.load(Ordering::Relaxed) as *mut _)
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
