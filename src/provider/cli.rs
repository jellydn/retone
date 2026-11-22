//! Shared runner for the CLI adapters (Pi, OpenCode).
//!
//! Each request: a private `0700` temp folder as the working directory (deleted afterwards), the
//! prompt on stdin (argv shows in `ps`), only verified CLI versions, a deadline, cancel that kills
//! the whole process group, and a cheap guard that compares the CLI's data folders before and
//! after the run. Retone never deletes the CLI's own files.

use super::{Cancel, DEADLINE, Outcome, answer_cap, redact};
use crate::styles;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io::Read;
use std::os::unix::fs::DirBuilderExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread::{self, JoinHandle, sleep};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const VERSION_DEADLINE: Duration = Duration::from_secs(5);
/// Most stderr bytes kept for the error message.
const STDERR_LIMIT: usize = 16 * 1024;

/// Retone's own scratch folder for one request, removed on drop (success, failure or cancel).
pub struct WorkDir(PathBuf);

impl WorkDir {
    pub fn new() -> std::io::Result<Self> {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = rune_core::time::system_now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos());
        loop {
            let name = format!("retone-{}-{nanos}-{}", std::process::id(), COUNTER.fetch_add(1, Ordering::Relaxed));
            let path = std::env::temp_dir().join(name);
            // `create` fails if the path exists, so nobody can plant the folder first.
            match std::fs::DirBuilder::new().mode(0o700).create(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for WorkDir {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

pub struct Run<'a> {
    /// Shown in messages: "Pi", "OpenCode".
    pub name: &'static str,
    pub program: &'a Path,
    pub verified: &'a [&'a str],
    pub args: Vec<OsString>,
    pub envs: Vec<(&'static str, OsString)>,
    pub data_dirs: Vec<PathBuf>,
    pub stdin: String,
}

/// Refuses model IDs that could be read as a flag or hide another argument.
pub fn check_model(name: &str, model: &str) -> Result<(), String> {
    if model.is_empty() || model.starts_with('-') || model.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(format!("Choose an exact {name} model ID in Settings."));
    }
    Ok(())
}

/// Runs the CLI once. `text` is the selection, used only to redact errors.
pub fn execute(run: Run, workdir: &WorkDir, text: &str, cancel: &Cancel, home: &Path) -> Outcome {
    // Taken before `--version` too, so a CLI that writes even when asked its version is caught.
    let before = snapshot(&run.data_dirs);
    let answer = version_then_rewrite(&run, workdir, text, cancel);
    let wrote_files = changed_dir(&before, &snapshot(&run.data_dirs));
    let answer = answer.map_err(|error| redact(&error, text, home.to_str()));
    Outcome { answer, wrote_files }
}

fn version_then_rewrite(run: &Run, workdir: &WorkDir, text: &str, cancel: &Cancel) -> Result<String, String> {
    let version_args = vec![OsString::from("--version")];
    let not_verified = |found: &str| {
        format!(
            "{} {found} is not verified for no-history use. Retone supports {} {}. Choose another provider or keep a verified version.",
            run.name,
            run.name,
            run.verified.join(", ")
        )
    };
    let version = match spawn_and_wait(run, workdir, &version_args, "", cancel, VERSION_DEADLINE, usize::MAX) {
        Ok(output) => parse_version(&output).ok_or_else(|| not_verified("(unknown version)"))?,
        Err(Failure::Cancelled) => return Err("Cancelled.".into()),
        Err(_) => return Err(format!("Cannot run {}. Is it installed and on your PATH?", run.name)),
    };
    if !run.verified.contains(&version.as_str()) {
        return Err(not_verified(&version));
    }
    let limit = answer_cap(text) * 4 + 1024;
    match spawn_and_wait(run, workdir, &run.args, &run.stdin, cancel, DEADLINE, limit) {
        Ok(output) => {
            let answer = styles::clean_answer(&strip_ansi(&output));
            if answer.is_empty() {
                Err(format!("{} returned no text.", run.name))
            } else if answer.chars().count() > answer_cap(text) {
                Err("The model's answer was far longer than the text, so Retone dropped it.".into())
            } else {
                Ok(answer)
            }
        }
        Err(Failure::Cancelled) => Err("Cancelled.".into()),
        Err(Failure::TimedOut) => Err(format!("No answer within {} s.", DEADLINE.as_secs())),
        Err(Failure::Exited(stderr)) => Err(format!("{} failed: {}", run.name, last_error_line(&stderr))),
        Err(Failure::Spawn) => Err(format!("Cannot run {}. Is it installed and on your PATH?", run.name)),
    }
}

/// The installed version, asked with `HOME` set to a temp folder and the usual deadline. For
/// `retone --cli-check`, whose probe must not touch the real data folders (OpenCode logs even
/// `--version` under `~/.local/share`).
pub fn installed_version(program: &Path) -> Option<String> {
    let workdir = WorkDir::new().ok()?;
    let run = Run {
        name: "",
        program,
        verified: &[],
        args: vec![],
        envs: vec![("HOME", workdir.path().as_os_str().to_owned())],
        data_dirs: vec![],
        stdin: String::new(),
    };
    let args = [OsString::from("--version")];
    let output = spawn_and_wait(&run, &workdir, &args, "", &Cancel::default(), VERSION_DEADLINE, usize::MAX).ok()?;
    parse_version(&output)
}

enum Failure {
    Spawn,
    Cancelled,
    TimedOut,
    Exited(String),
}

fn spawn_and_wait(
    run: &Run,
    workdir: &WorkDir,
    args: &[OsString],
    stdin: &str,
    cancel: &Cancel,
    deadline: Duration,
    stdout_limit: usize,
) -> Result<String, Failure> {
    let mut command = Command::new(run.program);
    command
        .args(args)
        .current_dir(workdir.path())
        .env("NO_COLOR", "1")
        .envs(run.envs.iter().map(|(key, value)| (key, value)))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Own process group, so cancel also stops helpers the CLI starts (OpenCode's server).
        .process_group(0);
    let mut child = command.spawn().map_err(|_| Failure::Spawn)?;
    let stdout = reader(child.stdout.take(), stdout_limit);
    let stderr = reader(child.stderr.take(), STDERR_LIMIT);
    if let Some(mut pipe) = child.stdin.take() {
        let input = stdin.to_string();
        // A writer thread, so a CLI that does not read stdin cannot block Retone.
        thread::spawn(move || std::io::Write::write_all(&mut pipe, input.as_bytes()).ok());
    }
    let started = rune_core::time::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if cancel.is_cancelled() => break None,
            Ok(None) if started.elapsed() > deadline => {
                kill_group(&mut child);
                return Err(Failure::TimedOut);
            }
            Ok(None) => sleep(Duration::from_millis(20)),
            Err(_) => break None,
        }
    };
    kill_group(&mut child);
    let Some(status) = status else { return Err(Failure::Cancelled) };
    let out = stdout.join().unwrap_or_default();
    let err = stderr.join().unwrap_or_default();
    if status.success() { Ok(out) } else { Err(Failure::Exited(err)) }
}

/// Reads a pipe to the end, keeping at most `limit` bytes; the rest is drained so the child
/// never blocks on a full pipe.
fn reader(pipe: Option<impl Read + Send + 'static>, limit: usize) -> JoinHandle<String> {
    thread::spawn(move || {
        let Some(mut pipe) = pipe else { return String::new() };
        let mut kept = Vec::new();
        let mut chunk = [0u8; 8192];
        while let Ok(read) = pipe.read(&mut chunk) {
            if read == 0 {
                break;
            }
            let room = limit.saturating_sub(kept.len());
            kept.extend_from_slice(&chunk[..read.min(room)]);
        }
        String::from_utf8_lossy(&kept).into_owned()
    })
}

/// Stops the child and everything in its process group, and reaps the child.
fn kill_group(child: &mut Child) {
    let group = format!("-{}", child.id());
    Command::new("/bin/kill").args(["-KILL", "--", &group]).stderr(Stdio::null()).status().ok();
    child.kill().ok();
    child.wait().ok();
}

/// The first `x.y.z` in `--version` output, such as "opencode v2.3.0" → "2.3.0".
pub fn parse_version(output: &str) -> Option<String> {
    output.split_whitespace().map(|word| word.trim_start_matches('v')).find_map(|word| {
        let parts: Vec<&str> = word.split('.').collect();
        let numeric = parts.len() == 3 && parts.iter().all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()));
        numeric.then(|| word.to_string())
    })
}

/// Removes terminal escape sequences (`ESC [ … letter`) that some CLIs print despite `NO_COLOR`.
pub fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\u{1b}' {
            out.push(c);
            continue;
        }
        if chars.peek() == Some(&'[') {
            chars.next();
            for c in chars.by_ref() {
                if c.is_ascii_alphabetic() {
                    break;
                }
            }
        }
    }
    out
}

/// The line that names the problem: the last one with "error", else the last non-empty one.
fn last_error_line(stderr: &str) -> String {
    let clean = strip_ansi(stderr);
    let lines: Vec<&str> = clean.lines().map(str::trim).filter(|line| !line.is_empty()).collect();
    let line = lines.iter().rev().find(|line| line.to_ascii_lowercase().contains("error")).or(lines.last());
    line.map_or("no details".into(), |line| line.trim_start_matches("Error:").trim().to_string())
}

type Snapshot = BTreeMap<PathBuf, Option<(u64, SystemTime)>>;

/// Size and modification time of each data folder and its top-level entries. A new file inside
/// a subfolder changes that subfolder's time, so one level covers new sessions cheaply.
fn snapshot(dirs: &[PathBuf]) -> Snapshot {
    let mut entries = Snapshot::new();
    let stamp = |path: &Path| std::fs::symlink_metadata(path).ok().and_then(|m| Some((m.len(), m.modified().ok()?)));
    for dir in dirs {
        entries.insert(dir.clone(), stamp(dir));
        for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            entries.insert(entry.path(), stamp(&entry.path()));
        }
    }
    entries
}

/// The data folder in which something changed, if any.
fn changed_dir(before: &Snapshot, after: &Snapshot) -> Option<PathBuf> {
    let changed = after.iter().find(|(path, stamp)| before.get(*path) != Some(stamp)).map(|(path, _)| path);
    let removed = before.keys().find(|path| !after.contains_key(*path));
    let path = changed.or(removed)?;
    Some(after.keys().chain(before.keys()).find(|dir| path.starts_with(dir)).unwrap_or(path).clone())
}

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;
