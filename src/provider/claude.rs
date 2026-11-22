//! Claude Code adapter: `claude -p --no-session-persistence` with every tool, MCP server, hook and
//! skill turned off, and prompt history skipped.
//!
//! User settings stay loaded: many users set the API address and token there
//! (`ANTHROPIC_BASE_URL`), and without them Claude Code cannot connect. Their hooks are user
//! scripts that can write files, so `disableAllHooks` turns them off; project and local settings
//! are not loaded, and the temp folder has none.

use super::cli::{self, Run, WorkDir};
use super::{Cancel, Outcome};
use crate::styles::{self, Style};
use std::ffi::OsString;
use std::path::Path;

/// Versions that passed `scripts/cli-history-check.sh`. Add one only after the check passes.
pub const VERIFIED: &[&str] = &[];

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
    if let Err(error) = cli::check_model("Claude Code", model) {
        return fail(error);
    }
    let Ok(workdir) = WorkDir::new() else { return fail(t!("provider.temp_folder").into()) };
    let args = [
        "-p",
        "--no-session-persistence",
        // An empty list turns every built-in tool off.
        "--tools",
        "",
        "--setting-sources",
        "user",
        "--settings",
        r#"{"disableAllHooks":true}"#,
        "--strict-mcp-config",
        "--disable-slash-commands",
        "--model",
    ];
    let mut args: Vec<OsString> = args.iter().map(OsString::from).collect();
    args.push(model.into());
    let run = Run {
        name: "Claude Code",
        program,
        verified,
        args,
        envs: vec![("CLAUDE_CODE_SKIP_PROMPT_HISTORY", "1".into()), ("DISABLE_AUTOUPDATER", "1".into())],
        // Sessions go to projects/<folder>/; a new folder there changes its time. The release
        // gate checks all of ~/.claude and ~/.claude.json.
        data_dirs: vec![home.join(".claude/projects")],
        stdin: styles::build_single(style, text),
    };
    cli::execute(run, &workdir, text, cancel, home)
}
