# TODO

A first-party rhun extension: the project's `TODO`, `FIXME`, `XXX` and `HACK` comments in a tree
in the sidebar, grouped by file. A click opens the line; the section's Rescan button, the
**TODO: Rescan** command and saving a file scan again. ripgrep (`rg`) finds them when it is on
PATH; otherwise a built-in walker reads the project's text files, leaving out hidden folders,
dependencies and build output. At most 20000 comments are shown.

Protocol 1.1, feature `view-tree` (retained views). Built with the Rust SDK `rhun-ext`.

    cargo build --release && mkdir -p bin && cp target/release/todo bin/todo-macos-arm64
