//! Launch at login through `SMAppService.mainApp` (macOS 13+). macOS keeps the state, so Retone
//! stores nothing in prefs and reads the status each time Settings shows it.

use objc2_service_management::{SMAppService, SMAppServiceStatus};

/// On when macOS will open Retone at login. "Requires approval" counts as on: the user turned
/// it on and macOS waits for the switch in System Settings › Login Items.
pub fn is_enabled() -> bool {
    // SAFETY: `mainAppService` returns the shared service object for this app; `status` only reads it.
    let status = unsafe { SMAppService::mainAppService().status() };
    status == SMAppServiceStatus::Enabled || status == SMAppServiceStatus::RequiresApproval
}

/// Turns launch at login on or off. The error is the system's text, for the Settings row.
pub fn set_enabled(on: bool) -> Result<(), String> {
    // SAFETY: as above; register and unregister are the documented calls on the main app service
    // and report failure through the returned NSError.
    let result = unsafe {
        let service = SMAppService::mainAppService();
        if on { service.registerAndReturnError() } else { service.unregisterAndReturnError() }
    };
    result.map_err(|error| error.localizedDescription().to_string())
}
