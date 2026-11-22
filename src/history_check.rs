//! `retone --cli-check`: runs rewrites of a fixed sample through the real Pi or OpenCode adapter,
//! so `scripts/cli-history-check.sh` can prove that the installed CLI version writes no history.
//!
//! The installed version counts as verified for this run only; that is what the check tests. The
//! script compares a full snapshot of the CLI's data folders before and after, because the app's
//! runtime guard looks only one level deep.
//!
//! ```text
//! retone --cli-check --provider pi --model ollama/qwen2.5:3b [--runs 5]
//! ```

#![forbid(unsafe_code)]

use crate::eval::flag;
use crate::provider::{Cancel, Env, Kind, Outcome, cli, opencode, pi};
use crate::styles::Style;
use std::path::Path;
use std::process::Command;

/// Sample text only: the check never sends user text.
const SAMPLE: &str = "hey so the deploy broke because the config had the wrong port, i fixed it now";

/// Runs the check when the arguments ask for it; returns the exit code, or None for a normal start.
pub fn run_from_args() -> Option<i32> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.iter().any(|arg| arg == "--cli-check") {
        return None;
    }
    Some(match run(&args) {
        Ok(true) => 0,
        Ok(false) => 1,
        Err(error) => {
            eprintln!("retone --cli-check: {error}");
            2
        }
    })
}

fn run(args: &[String]) -> Result<bool, String> {
    let model = flag(args, "--model").ok_or("--model is required")?;
    let runs: usize = flag(args, "--runs").unwrap_or("5").parse().map_err(|_| "--runs must be a number")?;
    let env = Env::system().ok_or("HOME is not set")?;
    let (kind, program) = match flag(args, "--provider").ok_or("--provider is required")? {
        "pi" => (Kind::Pi, &env.pi_program),
        "opencode" => (Kind::OpenCode, &env.opencode_program),
        other => return Err(format!("unknown provider {other} (use pi or opencode)")),
    };
    let name = kind.name();
    let version = installed_version(program).ok_or_else(|| format!("cannot read the {name} version"))?;
    let verified = [version.as_str()];
    println!("{name} {version} · {model} · {runs} runs");

    let mut passed = true;
    for i in 1..=runs {
        let rewrite = if kind == Kind::Pi { pi::rewrite } else { opencode::rewrite };
        let Outcome { answer, wrote_files } =
            rewrite(&verified, program, model, &Style::Clear, SAMPLE, &Cancel::default(), &env.home);
        // A failed rewrite did not run the whole path, so it cannot count as a pass.
        match &answer {
            Ok(text) => println!("run {i}/{runs}: ok ({} chars)", text.chars().count()),
            Err(error) => println!("run {i}/{runs}: FAILED: {error}"),
        }
        if let Some(dir) = &wrote_files {
            println!("run {i}/{runs}: runtime guard saw a change in {}", dir.display());
        }
        passed &= answer.is_ok() && wrote_files.is_none();
    }
    println!("version={version}");
    Ok(passed)
}

/// Asked with a temp `HOME`: OpenCode logs even `--version` under `~/.local/share`, and this
/// probe must not change the folders the script compares. The adapter's own probe uses its envs.
fn installed_version(program: &Path) -> Option<String> {
    let home = cli::WorkDir::new().ok()?;
    let output = Command::new(program).arg("--version").env("HOME", home.path()).env("NO_COLOR", "1").output().ok()?;
    cli::parse_version(&String::from_utf8_lossy(&output.stdout))
}
