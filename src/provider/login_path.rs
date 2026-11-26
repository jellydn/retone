//! The `PATH` of the user's login shell. A Mac app opened from Finder gets launchd's short `PATH`
//! (`/usr/bin:/bin:/usr/sbin:/sbin`): no Homebrew, no `~/.local/bin`, no Node. The CLIs would not
//! be found, and a CLI that is a `#!/usr/bin/env node` script could not start.

use std::ffi::{OsStr, OsString};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::thread::sleep;
use std::time::Duration;

/// Comes before the value, so text that a shell startup file prints is skipped.
const MARKER: &str = "__RETONE_PATH__=";
const SHELL_DEADLINE: Duration = Duration::from_secs(5);

/// Where CLI installers put their programs, under the home folder or as absolute paths. Searched
/// after the shell's `PATH`, because a shell can add them only in interactive sessions (fish's
/// `status is-interactive` block, `~/.zshrc`), which `-l -c` does not run.
const INSTALL_DIRS: &[&str] =
    &[".bun/bin", ".local/bin", ".npm-global/bin", "Library/pnpm", ".volta/bin", "/opt/homebrew/bin", "/usr/local/bin"];

/// Read once per run, on the first CLI request (a worker thread, never the frame loop): the
/// login shell's `PATH` (Retone's own when the shell fails or is too slow), then the install
/// folders that exist.
pub fn login_path() -> Option<&'static OsStr> {
    static PATH: OnceLock<Option<OsString>> = OnceLock::new();
    PATH.get_or_init(|| {
        let shell = read().or_else(|| std::env::var_os("PATH"));
        let home = rune_core::env::var("HOME").map(PathBuf::from);
        let installs = INSTALL_DIRS.iter().filter_map(|dir| match home.as_ref() {
            _ if dir.starts_with('/') => Some(PathBuf::from(dir)),
            Some(home) => Some(home.join(dir)),
            None => None,
        });
        let mut dirs: Vec<PathBuf> = shell.iter().flat_map(std::env::split_paths).collect();
        for dir in installs.filter(|dir| dir.is_dir()) {
            if !dirs.contains(&dir) {
                dirs.push(dir);
            }
        }
        std::env::join_paths(dirs).ok().filter(|path| !path.is_empty())
    })
    .as_deref()
}

fn read() -> Option<OsString> {
    let shell = rune_core::env::var("SHELL").filter(|shell| shell.starts_with('/')).unwrap_or("/bin/zsh".into());
    // Works in zsh, bash and fish: fish joins a quoted `$PATH` with colons too.
    let script = format!("printf '{MARKER}%s' \"$PATH\"");
    let mut child = Command::new(shell)
        .args(["-l", "-c", &script])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let started = rune_core::time::now();
    while child.try_wait().ok()?.is_none() {
        if started.elapsed() > SHELL_DEADLINE {
            child.kill().ok();
            child.wait().ok();
            return None;
        }
        sleep(Duration::from_millis(20));
    }
    let mut out = String::new();
    std::io::Read::read_to_string(&mut child.stdout.take()?, &mut out).ok()?;
    let path = out.rsplit_once(MARKER)?.1.trim();
    (!path.is_empty()).then(|| path.into())
}

/// The program to start: a path as given (the Settings override, or a test stub), or a bare
/// name found in `path`, as `command -v` finds it. A name that is not found stays bare, so the
/// spawn fails with "not found".
pub fn locate(program: &Path, path: Option<&OsStr>) -> PathBuf {
    let bare = program.components().count() == 1;
    let found = path.filter(|_| bare).and_then(|path| {
        std::env::split_paths(path).map(|dir| dir.join(program)).find(|candidate| is_executable(candidate))
    });
    found.unwrap_or_else(|| program.to_path_buf())
}

/// The `PATH` the CLI runs with: the program's own folder first (Node installs keep `node` next
/// to the CLI), then the login shell's. None keeps Retone's own.
pub fn child_path(program: &Path, path: Option<&OsStr>) -> Option<OsString> {
    let path = path?;
    let own = program.parent().filter(|dir| dir.is_absolute()).map(Path::to_path_buf);
    std::env::join_paths(own.into_iter().chain(std::env::split_paths(path))).ok()
}

fn is_executable(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
}
