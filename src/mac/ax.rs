//! Accessibility (AX) reads on the focused element of the frontmost app.
//!
//! Calls go to another process and can block, so they run off the frame loop with a short
//! messaging timeout. Nothing here writes to the target app.

use crate::placement::Rect;
use objc2_application_services::{
    AXError, AXIsProcessTrusted, AXIsProcessTrustedWithOptions, AXUIElement, AXValue, AXValueType,
};
use objc2_core_foundation::{CFBoolean, CFDictionary, CFRange, CFRetained, CFString, CFType, CGRect};
use std::ffi::c_void;
use std::ptr::NonNull;

/// An app that does not answer within this time is treated as having no AX text; the ⌘C
/// fallback then runs instead of the hotkey hanging.
const MESSAGING_TIMEOUT_SECS: f32 = 0.25;
/// Some apps put the selection on a parent of the focused element (a web area inside a group).
const MAX_PARENT_STEPS: usize = 3;

pub fn is_trusted() -> bool {
    // SAFETY: takes no arguments and only reads this process's TCC state.
    unsafe { AXIsProcessTrusted() }
}

/// Shows the system prompt that sends the user to Accessibility settings, once per launch.
pub fn request_trust() -> bool {
    // The constant's value is documented as "AXTrustedCheckOptionPrompt".
    let key = CFString::from_static_str("AXTrustedCheckOptionPrompt");
    let options = CFDictionary::<CFString, CFBoolean>::from_slices(&[&key], &[CFBoolean::new(true)]);
    // SAFETY: `options` is a valid CFDictionary of CFString to CFBoolean, as the API expects.
    unsafe { AXIsProcessTrustedWithOptions(Some(options.as_opaque())) }
}

/// The element the user typed in, captured at hotkey time and checked again before Replace.
pub struct Target {
    pub pid: i32,
    element: CFRetained<AXUIElement>,
    /// The selection range when the element reports one.
    pub range: Option<CFRange>,
}

impl Target {
    /// The focused element of the frontmost app, or None when AX gives none (no permission,
    /// an app without AX, or a timeout).
    pub fn focused() -> Option<Target> {
        // SAFETY: creates a new system-wide element; no arguments.
        let system = unsafe { AXUIElement::new_system_wide() };
        set_timeout(&system);
        let app = copy_attribute(&system, "AXFocusedApplication")?.downcast::<AXUIElement>().ok()?;
        Target::focused_in_app(&app)
    }

    /// The focused element inside the app with this pid, whichever app has key focus now.
    pub fn focused_in(pid: i32) -> Option<Target> {
        // SAFETY: creates an AX element for a pid; an exited process only makes later calls fail.
        let app = unsafe { AXUIElement::new_application(pid) };
        Target::focused_in_app(&app)
    }

    fn focused_in_app(app: &AXUIElement) -> Option<Target> {
        set_timeout(app);
        let element = copy_attribute(app, "AXFocusedUIElement")?.downcast::<AXUIElement>().ok()?;
        set_timeout(&element);
        let mut pid = 0;
        // SAFETY: `pid` is a valid out pointer for a pid_t.
        if unsafe { element.pid(NonNull::from(&mut pid)) } != AXError::Success {
            return None;
        }
        let range = selected_range(&element);
        Some(Target { pid, element, range })
    }

    /// Password fields are refused before any text is read or sent.
    pub fn is_secure(&self) -> bool {
        ["AXRole", "AXSubrole"]
            .iter()
            .any(|attribute| copy_string(&self.element, attribute).is_some_and(|value| value == "AXSecureTextField"))
    }

    /// The selected text: `AXSelectedText`, else the string for the selected range, on the
    /// element or one of its first parents. Empty means "no AX text"; the caller falls back to ⌘C.
    pub fn selected_text(&self) -> Option<String> {
        let mut element = self.element.clone();
        for _ in 0..=MAX_PARENT_STEPS {
            if let Some(text) = copy_string(&element, "AXSelectedText").filter(|text| !text.is_empty()) {
                return Some(text);
            }
            if let Some(range) = selected_range(&element).filter(|range| range.length > 0)
                && let Some(text) = string_for_range(&element, range).filter(|text| !text.is_empty())
            {
                return Some(text);
            }
            element = copy_attribute(&element, "AXParent")?.downcast::<AXUIElement>().ok()?;
        }
        None
    }

    /// Bounds of the end of the selection in global top-left points: the last character when
    /// there is a selection, else the insertion point. Callers still check it with
    /// `placement::usable_caret`, because some apps return nonsense here.
    pub fn caret_bounds(&self) -> Option<Rect> {
        let range = self.range?;
        let end = range.location + range.length;
        // A one-character range at the end gives the last line, not the whole multi-line block.
        let probe = if range.length > 0 { CFRange { location: end - 1, length: 1 } } else { range };
        let rect = bounds_for_range(&self.element, probe)
            .or_else(|| bounds_for_range(&self.element, CFRange { location: end, length: 0 }))?;
        Some(Rect::new(rect.origin.x, rect.origin.y, rect.size.width, rect.size.height))
    }

    /// Checks that Replace would land where the rewrite came from: same app, same selection
    /// range, and the same selected text when read again now. Err names the first check that
    /// failed. Element identity alone does not decide: Chromium and Electron (Slack, spike S2)
    /// hand out a new element object for the same field on each query.
    ///
    /// The check reads the target app's own focused element, not the system-wide one: Retone's
    /// popup holds key focus during Replace, so system-wide AX reports Retone as the focused app.
    /// The caller checks the frontmost app separately, after this, so that check is the last one
    /// before the paste.
    ///
    /// Chromium and Electron apps (Slack) report no focused element at all while their window is
    /// not key, but the captured element still answers. Then the captured element is read again,
    /// and the same range and text checks decide.
    pub fn still_matches(&self, captured_text: &str) -> Result<(), &'static str> {
        // `same_object` is false on the fallback, so the text is always compared there: no focus
        // query proves that the field is still the one the user works in.
        let (now, same_object) = match Target::focused_in(self.pid) {
            Some(now) => {
                let same_object = now.element == self.element;
                (now, same_object)
            }
            None => {
                let range = selected_range(&self.element).ok_or("no focused element")?;
                (Target { pid: self.pid, element: self.element.clone(), range: Some(range) }, false)
            }
        };
        let same_range = match (self.range, now.range) {
            (Some(a), Some(b)) => a.location == b.location && a.length == b.length,
            (None, None) => true,
            _ => false,
        };
        if !same_range {
            return Err("the selection range moved");
        }
        if same_object {
            return Ok(());
        }
        // Not proven the same object: accept it only if it still holds the captured selection.
        match now.selected_text() {
            Some(text) if text == captured_text => Ok(()),
            Some(_) => Err("the selected text differs"),
            None => Err("the field's selection cannot be read again"),
        }
    }
}

fn set_timeout(element: &AXUIElement) {
    // SAFETY: `element` is a live AXUIElement; the call only stores the timeout.
    unsafe { element.set_messaging_timeout(MESSAGING_TIMEOUT_SECS) };
}

fn copy_attribute(element: &AXUIElement, name: &str) -> Option<CFRetained<CFType>> {
    let name = CFString::from_str(name);
    let mut value: *const CFType = std::ptr::null();
    // SAFETY: `value` is a valid out pointer; on success it holds an owned (+1) reference.
    let status = unsafe { element.copy_attribute_value(&name, NonNull::from(&mut value)) };
    owned(status, value)
}

fn copy_string(element: &AXUIElement, name: &str) -> Option<String> {
    Some(copy_attribute(element, name)?.downcast::<CFString>().ok()?.to_string())
}

fn selected_range(element: &AXUIElement) -> Option<CFRange> {
    let value = copy_attribute(element, "AXSelectedTextRange")?.downcast::<AXValue>().ok()?;
    let mut range = CFRange { location: 0, length: 0 };
    // SAFETY: the out pointer matches the requested type, CFRange.
    let ok = unsafe { value.value(AXValueType::CFRange, NonNull::from(&mut range).cast::<c_void>()) };
    ok.then_some(range)
}

fn string_for_range(element: &AXUIElement, range: CFRange) -> Option<String> {
    let parameter = range_value(range)?;
    let value = copy_parameterized(element, "AXStringForRange", &parameter)?;
    Some(value.downcast::<CFString>().ok()?.to_string())
}

fn bounds_for_range(element: &AXUIElement, range: CFRange) -> Option<CGRect> {
    let parameter = range_value(range)?;
    let value = copy_parameterized(element, "AXBoundsForRange", &parameter)?.downcast::<AXValue>().ok()?;
    let mut rect = CGRect::default();
    // SAFETY: the out pointer matches the requested type, CGRect.
    let ok = unsafe { value.value(AXValueType::CGRect, NonNull::from(&mut rect).cast::<c_void>()) };
    ok.then_some(rect)
}

fn range_value(range: CFRange) -> Option<CFRetained<AXValue>> {
    let pointer = NonNull::from(&range).cast::<c_void>();
    // SAFETY: `pointer` points to a CFRange, the type named; AXValueCreate copies it.
    unsafe { AXValue::new(AXValueType::CFRange, pointer) }
}

fn copy_parameterized(element: &AXUIElement, name: &str, parameter: &AXValue) -> Option<CFRetained<CFType>> {
    let name = CFString::from_str(name);
    let mut value: *const CFType = std::ptr::null();
    // SAFETY: `parameter` is an AXValue of the type the attribute takes; `value` is a valid out
    // pointer that receives an owned (+1) reference on success.
    let status = unsafe { element.copy_parameterized_attribute_value(&name, parameter, NonNull::from(&mut value)) };
    owned(status, value)
}

/// Wraps the owned result of an AX "Copy" call.
fn owned(status: AXError, value: *const CFType) -> Option<CFRetained<CFType>> {
    if status != AXError::Success {
        return None;
    }
    let value = NonNull::new(value.cast_mut())?;
    // SAFETY: AX Copy functions return a +1 reference, which CFRetained releases on drop.
    Some(unsafe { CFRetained::from_raw(value) })
}
