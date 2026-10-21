//! Where the Windows taskbar is on a monitor, so the popover can sit next to it and step
//! aside when an auto-hidden taskbar slides out over it.

use windows::{
    core::{w, PCWSTR},
    Win32::{
        Foundation::{HWND, RECT},
        UI::WindowsAndMessaging::{FindWindowExW, GetWindowRect},
    },
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Edge {
    Left,
    Top,
    Right,
    Bottom,
}

#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    fn intersects(&self, other: &Rect) -> bool {
        self.left < other.right
            && other.left < self.right
            && self.top < other.bottom
            && other.top < self.bottom
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Taskbar {
    pub edge: Edge,
    /// Current screen rectangle; an auto-hidden taskbar lies almost entirely off screen.
    pub rect: Rect,
}

impl Taskbar {
    /// How far `window` must move away from the taskbar to keep `gap` pixels from it.
    pub fn intrusion(&self, window: &Rect, gap: i32) -> i32 {
        let overlaps_along = match self.edge {
            Edge::Top | Edge::Bottom => {
                window.left < self.rect.right && self.rect.left < window.right
            }
            Edge::Left | Edge::Right => {
                window.top < self.rect.bottom && self.rect.top < window.bottom
            }
        };
        if !overlaps_along {
            return 0;
        }
        let depth = match self.edge {
            Edge::Bottom => window.bottom - (self.rect.top - gap),
            Edge::Top => (self.rect.bottom + gap) - window.top,
            Edge::Left => (self.rect.right + gap) - window.left,
            Edge::Right => window.right - (self.rect.left - gap),
        };
        depth.max(0)
    }
}

/// The taskbar of `monitor` (primary or secondary), if that monitor has one.
pub fn find(monitor: &Rect) -> Option<Taskbar> {
    [w!("Shell_TrayWnd"), w!("Shell_SecondaryTrayWnd")]
        .into_iter()
        .flat_map(windows_of_class)
        .filter_map(|hwnd| {
            let mut r = RECT::default();
            // SAFETY: `hwnd` came from FindWindowExW and `r` outlives the call.
            unsafe { GetWindowRect(hwnd, &mut r) }.ok()?;
            Some(Rect {
                left: r.left,
                top: r.top,
                right: r.right,
                bottom: r.bottom,
            })
        })
        // A hidden taskbar still keeps a sliver on screen, so it touches its monitor.
        .find(|rect| rect.intersects(monitor))
        .map(|rect| Taskbar {
            edge: edge_of(&rect, monitor),
            rect,
        })
}

fn windows_of_class(class: PCWSTR) -> Vec<HWND> {
    let mut found = Vec::new();
    let mut after = None;
    // SAFETY: plain window enumeration; a failed lookup ends the loop.
    while let Ok(hwnd) = unsafe { FindWindowExW(None, after, class, PCWSTR::null()) } {
        found.push(hwnd);
        after = Some(hwnd);
    }
    found
}

fn edge_of(bar: &Rect, monitor: &Rect) -> Edge {
    let horizontal = bar.right - bar.left >= bar.bottom - bar.top;
    let (bar_center_x, bar_center_y) = ((bar.left + bar.right) / 2, (bar.top + bar.bottom) / 2);
    let (mon_center_x, mon_center_y) = (
        (monitor.left + monitor.right) / 2,
        (monitor.top + monitor.bottom) / 2,
    );
    match (
        horizontal,
        bar_center_y < mon_center_y,
        bar_center_x < mon_center_x,
    ) {
        (true, true, _) => Edge::Top,
        (true, false, _) => Edge::Bottom,
        (false, _, true) => Edge::Left,
        (false, _, false) => Edge::Right,
    }
}
