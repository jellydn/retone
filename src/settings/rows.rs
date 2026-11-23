//! The Settings page parts, after Hex's settings (github.com/anomalyco/hex, MIT): a raised card
//! of rows with thin separators, each row an icon, a title, an optional grey help line and a
//! control on the right.

use rune::{Color, El, Icon, TextStyle, col, el, icon, row, text};
use rune_kit::Kit;

/// A row's text inset inside its card.
const ROW_PX: f32 = 16.0;

/// The page title: larger than the rows, so the page reads at a glance.
pub fn page_title(k: &Kit, title: &str) -> El {
    text(title.to_string())
        .key("page-title")
        .role("heading")
        .level(1)
        .font(TextStyle::new(20.0, 600.0).lh(28.0))
        .color(k.c.text_base)
}

/// A raised card of rows with a thin line between each pair.
pub fn card(k: &Kit, key: &str, rows: Vec<El>) -> El {
    let mut card = col().key(key.to_string()).w_full().fill(k.c.bg_base).radius(k.radius.lg);
    for shadow in &k.elevation.raised {
        card = card.box_shadow(*shadow);
    }
    for (index, item) in rows.into_iter().enumerate() {
        if index > 0 {
            card = card.child(
                el().key(format!("sep{index}")).h(1.0).ml(ROW_PX).mr(ROW_PX).fill(k.c.border_muted).a11y_hidden(),
            );
        }
        card = card.child(item);
    }
    card
}

/// A grey help line for [`setting`].
pub fn help(k: &Kit, body: impl Into<String>) -> Option<(String, Color)> {
    Some((body.into(), k.c.text_muted))
}

/// One setting: icon, title and help line on the left, `control` on the right. The help line
/// takes its own colour for errors and warnings.
pub fn setting(
    k: &Kit,
    key: &str,
    glyph: &'static Icon,
    title: &str,
    help: Option<(String, Color)>,
    control: El,
) -> El {
    let described = help.is_some();
    // The drawn help line is hidden from the tree; a reader hears it as the row's description.
    let description = help.as_ref().map(|(body, _)| body.clone()).unwrap_or_default();
    let copy = col()
        .key("copy")
        .grow(1.0)
        .shrink()
        .min_w(0.0)
        .gap(2.0)
        .child(text(title.to_string()).key("title").font(k.ty.body).color(k.c.text_base).a11y_hidden())
        .children(
            help.map(|(body, color)| text(body).key("help").font(k.ty.text_xsmall).color(color).wrap().a11y_hidden()),
        );
    row()
        .key(key.to_string())
        .w_full()
        .min_h(if described { 60.0 } else { 48.0 })
        .px(ROW_PX)
        .py(10.0)
        .gap(12.0)
        .center_y()
        .label(title.to_string())
        .description(description)
        .child(icon(glyph, 16.0).key("icon").color(k.c.icon_base).a11y_hidden())
        .child(copy)
        .child(control)
}

/// A value that opens a sub-page: the value, then ›.
pub fn value_link<S: 'static>(k: &Kit, key: &str, value: &str, press: impl Fn(&mut S) + 'static) -> El {
    row()
        .key(key.to_string())
        .role("button")
        .a11y_value(value.to_string())
        .focusable(true)
        .focus_ring(2.0, 0.0, k.c.border_focus)
        .h(28.0)
        .pl(8.0)
        .pr(4.0)
        .gap(4.0)
        .center_y()
        .radius(k.radius.sm)
        .hover(k.c.hover)
        .press(k.c.pressed)
        .on_press(press)
        .child(text(value.to_string()).key("value").font(k.ty.ui_medium).color(k.c.text_base).truncate().max_w(260.0))
        .child(icon(&super::icons::CHEVRON_RIGHT, 16.0).key("chevron").color(k.c.icon_muted).a11y_hidden())
}
