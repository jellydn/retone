//! Diagnostics for the S1/S2 spikes, off unless `RETONE_SPIKE_LOG=1` is set at launch.
//!
//! Lines go to stderr and never contain selected or rewritten text: only bundle IDs, which
//! capture path ran, text lengths, rects and results. Remove once the app matrix (Phase 2)
//! replaces the spikes.

#![forbid(unsafe_code)]

use std::sync::OnceLock;

pub fn enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| rune_core::env::var("RETONE_SPIKE_LOG").is_some_and(|v| v == "1"))
}

macro_rules! spike {
    ($($arg:tt)*) => {
        if $crate::spike_log::enabled() {
            eprintln!("[retone-spike] {}", format!($($arg)*));
        }
    };
}
pub(crate) use spike;
