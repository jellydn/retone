//! The UI's words, from the catalogs in `locales/`. English is the source: a key another locale
//! does not have falls back to it. Model prompts and the style rules stay in English.

mod catalog {
    include!(concat!(env!("OUT_DIR"), "/i18n.rs"));
}

pub use catalog::KEYS;
pub use rune_i18n::Key;

pub static I18N: rune_i18n::I18n = rune_i18n::I18n::new(&catalog::CATALOG);

/// Uses the supported locale that best matches the system's preferred languages.
pub fn use_system_language() {
    let preferred = rune_i18n::system_languages();
    I18N.set_locale(I18N.negotiate(preferred.iter().map(String::as_str)));
}

/// The message for a key, checked at compile time: `t!("popup.copy")`. With arguments, fills its
/// `{{name}}` placeholders and returns a `String`: `t!("popup.replace_failed", error = e)`.
macro_rules! t {
    ($key:literal) => {
        $crate::i18n::I18N.text(key!($key))
    };
    ($key:literal, $($name:ident = $value:expr),+ $(,)?) => {
        $crate::i18n::I18N.format(key!($key), &[$((stringify!($name), &$value as &dyn ::std::fmt::Display)),+])
    };
}

/// The `Key` for a name, as a constant: a name missing from `locales/app/en.json` fails the build.
macro_rules! key {
    ($key:literal) => {{
        const KEY: $crate::i18n::Key = $crate::i18n::Key::named($crate::i18n::KEYS, $key);
        KEY
    }};
}
