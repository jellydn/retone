//! Codex adapter: `codex exec --ephemeral` in a read-only sandbox, without the user's config or
//! rules, and with the shell, memories, plugins, apps and browser turned off.
//!
//! `--ignore-user-config` skips `config.toml` (its MCP servers and its `notify` program); the
//! login still comes from `CODEX_HOME`. `memories` is on by default and could keep text on disk.

use super::cli::{self, Run, WorkDir};
use super::{Cancel, Outcome};
use crate::styles::{self, Style};
use std::ffi::OsString;
use std::path::Path;

/// Versions that passed `scripts/cli-history-check.sh`. Add one only after the check passes.
pub const VERIFIED: &[&str] = &[];

const DISABLED_FEATURES: [&str; 11] = [
    "shell_tool",
    "shell_snapshot",
    "memories",
    "apps",
    "plugins",
    "browser_use",
    "in_app_browser",
    "image_generation",
    "view_image",
    "tool_suggest",
    "skill_search",
];

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
    if let Err(error) = cli::check_model("Codex", model) {
        return fail(error);
    }
    let Ok(workdir) = WorkDir::new() else { return fail(t!("provider.temp_folder").into()) };
    // The child inherits CODEX_HOME and writes there; the same rule as the release gate's
    // ${CODEX_HOME:-$HOME/.codex}.
    let codex_home = rune_core::env::var("CODEX_HOME")
        .filter(|dir| !dir.is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| home.join(".codex"));
    let mut args: Vec<OsString> = [
        "exec",
        "--ephemeral",
        "--skip-git-repo-check",
        "--ignore-user-config",
        "--ignore-rules",
        "--sandbox",
        "read-only",
        "--color",
        "never",
    ]
    .iter()
    .map(OsString::from)
    .collect();
    for feature in DISABLED_FEATURES {
        args.extend(["--disable".into(), feature.into()]);
    }
    // `-` reads the prompt from stdin; it must come last.
    args.extend(["--model".into(), model.into(), "-".into()]);
    let run = Run {
        name: "Codex",
        program,
        verified,
        args,
        envs: vec![],
        // Sessions go to sessions/YYYY/MM/DD/, so this cheap guard sees only a new year folder;
        // the release gate checks the whole Codex home, and is the proof.
        data_dirs: vec![codex_home.join("sessions"), codex_home.join("memories")],
        stdin: styles::build_single(style, text),
    };
    cli::execute(run, &workdir, text, cancel, home)
}
