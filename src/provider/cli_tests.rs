use super::*;
use crate::provider::{Cancel, opencode, pi};
use crate::styles::Style;
use std::os::unix::fs::PermissionsExt;

/// A fake home and a stub CLI that records what it got into `<home>/record/`, then runs `body`.
struct Stub {
    _dir: WorkDir,
    home: PathBuf,
    program: PathBuf,
}

impl Stub {
    fn new(version: &str, body: &str) -> Stub {
        let dir = WorkDir::new().expect("dir");
        let home = dir.path().join("home");
        std::fs::create_dir_all(home.join("record")).expect("record");
        for data in [".pi/agent/sessions", ".local/share/opencode/log", ".local/state/opencode", ".config/opencode"] {
            std::fs::create_dir_all(home.join(data)).expect("data");
        }
        let program = dir.path().join("cli");
        let record = home.join("record");
        let script = format!(
            "#!/bin/sh\nif [ \"$1\" = --version ]; then echo 'tool v{version}'; exit 0; fi\n\
             printf '%s\\n' \"$@\" > '{r}/argv'\npwd > '{r}/cwd'\nenv > '{r}/env'\ncat > '{r}/stdin'\n{body}\n",
            r = record.display()
        );
        std::fs::write(&program, script).expect("write");
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        Stub { _dir: dir, home, program }
    }

    fn record(&self, name: &str) -> String {
        std::fs::read_to_string(self.home.join("record").join(name)).unwrap_or_default()
    }
}

const SECRET: &str = "the meeting are moved to tuesday";

#[test]
fn pi_runs_without_session_or_tools_and_gets_the_text_only_on_stdin() {
    let stub = Stub::new("1.5.2", "echo 'The meeting moved to Tuesday.'");
    let out =
        pi::rewrite(pi::VERIFIED, &stub.program, "ollama/qwen", &Style::Clear, SECRET, &Cancel::default(), &stub.home);
    assert_eq!(out, Outcome { answer: Ok("The meeting moved to Tuesday.".into()), wrote_files: None });
    let argv = stub.record("argv");
    for flag in ["-p", "--no-session", "--no-tools", "--no-extensions", "--no-skills", "--no-context-files"] {
        assert!(argv.lines().any(|line| line == flag), "{flag} missing: {argv}");
    }
    assert!(argv.ends_with("--model\nollama/qwen\n"), "{argv}");
    assert!(!argv.contains(SECRET), "the text must not be in argv");
    assert!(stub.record("stdin").contains(&format!("<text>\n{SECRET}\n</text>")));
    let cwd = PathBuf::from(stub.record("cwd").trim());
    assert!(cwd.file_name().is_some_and(|n| n.to_string_lossy().starts_with("retone-")), "{cwd:?}");
    assert!(!cwd.exists(), "the temp folder is removed after the run");
    assert!(stub.record("env").contains("NO_COLOR=1"));
}

#[test]
fn opencode_keeps_its_data_in_memory_and_in_the_temp_folder() {
    let stub = Stub::new("2.3.0", "echo 'Done.'");
    let out = opencode::rewrite(
        opencode::VERIFIED,
        &stub.program,
        "opencode/beam-free",
        &Style::Clear,
        SECRET,
        &Cancel::default(),
        &stub.home,
    );
    assert_eq!(out.answer, Ok("Done.".into()));
    let env = stub.record("env");
    // `pwd` resolves macOS's /var → /private/var link, so compare by the temp folder's name.
    let cwd = stub.record("cwd");
    let folder = Path::new(cwd.trim()).file_name().expect("name").to_string_lossy().into_owned();
    let var = |key: &str| env.lines().find_map(|line| line.strip_prefix(&format!("{key}="))).unwrap_or_default();
    assert!(env.contains("OPENCODE_DB=:memory:"));
    assert!(var("XDG_DATA_HOME").ends_with(&format!("{folder}/data")), "{}", var("XDG_DATA_HOME"));
    assert!(var("OPENCODE_CONFIG").ends_with(&format!("{folder}/opencode.json")));
    assert!(env.contains("OPENCODE_CONFIG_PROJECT_DISABLE=1"));
    assert_eq!(stub.record("argv"), "run\n--standalone\n--model\nopencode/beam-free\n");
}

#[test]
fn an_unverified_version_is_refused_before_any_request() {
    let stub = Stub::new("2.4.0", "echo never");
    let out = opencode::rewrite(
        opencode::VERIFIED,
        &stub.program,
        "opencode/x",
        &Style::Clear,
        SECRET,
        &Cancel::default(),
        &stub.home,
    );
    assert_eq!(
        out.answer,
        Err("OpenCode 2.4.0 is not verified for no-history use. Retone supports OpenCode 2.3.0. \
             Choose another provider or keep a verified version."
            .into())
    );
    assert_eq!(stub.record("stdin"), "", "the stub never got the text");
}

#[test]
fn a_cli_that_writes_into_its_data_folder_is_reported_and_nothing_is_deleted() {
    let stub = Stub::new("1.5.2", "echo session > \"$HOME/.pi/agent/sessions/new.jsonl\"; echo ok");
    // The stub reads $HOME from the real environment; point it at the fake home for this run.
    let program = stub.program.clone();
    let wrapper = stub.home.join("wrapper");
    std::fs::write(
        &wrapper,
        format!("#!/bin/sh\nHOME='{}' exec '{}' \"$@\"\n", stub.home.display(), program.display()),
    )
    .expect("wrapper");
    std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    let out = pi::rewrite(pi::VERIFIED, &wrapper, "m", &Style::Clear, SECRET, &Cancel::default(), &stub.home);
    assert_eq!(out.answer, Ok("ok".into()), "the result still shows");
    assert_eq!(out.wrote_files, Some(stub.home.join(".pi/agent/sessions")));
    assert!(stub.home.join(".pi/agent/sessions/new.jsonl").exists(), "Retone never deletes CLI files");
}

#[test]
fn cancel_kills_the_child_and_errors_are_redacted() {
    let stub = Stub::new("1.5.2", "sleep 30");
    let cancel = Cancel::default();
    let flag = cancel.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(200));
        flag.cancel();
    });
    let started = rune_core::time::now();
    let out = pi::rewrite(pi::VERIFIED, &stub.program, "m", &Style::Clear, SECRET, &cancel, &stub.home);
    assert_eq!(out.answer, Err("Cancelled.".into()));
    assert!(started.elapsed() < Duration::from_secs(5));

    let failing = Stub::new("1.5.2", &format!("echo 'Error: bad input {SECRET}' >&2; exit 1"));
    let out =
        pi::rewrite(pi::VERIFIED, &failing.program, "m", &Style::Clear, SECRET, &Cancel::default(), &failing.home);
    assert_eq!(out.answer, Err("Pi failed: bad input [selected text]".into()));
}

#[test]
fn model_ids_that_look_like_flags_are_refused() {
    let stub = Stub::new("1.5.2", "echo never");
    for model in ["", "--tools", "a b", "x\ny"] {
        let out =
            pi::rewrite(pi::VERIFIED, &stub.program, model, &Style::Clear, SECRET, &Cancel::default(), &stub.home);
        assert!(out.answer.is_err(), "{model:?}");
    }
    assert_eq!(stub.record("stdin"), "");
}

#[test]
fn versions_and_escape_codes_are_parsed() {
    assert_eq!(parse_version("opencode v2.3.0\n").as_deref(), Some("2.3.0"));
    assert_eq!(parse_version("1.5.2").as_deref(), Some("1.5.2"));
    assert_eq!(parse_version("version 2.3"), None);
    assert_eq!(strip_ansi("\u{1b}[91m\u{1b}[1mError: \u{1b}[0mlimit"), "Error: limit");
}
