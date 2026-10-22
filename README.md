# TODO

The project's `TODO`, `FIXME`, `XXX` and `HACK` comments in a tree in [rhun](https://rhun.app)'s
sidebar, grouped by file. A small Rust program built on
[rhun-ext](https://github.com/rhun-editor/extension-protocol/tree/main/rust/rhun-ext), and an
example of rhun's retained views: the extension sends the tree, and rhun draws, expands and
scrolls it.

- **A click** on a comment opens its file at that line.
- **Rescan**: the section's button, the **TODO: Rescan** command, or saving a file scans the
  project again.
- Each file's row shows how many comments it holds, and the word (TODO, FIXME, …) is highlighted.

ripgrep (`rg`) finds the comments when it is on your PATH, honoring `.gitignore`; otherwise a
built-in walker reads the project's text files, leaving out hidden folders, dependencies and
build output. At most 20000 comments are shown.

Install it from rhun's Extensions page (the extensions button in the title bar): **Install from
GitHub…**, then `rhun-editor/todo`. Or take `todo-TARGET.tar.gz` for your system from the
[latest release](https://github.com/rhun-editor/todo/releases/latest) and use **Install** >
**From an Archive**. It needs rhun 0.18 (extension protocol 1.1: retained views, feature
`view-tree`).

## Permissions

It reads your project's files and runs `rg` when you have it. It changes nothing and has no
internet.

## Build and test

```sh
cargo test
cargo build --release
tests/run.sh [RHUN]     # headless, against the rhun on your PATH
tests/check.sh          # the protocol's conformance checker (needs Go)
```

`tests/run.sh` builds the extension, lays it out as rhun installs it and runs `tests/basic.txt`
headlessly in a small project: the tree, a click that opens a comment's line, and a new comment
saved and found. A tag `vX.Y.Z` equal to `version` in `extension.ini` builds Linux, macOS and
Windows on their own runners and publishes what Install from GitHub takes.
