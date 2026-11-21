//! State both windows read: the prefs store, the Local AI API key, and the shortcut Test row's
//! flag.

#![forbid(unsafe_code)]

use crate::config::{APP_ID, Config};
use rune_core::prefs::Prefs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};

/// Pref that says a key is in the Keychain, so Settings can show it without reading the Keychain.
const API_KEY_SAVED: &str = "local_api_key_saved";

pub struct Shared {
    prefs: Mutex<Prefs>,
    /// Headless renders and tests keep the API key here and never touch the Keychain.
    memory_key: Option<Mutex<Option<String>>>,
    /// Set by Settings' Test row: the next hotkey press only proves the shortcut works.
    testing_hotkey: AtomicBool,
}

impl Shared {
    /// The real `prefs.json`, or an in-memory store for headless renders and tests, which must
    /// never touch the user's file.
    pub fn new(persistent: bool) -> Shared {
        let prefs = if persistent { Prefs::open(APP_ID) } else { Prefs::memory() };
        let memory_key = (!persistent || !cfg!(target_os = "macos")).then(|| Mutex::new(None));
        Shared { prefs: Mutex::new(prefs), memory_key, testing_hotkey: AtomicBool::new(false) }
    }

    pub fn prefs(&self) -> MutexGuard<'_, Prefs> {
        // A panic while holding the lock leaves the prefs as they were last saved; keep going.
        self.prefs.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn config(&self) -> Config {
        Config::load(&self.prefs())
    }

    pub fn api_key_saved(&self) -> bool {
        self.prefs().get(API_KEY_SAVED).unwrap_or(false)
    }

    /// The Local AI API key. Blocking (a Keychain prompt can show): call it from a worker.
    pub fn api_key(&self) -> Option<String> {
        if let Some(memory) = &self.memory_key {
            return memory.lock().unwrap_or_else(|p| p.into_inner()).clone();
        }
        #[cfg(target_os = "macos")]
        return crate::mac::keychain::local_api_key();
        #[cfg(not(target_os = "macos"))]
        None
    }

    /// Saves the key (`Some`) or removes it (`None`).
    pub fn set_api_key(&self, key: Option<&str>) -> Result<(), String> {
        if let Some(memory) = &self.memory_key {
            *memory.lock().unwrap_or_else(|p| p.into_inner()) = key.map(str::to_string);
        } else {
            #[cfg(target_os = "macos")]
            match key {
                Some(key) => crate::mac::keychain::set_local_api_key(key)?,
                None => crate::mac::keychain::remove_local_api_key()?,
            }
        }
        self.prefs().set(API_KEY_SAVED, &key.is_some()).map_err(|e| e.to_string())
    }

    pub fn set_testing(&self, on: bool) {
        self.testing_hotkey.store(on, Ordering::Relaxed);
    }

    /// True once per armed test: the press is used up by the Test row.
    pub fn take_testing(&self) -> bool {
        self.testing_hotkey.swap(false, Ordering::Relaxed)
    }
}
