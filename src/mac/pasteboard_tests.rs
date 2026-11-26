use super::*;
use std::sync::{Mutex, MutexGuard};

/// Pasteboard tests run one at a time. In parallel, two `pasteboardWithUniqueName` boards on
/// different threads sometimes saw each other's items. The app uses the pasteboard from one
/// worker thread only, so serial tests match it. `capture` tests use it too.
pub(crate) fn serial() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    // A failed test poisons the lock; the next test still runs.
    LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn mixed_snapshot() -> Snapshot {
    // Text, RTF, PNG-like bytes and a file URL in two items, as a Finder copy plus text would give.
    Snapshot {
        items: vec![
            vec![
                (PLAIN_TEXT_TYPE.into(), "héllo\nworld".as_bytes().to_vec()),
                ("public.rtf".into(), b"{\\rtf1\\ansi hello}".to_vec()),
            ],
            vec![
                ("public.png".into(), vec![0x89, b'P', b'N', b'G', 0, 1, 2, 255]),
                ("public.file-url".into(), b"file:///tmp/a%20b.txt".to_vec()),
            ],
        ],
        change_count: 0,
        complete: true,
    }
}

fn without_change_count(mut snapshot: Snapshot) -> Snapshot {
    snapshot.change_count = 0;
    snapshot
}

#[test]
fn restore_gives_back_every_item_type_and_byte() {
    let _serial = serial();
    let board = Board::unique();
    let original = mixed_snapshot();
    board.restore(&original);
    assert_eq!(without_change_count(board.snapshot()), original);
}

#[test]
fn restore_is_skipped_when_someone_copied_after_retone() {
    let _serial = serial();
    let board = Board::unique();
    let original = mixed_snapshot();
    board.restore(&original);
    let retone_write = board.write_transient_text("rewrite");
    // The user copies something new before Retone restores.
    board.write_transient_text("user copy");
    assert!(!board.restore_if_unchanged(&original, retone_write));
    assert_eq!(board.text().as_deref(), Some("user copy"));
}

#[test]
fn restore_runs_when_only_retone_wrote() {
    let _serial = serial();
    let board = Board::unique();
    let original = mixed_snapshot();
    board.restore(&original);
    let retone_write = board.write_transient_text("rewrite");
    assert_eq!(board.text().as_deref(), Some("rewrite"));
    assert!(board.restore_if_unchanged(&original, retone_write));
    assert_eq!(without_change_count(board.snapshot()), original);
}

#[test]
fn transient_text_carries_the_clipboard_manager_markers() {
    let _serial = serial();
    let board = Board::unique();
    board.write_transient_text("rewrite");
    let types: Vec<String> = board.snapshot().items[0].iter().map(|(kind, _)| kind.clone()).collect();
    assert!(types.contains(&TRANSIENT_TYPE.to_string()));
    assert!(types.contains(&AUTO_GENERATED_TYPE.to_string()));
}

#[test]
fn an_empty_pasteboard_restores_to_empty() {
    let _serial = serial();
    let board = Board::unique();
    let empty = board.snapshot();
    board.write_transient_text("rewrite");
    board.restore(&empty);
    assert!(board.snapshot().items.is_empty());
}
