use super::*;

const POPUP: (f64, f64) = (420.0, 240.0);

/// A 1440×900 primary screen with a 25 pt menu bar and a 70 pt Dock at the bottom.
fn laptop() -> Screen {
    Screen { frame: Rect::new(0.0, 0.0, 1440.0, 900.0), visible: Rect::new(0.0, 25.0, 1440.0, 805.0), scale: 2.0 }
}

/// A 1920×1080 scale-1 display placed above the primary one, as macOS allows.
fn display_above() -> Screen {
    Screen {
        frame: Rect::new(-200.0, -1080.0, 1920.0, 1080.0),
        visible: Rect::new(-200.0, -1080.0, 1920.0, 1080.0),
        scale: 1.0,
    }
}

fn caret(x: f64, y: f64) -> Rect {
    Rect::new(x, y, 2.0, 18.0)
}

#[test]
fn opens_below_the_caret_aligned_to_its_left_edge() {
    let at = place(caret(300.0, 200.0), POPUP, &[laptop()]);
    assert_eq!(at, Placement { x: 300.0, y: 226.0, scale: 2.0 });
}

#[test]
fn flips_above_when_the_space_below_is_too_small() {
    // Caret bottom at 718: below would end at 966, past the Dock's top at 830 − 8.
    let at = place(caret(300.0, 700.0), POPUP, &[laptop()]);
    assert_eq!(at.y, 700.0 - 8.0 - 240.0);
}

#[test]
fn pins_to_the_bottom_when_it_fits_neither_below_nor_above() {
    let tall = (420.0, 700.0);
    let at = place(caret(300.0, 400.0), tall, &[laptop()]);
    assert_eq!(at.y, 830.0 - 8.0 - 700.0);
}

#[test]
fn a_popup_taller_than_the_screen_keeps_its_top_on_screen() {
    let at = place(caret(300.0, 400.0), (420.0, 2000.0), &[laptop()]);
    assert_eq!(at.y, 25.0 + 8.0);
}

#[test]
fn shifts_left_at_the_right_edge_and_right_at_the_left_edge() {
    let right = place(caret(1430.0, 200.0), POPUP, &[laptop()]);
    assert_eq!(right.x, 1440.0 - 8.0 - 420.0);
    let left = place(caret(2.0, 200.0), POPUP, &[laptop()]);
    assert_eq!(left.x, 8.0);
}

#[test]
fn stays_below_the_menu_bar_for_a_caret_at_the_top() {
    let at = place(caret(300.0, 0.5), POPUP, &[laptop()]);
    assert!(at.y >= 33.0, "y = {}", at.y);
}

#[test]
fn uses_the_screen_of_the_anchor_and_its_scale() {
    let screens = [laptop(), display_above()];
    let at = place(caret(100.0, -500.0), POPUP, &screens);
    assert_eq!(at, Placement { x: 100.0, y: -500.0 + 18.0 + 8.0, scale: 1.0 });
    // Near the bottom of the upper display: flips above instead of crossing onto the laptop.
    let low = place(caret(100.0, -30.0), POPUP, &screens);
    assert_eq!(low.y, -30.0 - 8.0 - 240.0);
}

#[test]
fn a_pointer_anchor_is_a_point() {
    let at = place(Rect::new(500.0, 300.0, 0.0, 0.0), POPUP, &[laptop()]);
    assert_eq!((at.x, at.y), (500.0, 308.0));
}

#[test]
fn an_anchor_off_every_screen_falls_back_to_the_primary_screen() {
    let at = place(caret(5000.0, 5000.0), POPUP, &[laptop()]);
    assert_eq!((at.x, at.y), (1440.0 - 8.0 - 420.0, 830.0 - 8.0 - 240.0));
}

#[test]
fn appkit_frames_flip_to_top_left_space() {
    // The laptop's visible frame in AppKit: Dock 70 pt at the bottom, menu bar 25 pt at the top.
    assert_eq!(from_appkit(Rect::new(0.0, 70.0, 1440.0, 805.0), 900.0), laptop().visible);
    // A display above the primary one has a positive AppKit y and a negative top-left y.
    assert_eq!(from_appkit(Rect::new(-200.0, 900.0, 1920.0, 1080.0), 900.0), display_above().frame);
    // The mouse is a point: AppKit y 100 is 800 pt down from the primary screen's top.
    assert_eq!(from_appkit(Rect::new(10.0, 100.0, 0.0, 0.0), 900.0), Rect::new(10.0, 800.0, 0.0, 0.0));
}

#[test]
fn bad_caret_rects_are_rejected() {
    let screens = [laptop()];
    assert_eq!(usable_caret(caret(300.0, 200.0), &screens), Some(caret(300.0, 200.0)));
    assert_eq!(usable_caret(Rect::new(0.0, 0.0, 2.0, 18.0), &screens), None, "at the origin");
    assert_eq!(usable_caret(Rect::new(300.0, 200.0, 0.0, 0.0), &screens), None, "zero size");
    assert_eq!(usable_caret(Rect::new(300.0, 100.0, 600.0, 451.0), &screens), None, "taller than half the screen");
    assert_eq!(usable_caret(caret(3000.0, 200.0), &screens), None, "off every screen");
    // An insertion point is zero width but has a height: usable.
    assert!(usable_caret(Rect::new(300.0, 200.0, 0.0, 18.0), &screens).is_some());
}
