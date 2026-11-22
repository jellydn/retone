//! Codex adapter: `codex exec --ephemeral` in a read-only sandbox, without the user's config or
//! rules, and with the shell, memories, plugins, apps and browser turned off.
//!
//! `--ignore-user-config` skips `config.toml` (its MCP servers and its `notify` program).
//! `memories` is on by default and could keep text on disk.
//!
//! Codex writes its state on every run, even with `--ephemeral`: `installation_id`, SQLite
//! databases (threads, logs, memories, goals, queue) and its bundled skills. So it runs with
//! `CODEX_HOME` in Retone's private temp folder, deleted after the run, with only a copy of the
//! user's `auth.json` for the login. The user's Codex home is never written.

use super::cli::{self, Run, WorkDir};
use super::{Cancel, Outcome};
use crate::styles::{self, Style};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

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
    // The user's Codex home: the same rule as the release gate's ${CODEX_HOME:-$HOME/.codex}.
    let user_home = rune_core::env::var("CODEX_HOME")
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".codex"));
    let private = workdir.path().join("codex");
    if std::fs::create_dir(&private).is_err() {
        return fail(t!("provider.temp_folder").into());
    }
    let auth = user_home.join("auth.json");
    if auth.is_file() {
        std::fs::copy(&auth, private.join("auth.json")).ok();
    }
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
        // CODEX_SQLITE_HOME would move the databases out again, so it points there too.
        envs: vec![("CODEX_HOME", private.clone().into_os_string()), ("CODEX_SQLITE_HOME", private.into_os_string())],
        // Nothing should change in the user's Codex home now; the guard watches its top level and
        // every sessions/YYYY/MM/DD folder. The release gate checks all of it, and is the proof.
        data_dirs: [user_home.clone(), user_home.join("sessions"), user_home.join("memories")]
            .into_iter()
            .chain(session_folders(&user_home.join("sessions")))
            .collect(),
        stdin: styles::build_single(style, text),
    };
    cli::execute(run, &workdir, text, cancel, home)
}

/// Every `YYYY`, `YYYY/MM` and `YYYY/MM/DD` folder under `sessions`. The guard reads one level below
/// each folder, so a new file in an existing day folder shows only if that folder is watched.
fn session_folders(sessions: &Path) -> Vec<PathBuf> {
    let subfolders = |dir: &Path| -> Vec<PathBuf> {
        std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect()
    };
    let mut level = vec![sessions.to_path_buf()];
    let mut all = Vec::new();
    for _ in 0..3 {
        level = level.iter().flat_map(|dir| subfolders(dir)).collect();
        all.extend(level.iter().cloned());
    }
    all
}
