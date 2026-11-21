//! Where the popup opens: below the caret (or the pointer), kept fully on screen.
//!
//! Pure geometry so every edge case has a unit test. All rects are global points with a top-left
//! origin and y growing down, the space Accessibility returns; `mac::screens` converts AppKit's
//! bottom-left frames into it once.

#![forbid(unsafe_code)]

/// Space kept between the popup and the screen edges, and between the popup and its anchor.
const MARGIN: f64 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub const fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self { x, y, w, h }
    }

    fn right(&self) -> f64 {
        self.x + self.w
    }

    fn bottom(&self) -> f64 {
        self.y + self.h
    }

    fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }

    fn inset(&self, by: f64) -> Rect {
        Rect::new(self.x + by, self.y + by, (self.w - 2.0 * by).max(0.0), (self.h - 2.0 * by).max(0.0))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Screen {
    pub frame: Rect,
    /// The frame without the menu bar and the Dock.
    pub visible: Rect,
    pub scale: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    /// Top-left of the popup in global points.
    pub x: f64,
    pub y: f64,
    /// Backing scale of the screen the popup is on, to convert to physical pixels.
    pub scale: f64,
}

/// Returns the caret rect only if it can be trusted. Electron and some web views return empty
/// rects, rects at (0, 0), or the whole text area; the pointer is a better anchor then.
pub fn usable_caret(caret: Rect, screens: &[Screen]) -> Option<Rect> {
    let at_origin = caret.x == 0.0 && caret.y == 0.0;
    if caret.h <= 0.0 || caret.w < 0.0 || at_origin {
        return None;
    }
    let screen = screen_at(caret.x, caret.y + caret.h / 2.0, screens)?;
    if !screen.frame.contains(caret.x, caret.y + caret.h / 2.0) || caret.h > screen.frame.h / 2.0 {
        return None;
    }
    Some(caret)
}

/// Places a popup of `size` (w, h) next to `anchor`: below it if it fits, else above it, else
/// pinned to the bottom of the screen; always inside the visible frame of the anchor's screen.
/// A pointer anchor is a zero-size rect.
pub fn place(anchor: Rect, size: (f64, f64), screens: &[Screen]) -> Placement {
    let (w, h) = size;
    let center_y = anchor.y + anchor.h / 2.0;
    let Some(screen) = screen_at(anchor.x, center_y, screens).or(screens.first()) else {
        return Placement { x: anchor.x, y: anchor.bottom() + MARGIN, scale: 1.0 };
    };
    let area = screen.visible.inset(MARGIN);

    let below = anchor.bottom() + MARGIN;
    let above = anchor.y - MARGIN - h;
    let lowest = (area.bottom() - h).max(area.y);
    let y = if below + h <= area.bottom() {
        below
    } else if above >= area.y {
        above
    } else {
        lowest
    };
    // A caret in the menu bar, or an anchor on no screen, can still give a y outside the area.
    let y = y.min(lowest).max(area.y);
    let x = anchor.x.min(area.right() - w).max(area.x);
    Placement { x, y, scale: screen.scale }
}

/// Converts an AppKit rect (bottom-left origin at the primary screen's bottom, y up) to the
/// top-left space AX uses. `primary_height` is the height of the screen with the menu bar.
pub fn from_appkit(rect: Rect, primary_height: f64) -> Rect {
    Rect::new(rect.x, primary_height - rect.y - rect.h, rect.w, rect.h)
}

/// The screen whose full frame contains the point; screens are disjoint in global space.
fn screen_at(x: f64, y: f64, screens: &[Screen]) -> Option<&Screen> {
    screens.iter().find(|screen| screen.frame.contains(x, y))
}

#[cfg(test)]
#[path = "placement_tests.rs"]
mod tests;
