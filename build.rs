//! Turns the JSON catalogs in `locales/` into `$OUT_DIR/i18n.rs`, which `src/i18n.rs` includes.

use std::path::{Path, PathBuf};

fn main() -> std::io::Result<()> {
    let out_dir =
        PathBuf::from(std::env::var_os("OUT_DIR").ok_or_else(|| std::io::Error::other("OUT_DIR is not set"))?);
    rune_i18n::codegen::generate(Path::new("locales"), &out_dir)
}
