//! Screen color picker: a nearly transparent topmost overlay covers the virtual screen and
//! swallows input. A left click reads the pixel under the cursor from the screen DC after
//! the overlay is gone; Esc or a right click cancels.

use std::sync::Mutex;

use windows::{
    core::w,
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM},
        Graphics::Gdi::{GetDC, GetPixel, ReleaseDC},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Input::KeyboardAndMouse::{SetFocus, VK_ESCAPE},
            WindowsAndMessaging::{
                CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetCursorPos,
                GetMessageW, GetSystemMetrics, LoadCursorW, PostQuitMessage, RegisterClassW,
                SetCursor, SetForegroundWindow, SetLayeredWindowAttributes, ShowWindow,
                TranslateMessage, UnregisterClassW, IDC_CROSS, LWA_ALPHA, MSG, SM_CXVIRTUALSCREEN,
                SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SW_SHOW, WM_DESTROY,
                WM_KEYDOWN, WM_LBUTTONDOWN, WM_RBUTTONDOWN, WM_SETCURSOR, WNDCLASSW, WS_EX_LAYERED,
                WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
            },
        },
    },
};

/// Screen position of the accepted click; written by the window procedure.
static CLICK: Mutex<Option<POINT>> = Mutex::new(None);

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_SETCURSOR => {
            if let Ok(cursor) = LoadCursorW(None, IDC_CROSS) {
                SetCursor(Some(cursor));
            }
            LRESULT(1)
        }
        WM_LBUTTONDOWN => {
            let mut point = POINT::default();
            if GetCursorPos(&mut point).is_ok() {
                if let Ok(mut click) = CLICK.lock() {
                    *click = Some(point);
                }
            }
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_RBUTTONDOWN => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_KEYDOWN if wparam.0 == usize::from(VK_ESCAPE.0) => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}

/// Blocks until the user picks a color or cancels. Must run on a dedicated thread because
/// it owns a window and a message loop.
pub fn pick() -> Result<String, String> {
    *CLICK.lock().map_err(|_| "Picker state poisoned")? = None;

    // SAFETY: plain Win32 calls; every handle is created and released inside this function
    // on the calling thread, and the window procedure only touches `CLICK`.
    unsafe {
        let instance = GetModuleHandleW(None).map_err(|e| e.to_string())?;
        let class_name = w!("TrayforgeColorPickerOverlay");
        let class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance.into(),
            lpszClassName: class_name,
            ..Default::default()
        };
        if RegisterClassW(&class) == 0 {
            return Err("Cannot register overlay window class".into());
        }

        let overlay = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            class_name,
            w!("Trayforge color picker"),
            WS_POPUP,
            GetSystemMetrics(SM_XVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_CXVIRTUALSCREEN),
            GetSystemMetrics(SM_CYVIRTUALSCREEN),
            None,
            None,
            Some(instance.into()),
            None,
        );
        let overlay = match overlay {
            Ok(hwnd) => hwnd,
            Err(e) => {
                let _ = UnregisterClassW(class_name, Some(instance.into()));
                return Err(e.to_string());
            }
        };
        // Alpha 1/255 keeps the overlay invisible but hit-testable (alpha 0 lets clicks through).
        let _ = SetLayeredWindowAttributes(overlay, Default::default(), 1, LWA_ALPHA);
        let _ = ShowWindow(overlay, SW_SHOW);
        let _ = SetForegroundWindow(overlay);
        let _ = SetFocus(Some(overlay));

        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        let _ = UnregisterClassW(class_name, Some(instance.into()));

        let click = CLICK.lock().map_err(|_| "Picker state poisoned")?.take();
        let point = click.ok_or("cancelled")?;

        // Give DWM a moment to drop the destroyed overlay from the composed desktop.
        std::thread::sleep(std::time::Duration::from_millis(40));
        let screen = GetDC(None);
        let pixel = GetPixel(screen, point.x, point.y);
        ReleaseDC(None, screen);
        if pixel.0 == 0xFFFF_FFFF {
            return Err("Cannot read pixel".into());
        }
        // COLORREF is 0x00BBGGRR.
        let (r, g, b) = (
            pixel.0 & 0xFF,
            (pixel.0 >> 8) & 0xFF,
            (pixel.0 >> 16) & 0xFF,
        );
        Ok(format!("#{r:02X}{g:02X}{b:02X}"))
    }
}
