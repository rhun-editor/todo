#!/bin/sh
# Builds the extension for this machine, lays it out as rhun installs it, and runs tests/basic.txt
# headlessly against a rhun binary: the TODO tree of a small project, a click that opens a
# comment's line, and a save that scans again. usage: tests/run.sh [RHUN]; UPDATE=1 writes
# tests/basic.expected
set -e
cd "$(dirname "$0")/.."
RHUN=${1:-$(command -v rhun || echo "$HOME/.local/bin/rhun")}
name=$(sed -n 's/^name *= *//p' extension.ini | head -1)
owner=$(sed -n 's/^owner *= *//p' extension.ini | head -1)
case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) target=linux-x86_64 ;;
    Darwin-arm64) target=macos-arm64 ;;
    *) echo "no target for $(uname -s)-$(uname -m)"; exit 77 ;;
esac
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
dir=$work/exts/$owner/$name
mkdir -p "$dir/bin" "$work/proj/src" "$work/config/rhun" "$work/state"
sh tests/layout.sh "$dir"  # what the release ships: the manifest, README.md, LICENSE
cargo build --release --quiet
cp "target/release/$name" "$dir/bin/$name-$target"
printf 'fn main() {\n    // TODO: say hello\n    let x = 1; // FIXME overflow\n}\n' > "$work/proj/src/main.rs"
printf '# notes\n# HACK around it\nXXX nothing yet\n' > "$work/proj/notes.sh"
printf '[files]\nrestore_session = false\nrestore_project = false\n[updates]\ncheck = false\n[git]\nenabled = false\n[ui]\nsidebar = false\nagents_panel = false\n[editor]\ncursor_blink = false\n' > "$work/config/rhun/config"
run() { # SCRIPT: runs it in a fresh editor, the output on stdout
    HOME=$work XDG_CONFIG_HOME=$work/config XDG_STATE_HOME=$work/state \
        "$RHUN" "$work/proj" --headless 1000x600 --scale 1 --extensions "$work/exts" --script "$1" 2>&1
}
# where the rows are drawn depends on the platform's font: ask first, then click FIXME's row
printf 'wait 100\nview-show todo\nwait-ext todo running\nwait 800\nprint-view todo\n' > "$work/rows.txt"
row=$(run "$work/rows.txt" | sed -n 's/^row [0-9]* y=\([0-9]*\) .*id=t:src\/main.rs:3 .*$/\1/p' | head -1)
[ -n "$row" ] || { echo "FAIL basic: no row for the FIXME comment"; exit 1; }
sed -e "s|^click ROW|click 100 $((row + 14))|" tests/basic.txt > "$work/basic.txt"
# confined=2 is a Linux without unprivileged user namespaces (GitHub's Ubuntu runners): Landlock
# holds the files, so the extension is held all the same
run "$work/basic.txt" | sed -e 's/pid=[1-9][0-9]*/pid=PID/' -e 's/ confined=2/ confined=1/' \
    -e 's/ y=[0-9]*//' -e 's/^at=[0-9,]* size=[0-9x]* /at=X,Y size=WxH /' > "$work/out.txt"
[ -z "$UPDATE" ] || cp "$work/out.txt" tests/basic.expected  # UPDATE=1 writes the expected output
if cmp -s "$work/out.txt" tests/basic.expected; then
    echo "ok   basic"
else
    echo "FAIL basic"; diff tests/basic.expected "$work/out.txt" || true; exit 1
fi
