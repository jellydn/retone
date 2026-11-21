//! State both windows read: the prefs store and the shortcut Test row's flag.

#![forbid(unsafe_code)]

use crate::config::{APP_ID, Config};
use rune_core::prefs::Prefs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};

pub struct Shared {
    prefs: Mutex<Prefs>,
    /// Set by Settings' Test row: the next hotkey press only proves the shortcut works.
    testing_hotkey: AtomicBool,
}

impl Shared {
    /// The real `prefs.json`, or an in-memory store for headless renders and tests, which must
    /// never touch the user's file.
    pub fn new(persistent: bool) -> Shared {
        let prefs = if persistent { Prefs::open(APP_ID) } else { Prefs::memory() };
        Shared { prefs: Mutex::new(prefs), testing_hotkey: AtomicBool::new(false) }
    }

    pub fn prefs(&self) -> MutexGuard<'_, Prefs> {
        // A panic while holding the lock leaves the prefs as they were last saved; keep going.
        self.prefs.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn config(&self) -> Config {
        Config::load(&self.prefs())
    }

    pub fn set_testing(&self, on: bool) {
        self.testing_hotkey.store(on, Ordering::Relaxed);
    }

    /// True once per armed test: the press is used up by the Test row.
    pub fn take_testing(&self) -> bool {
        self.testing_hotkey.swap(false, Ordering::Relaxed)
    }
}
