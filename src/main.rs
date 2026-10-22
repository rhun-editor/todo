//! todo: the project's TODO, FIXME, XXX and HACK comments in a sidebar tree (a retained view,
//! protocol 1.1 feature view-tree), grouped by file; a click opens the line. ripgrep finds them
//! when it is on PATH, else a built-in walker that leaves out hidden folders, dependencies and
//! build output. Rescan: the section's button, the command, or a saved file.

use rhun_ext::views::{Icon, Kids, Node, Patch, RetainedEvent, State, ViewColor, ViewOpen, Where};
use rhun_ext::{run, Event, Ext, Feature, Handler, View};
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

const WORDS: [&str; 4] = ["TODO", "FIXME", "XXX", "HACK"];
/// At most this many comments shown (the rest are counted in the message).
const MAX_ITEMS: usize = 20_000;
const MAX_PER_FILE: usize = 1000;
const MAX_FILE: u64 = 1 << 20;
const SKIP_DIRS: [&str; 8] = [
    "node_modules",
    "target",
    "build",
    "dist",
    "out",
    "vendor",
    "__pycache__",
    "venv",
];

/// One comment: the file (relative, with /), its line and column (from 1), the text from the word.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    pub path: String,
    pub line: u32,
    pub col: u32,
    pub word: &'static str,
    pub text: String,
}

/// The first of WORDS in the line as a whole word: its byte offset and the word.
pub fn find_word(line: &str) -> Option<(usize, &'static str)> {
    let b = line.as_bytes();
    let mut best: Option<(usize, &'static str)> = None;
    for w in WORDS {
        let mut from = 0;
        while let Some(i) = line[from..].find(w) {
            let at = from + i;
            let before = at == 0 || !(b[at - 1].is_ascii_alphanumeric() || b[at - 1] == b'_');
            let end = at + w.len();
            let after = end >= b.len() || !(b[end].is_ascii_alphanumeric() || b[end] == b'_');
            if before && after {
                if best.is_none_or(|(o, _)| at < o) {
                    best = Some((at, w));
                }
                break;
            }
            from = at + 1;
        }
    }
    best
}

fn hit(path: &str, line: u32, text: &str) -> Option<Hit> {
    let (at, word) = find_word(text)?;
    let mut t: String = text[at..].trim_end().chars().take(200).collect();
    if t.is_empty() {
        t = word.to_string();
    }
    Some(Hit {
        path: rel_path(path),
        line,
        col: text[..at].chars().count() as u32 + 1,
        word,
        text: t,
    })
}

/// A path as rg or the walker gives it (`./src/a.rs`, `.\src\a.rs` on Windows) relative, with /.
fn rel_path(path: &str) -> String {
    let mut p = path.replace('\\', "/");
    while let Some(rest) = p.strip_prefix("./") {
        p = rest.to_string();
    }
    p
}

/// A hit's file as rhun writes paths: the project's root as rhun gave it, `/`, the relative path
/// (C:/p/src/a.rs on Windows too, where Path::join would give C:/p\src\a.rs and rhun would not
/// match it to the open document).
fn abs_path(root: &str, rel: &str) -> String {
    format!("{}/{rel}", root.trim_end_matches('/'))
}

/// ripgrep's `path NUL line:text` lines as hits (None when rg cannot run).
fn scan_rg(root: &str) -> Option<Vec<Hit>> {
    let out = Command::new("rg")
        .args([
            "--no-heading",
            "--with-filename",
            "--null",
            "--line-number",
            "--color",
            "never",
            "--no-messages",
            "-e",
        ])
        .arg(r"\b(TODO|FIXME|XXX|HACK)\b")
        .arg(".")
        .current_dir(root)
        .output()
        .ok()?;
    if !out.status.success() && out.status.code() != Some(1) {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    Some(parse_rg(&text))
}

/// `path NUL line:text` lines (ripgrep with --no-heading --null --line-number) as hits. The NUL
/// ends the path, so a drive (`C:\`) or a colon in a file's name cannot split it.
pub fn parse_rg(text: &str) -> Vec<Hit> {
    let mut hits = Vec::new();
    for l in text.lines() {
        let Some((path, after)) = l.split_once('\0') else {
            continue;
        };
        let Some((line, rest)) = after.split_once(':') else {
            continue;
        };
        let Ok(line) = line.parse::<u32>() else {
            continue;
        };
        if let Some(h) = hit(path, line, rest) {
            hits.push(h);
        }
        if hits.len() > MAX_ITEMS * 2 {
            break;
        }
    }
    hits
}

/// The built-in walker: text files under root, hidden folders, dependencies and build output left
/// out.
pub fn scan_walk(root: &Path) -> Vec<Hit> {
    let mut hits = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut entries: Vec<_> = rd.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let name = e.file_name().to_string_lossy().into_owned();
            let Ok(ft) = e.file_type() else { continue };
            let p = e.path();
            if ft.is_dir() {
                if !name.starts_with('.') && !SKIP_DIRS.contains(&name.as_str()) {
                    stack.push(p);
                }
                continue;
            }
            if !ft.is_file() || e.metadata().map(|m| m.len() > MAX_FILE).unwrap_or(true) {
                continue;
            }
            let Ok(bytes) = std::fs::read(&p) else {
                continue;
            };
            if bytes[..bytes.len().min(8000)].contains(&0) {
                continue;
            }
            let text = String::from_utf8_lossy(&bytes);
            let rel = p
                .strip_prefix(root)
                .unwrap_or(&p)
                .to_string_lossy()
                .replace('\\', "/");
            for (i, l) in text.lines().enumerate() {
                if let Some(h) = hit(&rel, i as u32 + 1, l) {
                    hits.push(h);
                }
            }
            if hits.len() > MAX_ITEMS * 2 {
                return hits;
            }
        }
    }
    hits
}

/// The view's content: files (by path) with their comments, the count as the badge.
pub fn tree(root: &str, mut hits: Vec<Hit>) -> Patch {
    hits.sort_by(|a, b| a.path.cmp(&b.path).then(a.line.cmp(&b.line)));
    let total = hits.len();
    let mut files: BTreeMap<String, Vec<Hit>> = BTreeMap::new();
    for h in hits.into_iter().take(MAX_ITEMS) {
        let v = files.entry(h.path.clone()).or_default();
        if v.len() < MAX_PER_FILE {
            v.push(h);
        }
    }
    let mut p = Patch::new();
    p.title("TODO")
        .badge(total.min(u32::MAX as usize) as u32)
        .action("rescan", Some("refresh"), Where::Title, "Rescan");
    if total > MAX_ITEMS {
        p.message(&format!("{MAX_ITEMS} of {total} shown"));
    } else {
        p.message("");
    }
    let mut top = Vec::new();
    for (path, list) in &files {
        let (folder, name) = match path.rfind('/') {
            Some(i) => (&path[..i], &path[i + 1..]),
            None => ("", path.as_str()),
        };
        let mut n = Node::new(format!("f:{path}"), name)
            .kids(Kids::Open)
            .icon(Icon::file(name), None)
            .badge(list.len() as u32)
            .tip(path);
        if !folder.is_empty() {
            n = n.desc(folder);
        }
        top.push(n);
    }
    p.kids(b"", &top, false);
    for (path, list) in &files {
        let abs = abs_path(root, path);
        let items: Vec<Node> = list
            .iter()
            .map(|h| {
                let (icon, color) = match h.word {
                    "FIXME" | "XXX" => ("error", ViewColor::Error),
                    _ => ("warning", ViewColor::Warning),
                };
                Node::new(format!("t:{path}:{}", h.line), &h.text)
                    .icon(Icon::named(icon), Some(color))
                    .highlight(0..h.word.len())
                    .desc(&format!(":{}", h.line))
                    .open(&abs, h.line, h.col)
            })
            .collect();
        p.kids(format!("f:{path}").as_bytes(), &items, false);
    }
    if total == 0 {
        p.state(State::Empty(
            "No TODO, FIXME, XXX or HACK comments in this project".into(),
            vec!["rescan".into()],
        ));
    } else {
        p.state(State::Ready);
    }
    p
}

#[derive(Default)]
struct Todo {
    root: Option<String>,
    view: Option<View>,
}

impl Todo {
    fn scan(&mut self, ext: &Ext) {
        let (Some(root), Some(view)) = (self.root.clone(), self.view) else {
            return;
        };
        let mut busy = Patch::new();
        busy.busy(true);
        let _ = ext.patch(view, &busy);
        let hits = scan_rg(&root).unwrap_or_else(|| scan_walk(Path::new(&root)));
        let mut p = tree(&root, hits);
        p.busy(false);
        if let Err(e) = ext.patch(view, &p) {
            eprintln!("todo: {e}");
        }
    }
}

impl Handler for Todo {
    fn ready(&mut self, ext: &Ext, project: Option<&str>) {
        self.root = project.map(String::from);
        self.scan(ext);
    }
    fn view_open(&mut self, ext: &Ext, open: &ViewOpen) {
        if open.name == "todo" {
            self.view = Some(open.view);
            if self.root.is_none() {
                let mut p = Patch::new();
                p.state(State::Empty(
                    "Open a folder to see its TODO comments".into(),
                    vec![],
                ));
                let _ = ext.patch(open.view, &p);
                return;
            }
            self.scan(ext);
        }
    }
    fn view_event(&mut self, ext: &Ext, view: View, ev: &RetainedEvent) {
        if Some(view) == self.view {
            if let RetainedEvent::Action { key, .. } = ev {
                if key == "rescan" {
                    self.scan(ext);
                }
            }
        }
    }
    fn command(&mut self, ext: &Ext, name: &str, _doc: rhun_ext::Doc) {
        if name == "rescan" {
            self.scan(ext);
        }
    }
    fn event(&mut self, ext: &Ext, ev: &Event) {
        if ev.name == "saved" {
            self.scan(ext);
        }
    }
}

fn main() {
    if let Err(e) = run(&[Feature::ViewTree], Todo::default()) {
        eprintln!("todo: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_are_whole_words() {
        assert_eq!(find_word("// TODO: parse"), Some((3, "TODO")));
        assert_eq!(
            find_word("let todos = TODOS; // FIXME here"),
            Some((22, "FIXME"))
        );
        assert_eq!(find_word("XXXL HACKER"), None);
        assert_eq!(find_word("# HACK and TODO"), Some((2, "HACK")));
    }

    #[test]
    fn rg_lines_and_the_tree() {
        let hits = parse_rg(
            "./src/main.rs\x002:    // TODO: parse\n.\\src\\lib.rs\x001:// FIXME lib\nREADME.md\x009:no words\n",
        );
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[1].path, "src/lib.rs");
        assert_eq!(hits[0].path, "src/main.rs");
        assert_eq!((hits[0].line, hits[0].col), (2, 8));
        assert_eq!(hits[0].text, "TODO: parse");
        let body = String::from_utf8(tree("/p", hits).encode().unwrap()).unwrap();
        assert!(body.contains("f:src/lib.rs"), "{body}");
        assert!(body.contains("t:src/main.rs:2"), "{body}");
        assert!(body.contains("open 2 8 14\n/p/src/main.rs"), "{body}");
        assert!(body.contains("state ready"), "{body}");
    }

    #[test]
    fn paths_as_rhun_writes_them() {
        assert_eq!(abs_path("C:/p", "src/a.rs"), "C:/p/src/a.rs");
        assert_eq!(abs_path("C:/", "a.rs"), "C:/a.rs");
        assert_eq!(abs_path("/", "a.rs"), "/a.rs");
        assert_eq!(rel_path(".\\src\\deep\\b.py"), "src/deep/b.py");
    }

    #[test]
    fn the_walker_leaves_out_hidden_and_build_folders() {
        let dir = std::env::temp_dir().join(format!("todo-walk-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::create_dir_all(dir.join("target")).unwrap();
        std::fs::create_dir_all(dir.join(".git")).unwrap();
        std::fs::write(dir.join("src/a.rs"), "fn a() {}\n// TODO one\n").unwrap();
        std::fs::write(dir.join("target/b.rs"), "// TODO built\n").unwrap();
        std::fs::write(dir.join(".git/c"), "TODO hidden\n").unwrap();
        std::fs::write(dir.join("bin.dat"), b"TODO\0binary").unwrap();
        let hits = scan_walk(&dir);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!((hits[0].path.as_str(), hits[0].line), ("src/a.rs", 2));
    }

    #[test]
    fn rg_finds_what_the_walker_finds() {
        let dir = std::env::temp_dir().join(format!("todo-rg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src/deep")).unwrap();
        std::fs::write(dir.join("src/a.rs"), "fn a() {}\n// TODO one\n").unwrap();
        std::fs::write(
            dir.join("src/deep/b.py"),
            "# FIXME two\nx = 1  # HACK three\n",
        )
        .unwrap();
        // rg is optional: without it there is nothing to compare
        let rg = scan_rg(dir.to_str().unwrap());
        let walk = scan_walk(&dir);
        let _ = std::fs::remove_dir_all(&dir);
        let Some(mut rg) = rg else { return };
        rg.sort_by(|a, b| a.path.cmp(&b.path).then(a.line.cmp(&b.line)));
        assert_eq!(rg.len(), 3, "{rg:?}");
        assert_eq!(rg[0].path, "src/a.rs");
        assert_eq!((rg[1].path.as_str(), rg[1].col), ("src/deep/b.py", 3));
        let mut walk = walk;
        walk.sort_by(|a, b| a.path.cmp(&b.path).then(a.line.cmp(&b.line)));
        assert_eq!(rg, walk);
    }
}
