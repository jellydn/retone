//! OpenCode adapter: `opencode run --standalone` with the session database in memory and a
//! private config whose only agent has every tool and permission denied.
//!
//! Spike S6 (OpenCode 2.3.0): `OPENCODE_DB=:memory:` keeps `opencode.db` unchanged, but every run
//! appends run metadata to `~/.local/share/opencode/log/opencode.log`, and `--log-level none`
//! does not stop it. With `XDG_DATA_HOME` in Retone's temp folder, 5 of 5 runs changed no files.

use super::cli::{self, Run, WorkDir};
use super::{Cancel, Outcome};
use crate::styles::{self, Style};
use std::ffi::OsString;
use std::path::Path;

/// Versions that passed `scripts/cli-history-check.sh`. Add one only after the check passes.
pub const VERIFIED: &[&str] = &["2.3.0"];

const CONFIG: &str = r#"{
  "$schema": "https://opencode.ai/config.json",
  "share": "disabled",
  "autoupdate": false,
  "snapshot": false,
  "default_agent": "retone",
  "agent": {
    "retone": {
      "mode": "primary",
      "description": "Rewrite text only",
      "prompt": "You rewrite text. Reply with the rewritten text only.",
      "tools": { "*": false },
      "permission": { "*": "deny" }
    }
  }
}"#;

/// Free models come from OpenCode's own provider and have their own limits and privacy terms.
pub fn is_free_model(model: &str) -> bool {
    model.starts_with("opencode/") && model.ends_with("-free")
}

pub fn rewrite(program: &Path, model: &str, style: &Style, text: &str, cancel: &Cancel, home: &Path) -> Outcome {
    let fail = |error: String| Outcome { answer: Err(error), wrote_files: None };
    if let Err(error) = cli::check_model("OpenCode", model) {
        return fail(error);
    }
    let Ok(workdir) = WorkDir::new() else { return fail("Retone could not create its temp folder.".into()) };
    let config = workdir.path().join("opencode.json");
    let data = workdir.path().join("data");
    if std::fs::write(&config, CONFIG).is_err() || std::fs::create_dir_all(data.join("opencode")).is_err() {
        return fail("Retone could not prepare OpenCode's private config.".into());
    }
    // Moving the data folder also hides OpenCode's login file; a copy in the temp folder keeps
    // connected providers working and is deleted with the folder.
    let auth = home.join(".local/share/opencode/auth.json");
    if auth.is_file() {
        std::fs::copy(&auth, data.join("opencode/auth.json")).ok();
    }
    let args: Vec<OsString> = ["run", "--standalone", "--model", model].iter().map(OsString::from).collect();
    let envs = vec![
        ("OPENCODE_DB", ":memory:".into()),
        ("XDG_DATA_HOME", data.into_os_string()),
        ("OPENCODE_CONFIG", config.into_os_string()),
        ("OPENCODE_CONFIG_DIR", workdir.path().as_os_str().to_owned()),
        ("OPENCODE_CONFIG_PROJECT_DISABLE", "1".into()),
        ("OPENCODE_DISABLE_PROJECT_CONFIG", "1".into()),
        ("OPENCODE_DISABLE_FILEWATCHER", "1".into()),
        ("OPENCODE_DISABLE_CLAUDE_CODE", "1".into()),
        ("OPENCODE_DISABLE_AUTOUPDATE", "1".into()),
    ];
    let data_dirs = [".local/share/opencode", ".local/state/opencode", ".cache/opencode", ".config/opencode"]
        .iter()
        .map(|dir| home.join(dir))
        .collect();
    let run = Run {
        name: "OpenCode",
        program,
        verified: VERIFIED,
        args,
        envs,
        data_dirs,
        stdin: styles::build_single(style, text),
    };
    cli::execute(run, &workdir, text, cancel, home)
}
