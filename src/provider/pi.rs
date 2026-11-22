//! Pi adapter: `pi -p --no-session` with every tool, extension and context file turned off.
//! Spike S6: Pi 1.5.2 wrote nothing under `~/.pi/agent` in 5 of 5 runs.

use super::cli::{self, Run, WorkDir};
use super::{Cancel, Outcome};
use crate::styles::{self, Style};
use std::ffi::OsString;
use std::path::Path;

/// Versions that passed `scripts/cli-history-check.sh`. Add one only after the check passes.
pub const VERIFIED: &[&str] = &["1.5.2"];

/// `verified` is [`VERIFIED`] in the app; only `retone --cli-check` passes the installed version, to test it.
pub fn rewrite(
    verified: &[&str],
    program: &Path,
    model: &str,
    style: &Style,
    text: &str,
    cancel: &Cancel,
    home: &Path,
) -> Outcome {
    let fail = |error: String| Outcome { answer: Err(error), wrote_files: None };
    if let Err(error) = cli::check_model("Pi", model) {
        return fail(error);
    }
    let Ok(workdir) = WorkDir::new() else { return fail("Retone could not create its temp folder.".into()) };
    let args = [
        "-p",
        "--no-session",
        "--no-tools",
        "--no-extensions",
        "--no-skills",
        "--no-prompt-templates",
        "--no-context-files",
        // Rewrites need no reasoning, and models without thinking refuse a thinking level.
        "--thinking",
        "off",
        "--model",
    ];
    let mut args: Vec<OsString> = args.iter().map(OsString::from).collect();
    args.push(model.into());
    let run = Run {
        name: "Pi",
        program,
        verified,
        args,
        envs: vec![("PI_OFFLINE", "1".into())],
        data_dirs: vec![home.join(".pi/agent/sessions")],
        stdin: styles::build_single(style, text),
    };
    cli::execute(run, &workdir, text, cancel, home)
}
