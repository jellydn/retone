//! Settings icons: 24-unit stroke drawings in the style of Hex's sidebar icons. Rune paints
//! `currentColor` with the element's colour, so one drawing serves every theme.

use rune::Icon;

macro_rules! stroke_icon {
    ($id:ident, $name:literal, $body:literal) => {
        pub static $id: Icon = Icon {
            name: concat!("retone-", $name),
            view: 24.0,
            body: concat!(
                r#"<g fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">"#,
                $body,
                "</g>"
            ),
        };
    };
}

stroke_icon!(
    GENERAL,
    "general",
    r#"<path d="M4 6h10M4 12h4M12 12h8M4 18h12"/><circle cx="16" cy="6" r="2"/><circle cx="10" cy="12" r="2"/><circle cx="18" cy="18" r="2"/>"#
);
stroke_icon!(
    PROVIDER,
    "provider",
    r#"<rect x="4" y="4" width="16" height="7" rx="2"/><rect x="4" y="13" width="16" height="7" rx="2"/><path d="M8 7.5h.01M8 16.5h.01"/>"#
);
stroke_icon!(STYLES, "styles", r#"<path d="M4 20l4-1L19 8a2.1 2.1 0 0 0-3-3L5 16z"/><path d="M14 7l3 3"/>"#);
stroke_icon!(PRIVACY, "privacy", r#"<path d="M12 3l7 3v5c0 4.5-3 8-7 10-4-2-7-5.5-7-10V6z"/>"#);
stroke_icon!(KEY, "key", r#"<circle cx="8" cy="15" r="4"/><path d="M11 12l8-8M16 7l2 2M14 9l2 2"/>"#);
stroke_icon!(
    GLOBE,
    "globe",
    r#"<circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3c2.5 2.5 3.8 5.5 3.8 9s-1.3 6.5-3.8 9c-2.5-2.5-3.8-5.5-3.8-9S9.5 5.5 12 3z"/>"#
);
stroke_icon!(NETWORK, "network", r#"<path d="M5 12.5a10 10 0 0 1 14 0M8 15.5a6 6 0 0 1 8 0"/><path d="M12 19h.01"/>"#);
stroke_icon!(MODEL, "model", r#"<path d="M12 3l8 4.5v9L12 21l-8-4.5v-9z"/><path d="M4 7.5l8 4.5 8-4.5M12 12v9"/>"#);
stroke_icon!(
    TERMINAL,
    "terminal",
    r#"<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M7 9l3 3-3 3M13 15h4"/>"#
);
stroke_icon!(THEME, "theme", r#"<circle cx="12" cy="12" r="9"/><path d="M12 3a9 9 0 0 0 0 18z" fill="currentColor"/>"#);
stroke_icon!(POWER, "power", r#"<path d="M12 3v8"/><path d="M6.5 6.5a8 8 0 1 0 11 0"/>"#);
stroke_icon!(
    ACCESSIBILITY,
    "accessibility",
    r#"<circle cx="12" cy="5" r="1.5"/><path d="M5 9l7 1.5L19 9M12 10.5V15l-3 6M12 15l3 6"/>"#
);
stroke_icon!(PLAY, "play", r#"<path d="M8 5l11 7-11 7z"/>"#);
stroke_icon!(LAYERS, "layers", r#"<path d="M12 3l9 5-9 5-9-5z"/><path d="M3 13l9 5 9-5"/>"#);
stroke_icon!(RESET, "reset", r#"<path d="M4 12a8 8 0 1 0 2.5-5.8"/><path d="M4 4v4h4"/>"#);
stroke_icon!(WARNING, "warning", r#"<path d="M12 4l9 16H3z"/><path d="M12 10v4M12 17h.01"/>"#);
stroke_icon!(NOTE, "note", r#"<path d="M6 3h9l4 4v14H6z"/><path d="M9 12h7M9 16h5"/>"#);
stroke_icon!(
    BOOK,
    "book",
    r#"<path d="M5 4h11a3 3 0 0 1 3 3v13H8a3 3 0 0 1-3-3z"/><path d="M5 17a3 3 0 0 1 3-3h11"/>"#
);
stroke_icon!(CHEVRON_RIGHT, "chevron-right", r#"<path d="M9 6l6 6-6 6"/>"#);
stroke_icon!(CHEVRON_LEFT, "chevron-left", r#"<path d="M15 6l-6 6 6 6"/>"#);
