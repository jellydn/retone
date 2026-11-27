//! Claude Code adapter: `claude -p --no-session-persistence` with every tool, MCP server, hook and
//! skill turned off, and prompt history skipped.
//!
//! User settings stay loaded: many users set the API address and token there
//! (`ANTHROPIC_BASE_URL`), and without them Claude Code cannot connect. Their hooks are user
//! scripts that can write files, so `disableAllHooks` turns them off; project and local settings
//! are not loaded, and the temp folder has none.

use super::cli::{self, Run, WorkDir};
use super::{Cancel, ErrorKind, Failure, Outcome};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Versions that passed `scripts/cli-history-check.sh`. Add one only after the check passes.
pub const VERIFIED: &[&str] = &["2.1.66"];

/// `verified` is [`VERIFIED`] in the app; only `retone --cli-check` passes the installed version, to test it.
/// `prompt` goes on stdin; `text` is the selection, for the answer cap and redaction.
pub fn rewrite(
    verified: &[&str],
    program: &Path,
    model: &str,
    prompt: String,
    text: &str,
    cancel: &Cancel,
    home: &Path,
) -> Outcome {
    let fail = |error: Failure| Outcome { answer: Err(error), wrote_files: None };
    if let Err(error) = cli::check_model("Claude Code", model) {
        return fail(Failure::new(ErrorKind::Setup, error));
    }
    let config = match config_dir(rune_core::env::var("CLAUDE_CONFIG_DIR"), home) {
        Ok(config) => config,
        Err(error) => return fail(Failure::new(ErrorKind::Setup, error)),
    };
    let Ok(workdir) = WorkDir::new() else { return fail(Failure::new(ErrorKind::Other, t!("provider.temp_folder"))) };
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
        // gate checks all of the config folder and ~/.claude.json.
        data_dirs: vec![config.join("projects")],
        stdin: prompt,
    };
    cli::execute(run, &workdir, text, cancel, home)
}

/// Where Claude Code keeps its settings and sessions: `CLAUDE_CONFIG_DIR` (from `var`), else
/// `~/.claude`. A relative value is refused: Claude runs in another folder than Retone and the
/// release gate, so each would resolve it to a different place.
fn config_dir(var: Option<String>, home: &Path) -> Result<PathBuf, String> {
    match var.filter(|dir| !dir.is_empty()).map(PathBuf::from) {
        None => Ok(home.join(".claude")),
        Some(dir) if dir.is_absolute() => Ok(dir),
        Some(_) => Err(t!("claude.relative_config_dir").into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_config_dir_follows_the_variable_and_refuses_a_relative_one() {
        let home = Path::new("/Users/me");
        assert_eq!(config_dir(None, home), Ok(home.join(".claude")));
        assert_eq!(config_dir(Some(String::new()), home), Ok(home.join(".claude")));
        assert_eq!(config_dir(Some("/w/claude".into()), home), Ok(PathBuf::from("/w/claude")));
        assert!(config_dir(Some("claude-work".into()), home).is_err());
        assert!(config_dir(Some("~/claude".into()), home).is_err());
    }
}
