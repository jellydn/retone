#!/bin/sh
# Release gate for the CLI providers: proves that the installed CLI version writes no
# history when Retone runs it. Run it on a real Mac before each release and whenever a supported
# CLI version changes. Add a version to VERIFIED in src/provider/<cli>.rs only after it passes.
#
#   sh scripts/cli-history-check.sh pi ollama/qwen2.5:3b [runs]
#   sh scripts/cli-history-check.sh opencode opencode/<model>-free [runs]
#   sh scripts/cli-history-check.sh claude claude-haiku-5-5 [runs]
#   sh scripts/cli-history-check.sh codex <model> [runs]
#
# It records the path, size, modification time and SHA-256 of every file under the CLI's data folders,
# runs the rewrites through Retone's own adapter code (`retone --cli-check`), records again, and
# fails on any new, changed or removed file. It never deletes or edits the CLI's files.
# Do not use the CLI in another window while the check runs: that also changes these folders.
set -eu

usage() {
	echo "usage: sh scripts/cli-history-check.sh pi|opencode|claude|codex MODEL [RUNS]" >&2
	exit 2
}

[ $# -ge 2 ] || usage
cli=$1
model=$2
runs=${3:-5}

# The same folders as the plan's release gate. Pi: all of ~/.pi/agent, wider than the app's
# runtime guard (sessions only), so settings or caches written by a new version also show.
# The folders become the positional arguments, so a HOME with spaces stays one path each.
case $cli in
pi)
	set -- "$HOME/.pi/agent"
	source_file=src/provider/pi.rs
	;;
opencode)
	set -- "$HOME/.local/share/opencode" "$HOME/.local/state/opencode" "$HOME/.cache/opencode" "$HOME/.config/opencode"
	source_file=src/provider/opencode.rs
	;;
claude)
	# CLAUDE_CONFIG_DIR moves the config folder (and its .claude.json) when set. ~/.claude.json is a
	# file; find and stat take it like a folder with one entry.
	case ${CLAUDE_CONFIG_DIR:-} in
	'' | /*) ;;
	*)
		echo "FAIL: CLAUDE_CONFIG_DIR must be an absolute path (it is '$CLAUDE_CONFIG_DIR')." >&2
		exit 1
		;;
	esac
	set -- "${CLAUDE_CONFIG_DIR:-$HOME/.claude}" "$HOME/.claude.json"
	source_file=src/provider/claude.rs
	;;
codex)
	set -- "${CODEX_HOME:-$HOME/.codex}"
	source_file=src/provider/codex.rs
	;;
*) usage ;;
esac

cd "$(dirname "$0")/.."
cargo build --quiet
retone=target/debug/retone

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# snapshot OUT DIR...: every non-folder entry with modification time, size and path (BSD stat,
# lstat), a SHA-256 of every regular file (times have one-second steps, so a same-size rewrite
# within a second needs the hash), and every folder path, at every level, unlike the runtime
# guard. Not in a pipe, so `set -e` stops the check when find, stat or shasum cannot read a path:
# an incomplete snapshot is no proof.
# Folder times are kept apart (OUT.folders) and only print a note: a folder's time also changes
# when an entry is made and deleted again. Pi does that on every start (even `pi --version`,
# which gets no text), and nothing stays on disk.
snapshot() {
	out=$1
	shift
	: >"$out.raw"
	: >"$out.folders.raw"
	for dir in "$@"; do
		if [ -e "$dir" ]; then
			{
				find -H "$dir" ! -type d -exec stat -f '%m %z %N' {} +
				find -H "$dir" -type f -exec shasum -a 256 {} +
				find -H "$dir" -type d -exec stat -f 'folder %N' {} +
			} >>"$out.raw"
			find -H "$dir" -type d -exec stat -f '%m %N' {} + >>"$out.folders.raw"
		else
			# [ -e ] is also false when a parent folder cannot be read. Only "No such file or
			# directory" means missing; any other error stops the check, because both snapshots
			# would then say "missing" and pass with nothing checked.
			err=$(LC_ALL=C ls -d "$dir" 2>&1 >/dev/null) || true
			case $err in
			*'No such file or directory'*) echo "missing $dir" >>"$out.raw" ;;
			*)
				echo "FAIL: cannot read $dir${err:+: $err}. The check is not complete." >&2
				exit 1
				;;
			esac
		fi
	done
	LC_ALL=C sort "$out.raw" >"$out"
	LC_ALL=C sort "$out.folders.raw" >"$out.folders"
}

snapshot "$work/before" "$@"
# A pipe hides the exit code in POSIX sh, so the code is printed as a line and read back.
{ "$retone" --cli-check --provider "$cli" --model "$model" --runs "$runs" 2>&1 && echo "exit=0" || echo "exit=$?"; } |
	tee "$work/run"
snapshot "$work/after" "$@"

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
echo "PASS: $cli $version left no new, changed or removed file in: $*"
if ! diff -q "$work/before.folders" "$work/after.folders" >/dev/null; then
	echo "Note: these folder times changed, with no file left behind (a short-lived lock or temp file):"
	diff "$work/before.folders" "$work/after.folders" | sed -n 's/^> [0-9]* /  /p'
fi
if ! grep -q "\"$version\"" "$source_file"; then
	echo "Add \"$version\" to VERIFIED in $source_file."
fi
