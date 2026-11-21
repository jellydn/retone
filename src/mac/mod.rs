//! macOS-only native code. The only place in Retone where `unsafe` is allowed, each block with a
//! `// SAFETY:` comment, behind safe functions.

#[allow(unsafe_code)]
pub mod ax;
#[allow(unsafe_code)]
pub mod events;
#[allow(unsafe_code)]
pub mod pasteboard;
pub mod permissions;
pub mod screens;
#[allow(unsafe_code)]
pub mod status;
