//! The `main` window exists only while it is visible: it is created on demand and always
//! destroyed (never hidden), so WebView2 and its child processes exit with it.

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use serde_json::{json, Value};
use tauri::{
    window::{Effect, EffectsBuilder},
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, WindowEvent,
};

use windows::Win32::UI::WindowsAndMessaging::{GetAncestor, GetForegroundWindow, GA_ROOTOWNER};

use crate::{
    settings::{self, Settings},
    taskbar::{self, Edge, Rect},
};

pub const LABEL: &str = "main";
const DEFAULT_SIZE: (f64, f64) = (820.0, 650.0);
const MIN_STANDALONE_SIZE: (f64, f64) = (480.0, 360.0);
const MIN_POPOVER_SIZE: (f64, f64) = (560.0, 420.0);
const POPOVER_MARGIN: f64 = 12.0;
/// How often an open popover checks whether an auto-hidden taskbar slid over it.
const TASKBAR_POLL: Duration = Duration::from_millis(25);
/// Matches the UI's fade-out (`.app.closing` in styles.css).
const CLOSE_ANIMATION: Duration = Duration::from_millis(120);
const WINDOW_STATE_FILE: &str = "window-state.json";
/// Logical size the user last resized the popover to; overrides the Appearance size until
/// that size is changed in settings.
pub const POPOVER_STATE_FILE: &str = "popover-state.json";
/// A tray click that destroyed the popover by stealing focus must not reopen it.
const REOPEN_GUARD: Duration = Duration::from_millis(400);

/// True while the screen color picker overlay owns the focus; the popover must survive it.
#[derive(Default)]
pub struct PickerActive(pub AtomicBool);

#[derive(Default)]
pub struct LastBlurClose(Mutex<Option<Instant>>);

/// Tray / hotkey / second-launch entry point. Window creation must not run on the event
/// loop thread (it would deadlock), hence the task.
pub fn toggle(app: &AppHandle, anchor: Option<PhysicalPosition<f64>>) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Some(window) = app.get_webview_window(LABEL) {
            close(&window);
        } else if let Err(e) = open(&app, anchor, None) {
            eprintln!("trayforge: cannot open window: {e}");
        }
    });
}

/// Tray click variant of [`toggle`] that ignores the click which just closed the popover.
pub fn toggle_from_tray(app: &AppHandle, anchor: PhysicalPosition<f64>) {
    let recently_closed = app
        .state::<LastBlurClose>()
        .0
        .lock()
        .ok()
        .and_then(|last| *last)
        .is_some_and(|at| at.elapsed() < REOPEN_GUARD);
    if !recently_closed {
        toggle(app, Some(anchor));
    }
}

/// Opens the window (or focuses it) and optionally navigates the UI to `route`.
pub fn show(app: &AppHandle, route: Option<&'static str>) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Some(window) = app.get_webview_window(LABEL) {
            if let Some(route) = route {
                if let Err(e) = window.emit("navigate", route) {
                    eprintln!("trayforge: cannot emit navigate: {e}");
                }
            }
            focus(&window);
        } else if let Err(e) = open(&app, None, route) {
            eprintln!("trayforge: cannot open window: {e}");
        }
    });
}

/// Saves the window state, lets the UI play its fade-out and destroys the window.
pub fn close(window: &WebviewWindow) {
    if is_standalone(&settings::load_lenient(window.app_handle())) {
        save_state(window);
    } else {
        save_popover_size(window);
    }
    if let Err(e) = window.emit("closing", ()) {
        eprintln!("trayforge: cannot emit closing: {e}");
    }
    let window = window.clone();
    tauri::async_runtime::spawn_blocking(move || {
        std::thread::sleep(CLOSE_ANIMATION);
        if let Err(e) = window.destroy() {
            eprintln!("trayforge: cannot destroy window: {e}");
        }
    });
}

/// Forgets the resized size; an open popover goes back to the default size and spot.
pub fn reset_popover(app: &AppHandle) -> Result<(), String> {
    settings::remove_object(app, POPOVER_STATE_FILE)?;
    let settings = settings::load_lenient(app);
    if let Some(window) = app.get_webview_window(LABEL) {
        if !is_standalone(&settings) {
            place_popover(app, &window, None, default_size(&settings));
        }
    }
    Ok(())
}

fn default_size(settings: &Settings) -> (f64, f64) {
    (
        setting_size(settings, "popoverWidth", DEFAULT_SIZE.0),
        setting_size(settings, "popoverHeight", DEFAULT_SIZE.1),
    )
}

fn is_standalone(settings: &Settings) -> bool {
    settings
        .get("standaloneWindow")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn setting_size(settings: &Settings, key: &str, default: f64) -> f64 {
    settings
        .get(key)
        .and_then(Value::as_f64)
        .filter(|v| (300.0..=4000.0).contains(v))
        .unwrap_or(default)
}

fn open(
    app: &AppHandle,
    anchor: Option<PhysicalPosition<f64>>,
    route: Option<&'static str>,
) -> Result<(), String> {
    let settings = settings::load_lenient(app);
    let standalone = is_standalone(&settings);
    let remembered = settings::read_object(app, POPOVER_STATE_FILE).unwrap_or_default();
    let (default_width, default_height) = default_size(&settings);
    let width = setting_size(&remembered, "width", default_width);
    let height = setting_size(&remembered, "height", default_height);
    let effect = match settings.get("windowEffect").and_then(Value::as_str) {
        Some("mica") => Some(Effect::Mica),
        Some("blur") => Some(Effect::Blur),
        Some("none") => None,
        _ => Some(Effect::Acrylic),
    };

    let mut builder = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html".into()))
        .title("Trayforge")
        .inner_size(width, height)
        .transparent(effect.is_some())
        .disable_drag_drop_handler()
        .visible(false)
        .focused(true);

    if let Some(effect) = effect {
        builder = builder.effects(EffectsBuilder::new().effect(effect).build());
    }

    builder = if standalone {
        builder
            .decorations(true)
            .resizable(true)
            .min_inner_size(MIN_STANDALONE_SIZE.0, MIN_STANDALONE_SIZE.1)
    } else {
        builder
            .decorations(false)
            // Undecorated but still resizable from its edges, like a normal window.
            .resizable(true)
            .min_inner_size(MIN_POPOVER_SIZE.0, MIN_POPOVER_SIZE.1)
            .skip_taskbar(true)
            .always_on_top(true)
    };

    if let Some(route) = route {
        // The UI reads this on mount; the event in `on_page_load` only reaches an already
        // running UI.
        builder = builder.initialization_script(format!(
            "window.__TRAYFORGE_INITIAL_ROUTE__ = {};",
            json!(route)
        ));
    }
    builder = builder.on_page_load(move |window, payload| {
        if !matches!(payload.event(), tauri::webview::PageLoadEvent::Finished) {
            return;
        }
        if let Some(route) = route {
            if let Err(e) = window.emit("navigate", route) {
                eprintln!("trayforge: cannot emit navigate: {e}");
            }
        }
        // WebView2 takes a while to start, and Windows may refuse the focus requested
        // before the page exists. Ask again once the UI is there.
        focus(&window);
    });

    let window = builder.build().map_err(|e| e.to_string())?;
    let was_focused = Arc::new(AtomicBool::new(false));

    if standalone {
        restore_state(app, &window);
    } else {
        place_popover(app, &window, anchor, (width, height));
        follow_taskbar(window.clone(), was_focused.clone());
    }

    let handle = app.clone();
    window.on_window_event(move |event| match event {
        WindowEvent::Focused(true) => was_focused.store(true, Ordering::Relaxed),
        WindowEvent::Focused(false)
            if !standalone
                && was_focused.load(Ordering::Relaxed)
                && !handle.state::<PickerActive>().0.load(Ordering::Relaxed) =>
        {
            close_on_blur(&handle);
        }
        WindowEvent::CloseRequested { .. } if standalone => {
            if let Some(window) = handle.get_webview_window(LABEL) {
                save_state(&window);
            }
        }
        _ => {}
    });

    window.show().map_err(|e| e.to_string())?;
    focus(&window);
    Ok(())
}

/// Brings the window to the front. Focusing the webview itself is avoided on purpose: it
/// takes the focus from the window, and the popover would close on that blur.
fn focus(window: &WebviewWindow) {
    if let Err(e) = window.set_focus() {
        eprintln!("trayforge: cannot focus window: {e}");
    }
}

fn close_on_blur(app: &AppHandle) {
    if let Ok(mut last) = app.state::<LastBlurClose>().0.lock() {
        *last = Some(Instant::now());
    }
    if let Some(window) = app.get_webview_window(LABEL) {
        close(&window);
    }
}

/// The top-level window that owns the foreground window (dialogs resolve to their owner);
/// `None` while Windows is switching windows.
fn foreground_root() -> Option<isize> {
    let foreground = unsafe { GetForegroundWindow() };
    if foreground.is_invalid() {
        return None;
    }
    Some(unsafe { GetAncestor(foreground, GA_ROOTOWNER) }.0 as isize)
}

/// Next to the taskbar at its tray end, like the system tray flyouts, on the monitor under
/// the tray click (or the cursor). A tray click also sets the position along the taskbar.
fn place_popover(
    app: &AppHandle,
    window: &WebviewWindow,
    anchor: Option<PhysicalPosition<f64>>,
    (width, height): (f64, f64),
) {
    let Some(point) = anchor.or_else(|| app.cursor_position().ok()) else {
        return;
    };
    let monitor = app
        .monitor_from_point(point.x, point.y)
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else { return };

    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let (left, top) = (area.position.x as f64, area.position.y as f64);
    let (right, bottom) = (left + area.size.width as f64, top + area.size.height as f64);
    let margin = POPOVER_MARGIN * scale;
    let w = (width * scale).min(right - left - 2.0 * margin);
    let h = (height * scale).min(bottom - top - 2.0 * margin);

    if let Err(e) = window.set_size(PhysicalSize::new(w as u32, h as u32)) {
        eprintln!("trayforge: cannot size window: {e}");
    }
    // The outer frame of an undecorated window is still larger than the requested inner
    // size, so place it by the size Windows actually gave it.
    let (outer_w, outer_h) = window
        .outer_size()
        .map(|s| (s.width as f64, s.height as f64))
        .unwrap_or((w, h));

    let along = |anchor: f64, size: f64, min: f64, max: f64| {
        (anchor - size / 2.0).clamp(min + margin, max - margin - size)
    };
    let edge = taskbar::find(&monitor_rect(&monitor)).map_or(Edge::Bottom, |bar| bar.edge);
    let x = match (edge, anchor) {
        (Edge::Left, _) => left + margin,
        (Edge::Top | Edge::Bottom, Some(a)) => along(a.x, outer_w, left, right),
        _ => right - margin - outer_w,
    };
    let y = match (edge, anchor) {
        (Edge::Top, _) => top + margin,
        (Edge::Left | Edge::Right, Some(a)) => along(a.y, outer_h, top, bottom),
        _ => bottom - margin - outer_h,
    };

    if let Err(e) = window.set_position(PhysicalPosition::new(x as i32, y as i32)) {
        eprintln!("trayforge: cannot position window: {e}");
    }
}

fn monitor_rect(monitor: &tauri::Monitor) -> Rect {
    let (position, size) = (monitor.position(), monitor.size());
    Rect {
        left: position.x,
        top: position.y,
        right: position.x + size.width as i32,
        bottom: position.y + size.height as i32,
    }
}

/// While the popover is open, keeps it clear of an auto-hidden taskbar: when the taskbar
/// slides out over the window, the window moves aside with it and comes back when the
/// taskbar hides again. It only reacts to the taskbar, so dragging or resizing the window
/// is never fought. The thread ends with the window, keeping the idle tray process free
/// of timers.
///
/// It also closes a popover that Windows never let take the focus (so no blur event will
/// come) as soon as the user activates some other window.
fn follow_taskbar(window: WebviewWindow, was_focused: Arc<AtomicBool>) {
    let own = window.hwnd().map(|hwnd| hwnd.0 as isize).ok();
    // The window that was active when the popover opened: the taskbar after a tray click,
    // the current app after the hotkey.
    let opened_over = foreground_root();
    std::thread::spawn(move || {
        // How far the window currently stands off the place the user left it.
        let mut shift = 0;
        loop {
            std::thread::sleep(TASKBAR_POLL);
            let app = window.app_handle();
            if app.get_webview_window(LABEL).is_none() {
                return;
            }
            if let (Some(own), Some(active)) = (own, foreground_root()) {
                if !was_focused.load(Ordering::Relaxed)
                    && active != own
                    && Some(active) != opened_over
                    && !app.state::<PickerActive>().0.load(Ordering::Relaxed)
                {
                    close_on_blur(app);
                    return;
                }
            }
            let (Ok(position), Ok(size), Ok(Some(monitor))) = (
                window.outer_position(),
                window.outer_size(),
                window.current_monitor(),
            ) else {
                return;
            };
            let Some(bar) = taskbar::find(&monitor_rect(&monitor)) else {
                continue;
            };
            let (dx, dy) = match bar.edge {
                Edge::Bottom => (0, -1),
                Edge::Top => (0, 1),
                Edge::Left => (1, 0),
                Edge::Right => (-1, 0),
            };
            let (home_x, home_y) = (position.x - dx * shift, position.y - dy * shift);
            let home = Rect {
                left: home_x,
                top: home_y,
                right: home_x + size.width as i32,
                bottom: home_y + size.height as i32,
            };
            let gap = (POPOVER_MARGIN * monitor.scale_factor()) as i32;
            let wanted = bar.intrusion(&home, gap);
            if wanted != shift {
                let target = PhysicalPosition::new(home_x + dx * wanted, home_y + dy * wanted);
                if let Err(e) = window.set_position(target) {
                    eprintln!("trayforge: cannot move away from the taskbar: {e}");
                    return;
                }
                shift = wanted;
            }
        }
    });
}

/// Size the popover was resized to, restored on the next open.
fn save_popover_size(window: &WebviewWindow) {
    let (Ok(size), Ok(scale)) = (window.inner_size(), window.scale_factor()) else {
        return;
    };
    let size = size.to_logical::<f64>(scale);
    let mut state = Settings::new();
    state.insert("width".into(), size.width.round().into());
    state.insert("height".into(), size.height.round().into());
    if let Err(e) = settings::write_object(window.app_handle(), POPOVER_STATE_FILE, &state) {
        eprintln!("trayforge: cannot save popover size: {e}");
    }
}

fn save_state(window: &WebviewWindow) {
    let (Ok(position), Ok(size)) = (window.outer_position(), window.inner_size()) else {
        return;
    };
    let mut state = Settings::new();
    state.insert("x".into(), position.x.into());
    state.insert("y".into(), position.y.into());
    state.insert("width".into(), size.width.into());
    state.insert("height".into(), size.height.into());
    if let Err(e) = settings::write_object(window.app_handle(), WINDOW_STATE_FILE, &state) {
        eprintln!("trayforge: cannot save window state: {e}");
    }
}

fn restore_state(app: &AppHandle, window: &WebviewWindow) {
    let state = settings::read_object(app, WINDOW_STATE_FILE).unwrap_or_default();
    let number = |key: &str| state.get(key).and_then(Value::as_i64);
    let (Some(x), Some(y), Some(w), Some(h)) =
        (number("x"), number("y"), number("width"), number("height"))
    else {
        let _ = window.center();
        return;
    };
    // A monitor that was unplugged since the last run must not strand the window offscreen.
    let on_screen = matches!(
        app.monitor_from_point(x as f64 + 40.0, y as f64 + 40.0),
        Ok(Some(_))
    );
    if !on_screen {
        let _ = window.center();
        return;
    }
    if let Err(e) = window.set_position(PhysicalPosition::new(x as i32, y as i32)) {
        eprintln!("trayforge: cannot restore position: {e}");
    }
    if let Err(e) = window.set_size(PhysicalSize::new(w as u32, h as u32)) {
        eprintln!("trayforge: cannot restore size: {e}");
    }
}
