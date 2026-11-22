#!/bin/sh
# Release gate for the CLI providers: proves that the installed Pi or OpenCode version writes no
# history when Retone runs it. Run it on a real Mac before each release and whenever a supported
# CLI version changes. Add a version to VERIFIED in src/provider/<cli>.rs only after it passes.
#
#   sh scripts/cli-history-check.sh pi ollama/qwen2.5:3b [runs]
#   sh scripts/cli-history-check.sh opencode opencode/<model>-free [runs]
#
# It records the path, size and modification time of every file under the CLI's data folders,
# runs the rewrites through Retone's own adapter code (`retone --cli-check`), records again, and
# fails on any new, changed or removed file. It never deletes or edits the CLI's files.
# Do not use the CLI in another window while the check runs: that also changes these folders.
set -eu

usage() {
	echo "usage: sh scripts/cli-history-check.sh pi|opencode MODEL [RUNS]" >&2
	exit 2
}

[ $# -ge 2 ] || usage
cli=$1
model=$2
runs=${3:-5}

# The same folders as the plan's release gate. Pi: all of ~/.pi/agent, wider than the app's
# runtime guard (sessions only), so settings or caches written by a new version also show.
case $cli in
pi) dirs="$HOME/.pi/agent" source_file=src/provider/pi.rs ;;
opencode)
	dirs="$HOME/.local/share/opencode $HOME/.local/state/opencode $HOME/.cache/opencode $HOME/.config/opencode"
	source_file=src/provider/opencode.rs
	;;
*) usage ;;
esac

cd "$(dirname "$0")/.."
cargo build --quiet
retone=target/debug/retone

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# Files: modification time, size, path (BSD stat), at every level, unlike the runtime guard.
# Folders: path only. A folder's time also changes when an entry is made and deleted again; Pi
# does that on every start (even `pi --version`, which gets no text), and nothing stays on disk.
# Folder times go to a second list that only prints a note.
snapshot() {
	for dir in $dirs; do
		if [ -e "$dir" ]; then
			find "$dir" ! -type d -exec stat -f '%m %z %N' {} +
			find "$dir" -type d -exec stat -f 'folder %N' {} +
		else
			echo "missing $dir"
		fi
	done | LC_ALL=C sort
}

folder_times() {
	for dir in $dirs; do
		[ -e "$dir" ] && find "$dir" -type d -exec stat -f '%m %N' {} +
	done | LC_ALL=C sort
}

snapshot >"$work/before"
folder_times >"$work/folders-before"
# A pipe hides the exit code in POSIX sh, so the code is printed as a line and read back.
{ "$retone" --cli-check --provider "$cli" --model "$model" --runs "$runs" 2>&1 && echo "exit=0" || echo "exit=$?"; } |
	tee "$work/run"
snapshot >"$work/after"
folder_times >"$work/folders-after"

status=$(sed -n 's/^exit=//p' "$work/run")
version=$(sed -n 's/^version=//p' "$work/run")
if ! diff -u "$work/before" "$work/after" >"$work/diff"; then
	echo
	echo "FAIL: $cli ${version:-?} changed files in its data folders:"
	grep -E '^[-+][^-+]' "$work/diff" || true
	echo "Nothing was deleted. Do not add this version to VERIFIED."
	exit 1
fi
if [ "$status" -ne 0 ]; then
	echo
	echo "FAIL: not every rewrite finished, so the check is not complete (exit $status)."
	exit 1
fi
echo
echo "PASS: $cli $version left no new, changed or removed file in: $dirs"
if ! diff -q "$work/folders-before" "$work/folders-after" >/dev/null; then
	echo "Note: these folder times changed, with no file left behind (a short-lived lock or temp file):"
	diff "$work/folders-before" "$work/folders-after" | sed -n 's/^> [0-9]* /  /p'
fi
if ! grep -q "\"$version\"" "$source_file"; then
	echo "Add \"$version\" to VERIFIED in $source_file."
fi
