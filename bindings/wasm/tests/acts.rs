//! Every command declares the editor state it reads (`KitCommand::reads`): what the engine's code
//! reads, found by reading it, not by what descriptions say. Each command's definition (a `cmd!`,
//! `query!` or other constructor call, or a `CommandSpec { … }`) is followed through the engine's
//! functions, the `Session` methods, the macros and the commands it runs (`execute("…")`) to the
//! editor state: the layer, property, keyframe and mask-vertex selections (`selection`), the
//! active composition (`part`) and its current time (`time`). A command that reaches one and does
//! not declare it, or declares what it never reads, fails here; so does the kit's own table of the
//! commands that read the keyframe selection, which `on` (layer ids) cannot set (task card
//! C4-motion). Where the scan cannot tell (`mode`, `active`, `area`; a function pointer; a path a
//! command does not take), the difference is reviewed and listed, with its reason, in
//! `acts_known_differences.txt` beside this file, which the tests hold strictly: a difference not
//! listed, or a listed one that no longer differs, fails. To see how the scan reached a reading,
//! set `ACTS_DEBUG=<command id>` or `ACTS_FN=<engine function>`; `ACTS_PRINT=1` prints what the
//! scan derives for every command and each difference (`difference <command> | <difference>`).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use kition_kit::KitDocument;
use kition_motion_kit::MotionDocument;

/// Code that reads a selection the call context sets or keeps (`on` is layer ids): the selected
/// layers, and the properties, keyframes and mask vertices selected on them (`st` is the engine's
/// name for the editor state an edit gets).
const SELECTION: &[&str] = &[
    "state.selected_layers",
    "st.selected_layers",
    "state.selected_props",
    "st.selected_props",
    "state.selected_keys",
    "st.selected_keys",
    "state.selected_vertices",
    "st.selected_vertices",
];
/// Code that reads the keyframe selection: keyframes have no ids, so `on` cannot name them; the
/// commands that read it are the kit's table (`acts::reads_key_selection`).
const KEYS: &[&str] = &["state.selected_keys", "st.selected_keys"];
/// Code that reads which composition is active.
const PART: &[&str] = &["state.active_comp", "st.active_comp"];
/// Code that reads a composition's current time.
const TIME: &[&str] = &["state.times", "st.times"];
/// Code that reads the Project panel's selection: editor state no call context sets (`on` is layer
/// ids); counted, not declared.
const PROJECT: &[&str] = &["state.project_selection", "st.project_selection"];
/// Code that resolves a layer reference of a composition: an id, else a 1-based stack position
/// (`#n`) or a name. A command that reaches it takes the layers `on` names.
const LAYERS: &[&str] = &["resolve_layer"];

/// Calls the scan does not follow: running a command (followed by id instead, through `runs`),
/// the engine's command table, the dispatcher's own checks, and `Session::sanitize_state`, which
/// drops from the editor state the ids an edit removed (its `retain`s look at the selection only
/// to prune it; no document change depends on it, and the host puts the editor state back).
const NOT_FOLLOWED: &[&str] =
    &["execute", "execute_as", "execute_unguarded", "enabled_for", "is_enabled", "build", "command_specs", "commands", "find", "find_spec", "sanitize_state"];

fn engine_src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/engine/engine/src")
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs")
            && !path.file_name().is_some_and(|n| {
                let n = n.to_string_lossy();
                n == "tests.rs" || n.starts_with("tests_") || n.ends_with("_tests.rs")
            })
        {
            out.push(path);
        }
    }
}

/// `source` with every comment, string and character literal blanked (to spaces), so brackets and
/// names are only the code's; same length, byte for byte.
fn code_only(source: &str) -> Vec<u8> {
    let b = source.as_bytes();
    let mut out = b.to_vec();
    let mut i = 0;
    let blank = |out: &mut Vec<u8>, from: usize, to: usize| {
        for byte in out.iter_mut().take(to).skip(from) {
            *byte = b' ';
        }
    };
    while i < b.len() {
        let ident_before = i > 0 && (b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_');
        if b[i..].starts_with(b"//") {
            let end = b[i..].iter().position(|&c| c == b'\n').map_or(b.len(), |n| i + n);
            blank(&mut out, i, end);
            i = end;
        } else if b[i..].starts_with(b"/*") {
            let end = source[i + 2..].find("*/").map_or(b.len(), |n| i + 2 + n + 2);
            blank(&mut out, i, end);
            i = end;
        } else if !ident_before && (b[i] == b'r' || b[i..].starts_with(b"br")) && {
            let start = if b[i] == b'b' { i + 2 } else { i + 1 };
            let hashes = b[start..].iter().take_while(|&&c| c == b'#').count();
            b.get(start + hashes) == Some(&b'"')
        } {
            let start = if b[i] == b'b' { i + 2 } else { i + 1 };
            let hashes = b[start..].iter().take_while(|&&c| c == b'#').count();
            let close = format!("\"{}", "#".repeat(hashes));
            let body = start + hashes + 1;
            let end = source[body..].find(&close).map_or(b.len(), |n| body + n + close.len());
            blank(&mut out, i, end);
            i = end;
        } else if b[i] == b'"' {
            let mut j = i + 1;
            while j < b.len() && b[j] != b'"' {
                j += if b[j] == b'\\' { 2 } else { 1 };
            }
            blank(&mut out, i, (j + 1).min(b.len()));
            i = j + 1;
        } else if b[i] == b'\'' {
            // A character literal ('a', '\n', '{', 'é'), not a lifetime ('a).
            let rest = &source[i + 1..];
            let len = if let Some(escaped) = rest.strip_prefix('\\') {
                escaped.find('\'').map(|n| n + 2)
            } else {
                rest.chars().next().map(char::len_utf8).filter(|&n| rest[n..].starts_with('\''))
            };
            match len {
                Some(len) => {
                    blank(&mut out, i, i + len + 2);
                    i += len + 2;
                }
                None => i += 1,
            }
        } else {
            i += 1;
        }
    }
    out
}

/// The position after the bracket matching the one at `open`.
fn matching(code: &[u8], open: usize) -> usize {
    let (left, right) = match code[open] {
        b'(' => (b'(', b')'),
        b'{' => (b'{', b'}'),
        _ => (b'[', b']'),
    };
    let mut depth = 0usize;
    for (at, &c) in code.iter().enumerate().skip(open) {
        if c == left {
            depth += 1;
        } else if c == right {
            depth -= 1;
            if depth == 0 {
                return at + 1;
            }
        }
    }
    code.len()
}

fn is_ident(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

fn ident_at(code: &[u8], at: usize) -> &str {
    let end = code[at..].iter().position(|c| !is_ident(*c)).map_or(code.len(), |n| at + n);
    std::str::from_utf8(&code[at..end]).unwrap_or("")
}

/// The identifier that ends right before `at` (skipping whitespace), and where it starts.
fn ident_before(code: &[u8], at: usize) -> Option<(usize, &str)> {
    let mut end = at;
    while end > 0 && code[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    let mut start = end;
    while start > 0 && is_ident(code[start - 1]) {
        start -= 1;
    }
    (start < end).then(|| (start, std::str::from_utf8(&code[start..end]).unwrap_or("")))
}

fn skip_space(code: &[u8], at: usize) -> usize {
    at + code[at..].iter().take_while(|byte| byte.is_ascii_whitespace()).count()
}

fn compact_tokens(code: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(code.len());
    let mut at = 0;
    while at < code.len() {
        if code[at].is_ascii_whitespace() {
            let next = skip_space(code, at);
            if out.last().copied().is_some_and(is_ident) && code.get(next).copied().is_some_and(is_ident) {
                out.push(b' ');
            }
            at = next;
        } else {
            out.push(code[at]);
            at += 1;
        }
    }
    out
}

/// A local value shadows a free function only from its binding to its enclosing block's end.
fn local_bindings(code: &[u8]) -> Vec<(String, usize, usize)> {
    let mut out = Vec::new();
    let mut blocks = Vec::new();
    let mut at = 0;
    while at < code.len() {
        match code[at] {
            b'{' => blocks.push(at),
            b'}' => {
                blocks.pop();
            }
            _ => {}
        }
        if ident_at(code, at) == "let" && (at == 0 || !is_ident(code[at - 1])) {
            let mut start = skip_space(code, at + 3);
            if ident_at(code, start) == "mut" {
                start = skip_space(code, start + 3);
            }
            let name = ident_at(code, start);
            if !name.is_empty() {
                out.push((name.to_owned(), start + name.len(), blocks.last().map_or(code.len(), |open| matching(code, *open))));
            }
        }
        at += 1;
    }
    out
}

/// A qualified call keeps its namespace; its last identifier alone is ambiguous across modules.
fn call_path(code: &[u8], at: usize) -> String {
    let mut parts = vec![ident_at(code, at)];
    let mut before = at;
    loop {
        while before > 0 && code[before - 1].is_ascii_whitespace() {
            before -= 1;
        }
        if before < 2 || &code[before - 2..before] != b"::" {
            break;
        }
        let Some((start, name)) = ident_before(code, before - 2) else { break };
        parts.push(name);
        before = start;
    }
    parts.reverse();
    parts.join("::")
}

fn absolute_path(module: &str, path: &str) -> String {
    let mut parts: Vec<_> = module.split("::").filter(|part| !part.is_empty()).collect();
    let mut tail = path.split("::").peekable();
    match tail.peek().copied() {
        Some("crate") => {
            parts.clear();
            tail.next();
        }
        Some("self") => {
            tail.next();
        }
        Some("super") => {
            while tail.peek() == Some(&"super") {
                parts.pop();
                tail.next();
            }
        }
        _ => {}
    }
    parts.extend(tail);
    parts.join("::")
}

/// Import trees and aliases are lexical scope, including grouped and glob imports.
fn imports(code: &[u8], module: &str) -> BTreeMap<String, String> {
    fn tree(tokens: &[String], at: &mut usize, prefix: &[String], out: &mut Vec<(String, String)>) {
        let mut path = prefix.to_vec();
        while let Some(token) = tokens.get(*at) {
            match token.as_str() {
                "{" => {
                    *at += 1;
                    while tokens.get(*at).is_some_and(|token| token != "}") {
                        tree(tokens, at, &path, out);
                        if tokens.get(*at).is_some_and(|token| token == ",") {
                            *at += 1;
                        }
                    }
                    *at += usize::from(tokens.get(*at).is_some());
                    return;
                }
                "::" => {
                    *at += 1;
                }
                "as" => {
                    *at += 1;
                    if let Some(alias) = tokens.get(*at) {
                        out.push((alias.clone(), path.join("::")));
                        *at += 1;
                    }
                    return;
                }
                "," | "}" | ";" => break,
                _ => {
                    path.push(token.clone());
                    *at += 1;
                }
            }
        }
        if let Some(name) = path.last() {
            let alias = if name == "self" && path.len() > 1 { path.get(path.len() - 2).unwrap() } else { name };
            out.push((alias.clone(), path.join("::")));
        }
    }
    let mut tokens = Vec::new();
    let mut at = 0;
    while at < code.len() {
        if code[at].is_ascii_whitespace() {
            at += 1;
            continue;
        }
        let name = ident_at(code, at);
        let token = if !name.is_empty() {
            name.to_owned()
        } else if code.get(at..at + 2) == Some(b"::") {
            "::".into()
        } else {
            char::from(code[at]).to_string()
        };
        at += token.len();
        tokens.push(token);
    }
    let mut out = BTreeMap::new();
    for (start, token) in tokens.iter().enumerate() {
        if token != "use" {
            continue;
        }
        let mut imported = Vec::new();
        tree(&tokens, &mut (start + 1), &[], &mut imported);
        for (name, path) in imported {
            let internal = path.split("::").next().is_some_and(|part| ["crate", "self", "super"].contains(&part));
            let path = if internal { absolute_path(module, &path) } else { path };
            let key = if name == "*" { format!("*{path}") } else { name };
            out.insert(key, path.trim_end_matches("::*").to_owned());
        }
    }
    out
}

/// The source cut before its `#[cfg(test)]` module, the code of what is left, its file, and the
/// byte ranges of its `impl Session` blocks.
struct File {
    source: String,
    code: Vec<u8>,
    name: String,
    imports: BTreeMap<String, String>,
    session_impls: Vec<(usize, usize)>,
}

fn files() -> Vec<File> {
    let mut paths = Vec::new();
    rust_files(&engine_src(), &mut paths);
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let mut source = std::fs::read_to_string(&path).unwrap();
            if let Some(at) = source.find("#[cfg(test)]\nmod ") {
                source.truncate(at);
            }
            let code = code_only(&source);
            let text = std::str::from_utf8(&code).unwrap_or("").to_owned();
            let session_impls = ["impl Session {", "impl crate::Session {", "impl super::Session {"]
                .iter()
                .flat_map(|head| text.match_indices(head).map(|(at, head)| (at, matching(&code, at + head.len() - 1))).collect::<Vec<_>>())
                .collect();
            let relative = path.strip_prefix(engine_src()).unwrap().with_extension("");
            let mut parts: Vec<_> = relative.iter().map(|part| part.to_string_lossy().into_owned()).collect();
            if parts.last().is_some_and(|part| part == "lib" || part == "mod") {
                parts.pop();
            }
            let name = parts.join("::");
            let imports = imports(&code, &name);
            File { name, source, code, imports, session_impls }
        })
        .collect()
}

/// What a piece of code reads directly, the free functions, `Session` methods and macros it calls,
/// and the commands it runs.
#[derive(Default, Clone)]
struct Reach {
    reads: BTreeSet<&'static str>,
    calls: BTreeSet<String>,
    methods: BTreeSet<String>,
    runs: BTreeSet<String>,
}

fn reach(code: &[u8], source: &str, modules: &BTreeSet<String>) -> Reach {
    let compact = compact_tokens(code);
    let text = std::str::from_utf8(&compact).unwrap_or("");
    let bindings = local_bindings(code);
    let mut found = Reach::default();
    let has = |token: &str| {
        text.match_indices(token).any(|(at, _)| {
            let end = at + token.len();
            // `state.selection` is not `state.selection_follows_playhead`, nor `my_state.selection`.
            let whole = !compact.get(end).copied().is_some_and(is_ident) && !(at > 0 && is_ident(compact[at - 1]));
            // Setting it (`st.selection = vec![id]`, `st.selection.clear()`) is no read.
            let rest = text[end..].trim_start();
            let written = (rest.starts_with('=') && !rest.starts_with("=="))
                || [".clear()", ".push(", ".extend(", ".truncate(", ".dedup(", ".insert(", ".remove("].iter().any(|write| rest.starts_with(write));
            whole && !written
        })
    };
    for (kind, tokens) in [("selection", SELECTION), ("part", PART), ("time", TIME), ("keys", KEYS), ("project", PROJECT), ("layers", LAYERS)] {
        if tokens.iter().any(|token| has(token)) {
            found.reads.insert(kind);
        }
    }
    for at in 0..code.len() {
        let starts = (code[at].is_ascii_alphabetic() || code[at] == b'_') && (at == 0 || !is_ident(code[at - 1]));
        if !starts {
            continue;
        }
        let name = ident_at(code, at);
        let after = skip_space(code, at + name.len());
        let previous = code[..at].iter().rposition(|byte| !byte.is_ascii_whitespace());
        let method = previous.is_some_and(|previous| code[previous] == b'.');
        let path = call_path(code, at);
        let shadowed = !path.contains("::") && bindings.iter().any(|(bound, from, to)| bound == name && (*from..*to).contains(&at));
        if code.get(after) == Some(&b'(') {
            if method {
                found.methods.insert(name.to_owned());
            } else if !shadowed && in_engine(code, at, modules) {
                found.calls.insert(path.clone());
            }
        }
        if code.get(after) == Some(&b'!') && code.get(after + 1).is_some_and(|c| matches!(c, b'(' | b'[' | b'{')) {
            found.calls.insert(format!("{path}!"));
        }
        // `s.execute("time.set", …)`: the command runs another, and reads what it reads.
        if ["execute", "execute_as", "execute_unguarded", "find_spec"].contains(&name) && code.get(after) == Some(&b'(') {
            let rest = source[after + 1..].trim_start();
            if let Some(stripped) = rest.strip_prefix('"')
                && let Some(end) = stripped.find('"')
            {
                found.runs.insert(stripped[..end].to_owned());
            } else if name != "find_spec" {
                // A command id from input can select any registered command, including future
                // editor-state kinds; it is not a call to an unrelated function named `run`.
                found.runs.insert("*".into());
            }
        }
    }
    found
}

#[test]
fn source_reads_ignore_whitespace_and_local_callback_shadowing() {
    let source = "s.state\n .selected_layers.first(); let apply = |value| value; apply(1);";
    let found = reach(&code_only(source), source, &BTreeSet::new());
    assert!(found.reads.contains("selection"), "a field access may cross whitespace");
    assert!(!found.calls.contains("apply"), "a local callback is not an unrelated free function");
    let source = "let apply = |value| value; super :: other :: apply (1); s.state.selected_layers.retain(|id| keep(id));";
    let found = reach(&code_only(source), source, &BTreeSet::new());
    assert!(found.calls.contains("super::other::apply"), "a qualified path is not shadowed by a local value");
    assert!(found.reads.contains("selection"), "retain predicates inspect the old selected elements");
}

/// Pruning the editor state after an edit (`sanitize_state`) is no read: undo reads no selection,
/// while a query that falls back to the selected layer (`layer_p`) still does.
#[test]
fn pruning_the_editor_state_is_no_read() {
    let derived = derived(&["edit.undo".to_owned(), "layer.tree".to_owned()]);
    assert!(!derived["edit.undo"].contains("selection"), "{:?}", derived["edit.undo"]);
    assert!(derived["layer.tree"].contains("selection"), "{:?}", derived["layer.tree"]);
}

#[test]
fn source_calls_keep_import_namespaces_and_dynamic_command_dispatch() {
    let source = "use super::{prop::{prop_ref as lookup}, app_more::grouped}; use crate::commands::*;";
    let imported = imports(&code_only(source), "commands::anim");
    assert_eq!(imported.get("lookup").map(String::as_str), Some("commands::prop::prop_ref"));
    assert_eq!(imported.get("grouped").map(String::as_str), Some("commands::app_more::grouped"));
    assert_eq!(imported.get("*commands::*").map(String::as_str), Some("commands"));
    let source = "s.execute(command_from_input, params); s.execute(\"layer.newNull\", params);";
    let found = reach(&code_only(source), source, &BTreeSet::new());
    assert_eq!(found.runs, BTreeSet::from(["*".to_owned(), "layer.newNull".to_owned()]));
}

/// Whether the call at `at` is to the engine's own code: unqualified, or through `crate`, `super`,
/// `self` or one of its modules, not `std::mem::take` or another crate's function.
fn in_engine(code: &[u8], at: usize, modules: &BTreeSet<String>) -> bool {
    let path = call_path(code, at);
    let first = path.split("::").next().unwrap_or("");
    !path.contains("::") || ["crate", "super", "self"].contains(&first) || modules.contains(first)
}

/// A function's or macro's body, by name, with the file it is in: free functions under their
/// name, `Session` methods under `.name`, macros under `name!`.
type Bodies = BTreeMap<String, Vec<(String, Reach)>>;

fn bodies(files: &[File], modules: &BTreeSet<String>) -> Bodies {
    let mut out: Bodies = BTreeMap::new();
    for file in files {
        let code = &file.code;
        let text = std::str::from_utf8(code).unwrap_or("");
        for (at, _) in text.match_indices("fn ") {
            if at > 0 && is_ident(code[at - 1]) {
                continue;
            }
            let name = ident_at(code, at + 3).to_owned();
            if name.is_empty() {
                continue;
            }
            let Some(open) = code[at..].iter().position(|&c| c == b'(').map(|n| at + n) else { continue };
            let params_end = matching(code, open);
            let Some(brace) = code[params_end..].iter().position(|&c| c == b'{' || c == b';').map(|n| params_end + n) else { continue };
            if code[brace] == b';' {
                continue;
            }
            let end = matching(code, brace);
            let body = reach(&code[brace..end], &file.source[brace..end], modules);
            let method = file.session_impls.iter().any(|(from, to)| (*from..*to).contains(&at));
            let key = if method { format!(".{name}") } else { name };
            out.entry(key).or_default().push((file.name.clone(), body));
        }
        for (at, _) in text.match_indices("macro_rules!") {
            let name = ident_at(code, at + "macro_rules!".len() + 1).to_owned();
            let Some(brace) = code[at..].iter().position(|&c| c == b'{').map(|n| at + n) else { continue };
            let end = matching(code, brace);
            let body = reach(&code[brace..end], &file.source[brace..end], modules);
            out.entry(format!("{name}!")).or_default().push((file.name.clone(), body));
        }
    }
    out
}

/// The names (functions and macros) whose bodies make a `CommandSpec`, directly or through
/// another of them: `cmd!`, `query!`, the modules' `spec` helpers, `tt!`, …
fn constructors(files: &[File]) -> BTreeSet<String> {
    let mut direct: BTreeMap<String, (String, String)> = BTreeMap::new();
    for file in files {
        let code = &file.code;
        let text = std::str::from_utf8(code).unwrap_or("");
        for (at, _) in text.match_indices("fn ").chain(text.match_indices("macro_rules!")) {
            if at > 0 && is_ident(code[at - 1]) {
                continue;
            }
            let is_macro = text[at..].starts_with("macro_rules!");
            let name = if is_macro { ident_at(code, at + "macro_rules!".len() + 1) } else { ident_at(code, at + 3) };
            let Some(brace) = code[at..].iter().position(|&c| c == b'{').map(|n| at + n) else { continue };
            let end = matching(code, brace);
            let body = text[brace..end].to_owned();
            // A function returning `Vec<CommandSpec>` lists commands; it constructs none of them.
            let head = &text[at..brace];
            if !is_macro && head.contains("Vec<CommandSpec>") {
                continue;
            }
            direct.insert(if is_macro { format!("{name}!") } else { name.to_owned() }, (file.name.clone(), body));
        }
    }
    let mut found: BTreeSet<String> = direct.iter().filter(|(_, (_, body))| body.contains("CommandSpec {")).map(|(name, _)| name.clone()).collect();
    // `let q = |id, label, params, run| CommandSpec { … };`: a module's local constructor.
    for file in files {
        let text = std::str::from_utf8(&file.code).unwrap_or("");
        for (at, _) in text.match_indices("let ") {
            let name = ident_at(&file.code, at + 4);
            let rest = text[at + 4 + name.len()..].trim_start();
            if !name.is_empty() && rest.starts_with("= |") && rest.split(';').next().is_some_and(|statement| statement.contains("CommandSpec {")) {
                found.insert(name.to_owned());
            }
        }
    }
    loop {
        let more: Vec<String> = direct
            .iter()
            .filter(|(name, _)| !found.contains(*name))
            .filter(|(_, (_, body))| found.iter().any(|c| body.contains(&format!("{c}("))))
            .map(|(name, _)| name.clone())
            .collect();
        if more.is_empty() {
            break;
        }
        found.extend(more);
    }
    found
}

/// The code that defines command `id`: the call to a constructor whose first argument is the id,
/// or the `CommandSpec { id: "…", … }` (also one whose id a macro writes with `concat!`), with the
/// constructor itself as a call.
fn definitions(id: &str, files: &[File], constructors: &BTreeSet<String>, functions: &BTreeSet<String>, modules: &BTreeSet<String>) -> Vec<(String, Reach)> {
    let mut out = Vec::new();
    let literal = format!("\"{id}\"");
    for file in files {
        let code = &file.code;
        let source = &file.source;
        let text = std::str::from_utf8(code).unwrap_or("");
        let mut region = |from: usize, to: usize, constructor: Option<String>| {
            let mut found = reach(&code[from..to], &source[from..to], modules);
            // A function given by name (`enabled: has_seq`, `run: detect_beats`, `q(…, events_list)`).
            for at in from..to {
                if is_ident(code[at]) && !code[at].is_ascii_digit() && (at == 0 || !(is_ident(code[at - 1]) || code[at - 1] == b'.')) {
                    let name = ident_at(code, at);
                    let next = code.get(at + name.len()).copied().unwrap_or(b' ');
                    if next != b'(' && next != b'!' && functions.contains(name) && in_engine(code, at, modules) {
                        found.calls.insert(call_path(code, at));
                    }
                }
            }
            if let Some(constructor) = constructor {
                found.calls.insert(constructor);
            }
            out.push((file.name.clone(), found));
        };
        let spec_around = |at: usize| -> Option<(usize, usize)> {
            let start = text[..at].rfind("CommandSpec {")?;
            let open = start + "CommandSpec ".len();
            let end = matching(code, open);
            (end > at).then_some((open, end))
        };
        for (at, _) in source.match_indices(&literal) {
            let mut before = at;
            while before > 0 && code[before - 1].is_ascii_whitespace() {
                before -= 1;
            }
            if before > 0 && code[before - 1] == b'(' {
                let mut callee_end = before - 1;
                let bang = callee_end > 0 && code[callee_end - 1] == b'!';
                if bang {
                    callee_end -= 1;
                }
                let Some((_, name)) = ident_before(code, callee_end) else { continue };
                let name = if bang { format!("{name}!") } else { name.to_owned() };
                if constructors.contains(&name) {
                    region(before - 1, matching(code, before - 1), Some(name));
                }
            } else if before >= 3
                && &code[before - 3..before] == b"id:"
                && let Some((open, end)) = spec_around(at)
            {
                region(open, end, None);
            }
        }
        // `concat!("view.preset", $n)`: a macro writes the number.
        for (at, _) in text.match_indices("concat!(") {
            let rest = source[at + "concat!(".len()..].trim_start();
            let Some(prefix) = rest.strip_prefix('"').and_then(|rest| rest.split('"').next()) else { continue };
            let tail = id.strip_prefix(prefix);
            if prefix.contains('.')
                && tail.is_some_and(|tail| !tail.is_empty() && tail.bytes().all(|c| c.is_ascii_digit()))
                && let Some((open, end)) = spec_around(at)
            {
                region(open, end, None);
            }
        }
    }
    out
}

/// What every command reads, through the functions, methods, macros and commands it reaches.
fn derived(ids: &[String]) -> BTreeMap<String, BTreeSet<&'static str>> {
    let files = files();
    let mut modules: BTreeSet<String> = files.iter().flat_map(|file| file.name.split("::").map(str::to_owned)).collect();
    modules.insert("commands".into());
    let bodies = bodies(&files, &modules);
    let constructors = constructors(&files);
    let functions: BTreeSet<String> = bodies.keys().filter(|name| !name.starts_with('.') && !name.ends_with('!')).cloned().collect();
    // A body's reads, to a fixed point over the calls (a name defined in several files is the
    // caller's own file's when it has one, else every one of them).
    let mut reads: BTreeMap<(String, String), BTreeSet<&'static str>> = BTreeMap::new();
    let mut reached_runs: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    for (name, defs) in &bodies {
        for (file, body) in defs {
            reads.insert((name.clone(), file.clone()), body.reads.clone());
            reached_runs.insert((name.clone(), file.clone()), body.runs.clone());
        }
    }
    let callees = |body: &Reach| -> Vec<String> {
        body.calls
            .iter()
            .filter(|call| !NOT_FOLLOWED.contains(&call.rsplit("::").next().unwrap_or(call)))
            .cloned()
            .chain(body.methods.iter().filter(|method| !NOT_FOLLOWED.contains(&method.as_str())).map(|method| format!(".{method}")))
            .collect()
    };
    let resolved_keys = |name: &str, file: &str| -> Vec<(String, String)> {
        if name.starts_with('.') {
            return bodies.get(name).into_iter().flatten().map(|(file, _)| (name.to_owned(), file.clone())).collect();
        }
        let Some(context) = files.iter().find(|context| context.name == file) else { return Vec::new() };
        let macro_call = name.ends_with('!');
        let path = name.trim_end_matches('!');
        let mut parts = path.split("::");
        let first = parts.next().unwrap_or("");
        let tail = parts.collect::<Vec<_>>();
        let mut candidates = Vec::new();
        if ["crate", "super", "self"].contains(&first) {
            candidates.push(absolute_path(file, path));
        } else if let Some(imported) = context.imports.get(first) {
            candidates.push(std::iter::once(imported.as_str()).chain(tail.iter().copied()).collect::<Vec<_>>().join("::"));
        } else {
            candidates.push(absolute_path(file, path));
            if !tail.is_empty() || macro_call {
                candidates.push(path.to_owned());
            }
            for (alias, imported) in &context.imports {
                if alias.starts_with('*') {
                    candidates.push(format!("{imported}::{path}"));
                }
            }
        }
        for candidate in candidates {
            let (namespace, function) = candidate.rsplit_once("::").unwrap_or(("", candidate.as_str()));
            let function = if macro_call { format!("{function}!") } else { function.to_owned() };
            if bodies.get(&function).is_some_and(|defs| defs.iter().any(|(file, _)| file == namespace)) {
                return vec![(function, namespace.to_owned())];
            }
        }
        Vec::new()
    };
    let resolve = |name: &str, file: &str, reads: &BTreeMap<(String, String), BTreeSet<&'static str>>| -> BTreeSet<&'static str> {
        resolved_keys(name, file).iter().flat_map(|key| reads.get(key).cloned().unwrap_or_default()).collect()
    };
    loop {
        let mut changed = false;
        for (name, defs) in &bodies {
            for (file, body) in defs {
                let mut now = reads.get(&(name.clone(), file.clone())).cloned().unwrap_or_default();
                let mut now_runs = reached_runs.get(&(name.clone(), file.clone())).cloned().unwrap_or_default();
                for call in callees(body) {
                    if &call != name {
                        now.extend(resolve(&call, file, &reads));
                        for key in resolved_keys(&call, file) {
                            now_runs.extend(reached_runs.get(&key).cloned().unwrap_or_default());
                        }
                    }
                }
                if reached_runs.get(&(name.clone(), file.clone())) != Some(&now_runs) {
                    reached_runs.insert((name.clone(), file.clone()), now_runs);
                    changed = true;
                }
                if reads.get(&(name.clone(), file.clone())) != Some(&now) {
                    reads.insert((name.clone(), file.clone()), now);
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    if let Ok(wanted) = std::env::var("ACTS_FN") {
        for (file, body) in bodies.get(&wanted).into_iter().flatten() {
            eprintln!("{wanted} in {file}: direct {:?}", body.reads);
            for call in callees(body) {
                eprintln!("  calls {call}: {:?}", resolve(&call, file, &reads));
            }
        }
    }
    let mut commands: BTreeMap<String, BTreeSet<&'static str>> = BTreeMap::new();
    let mut runs: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut missing = Vec::new();
    for id in ids {
        let defs = definitions(id, &files, &constructors, &functions, &modules);
        if defs.is_empty() {
            missing.push(id.clone());
        }
        let mut now = BTreeSet::new();
        for (file, def) in &defs {
            now.extend(def.reads.iter().copied());
            for call in callees(def) {
                now.extend(resolve(&call, file, &reads));
                for key in resolved_keys(&call, file) {
                    runs.entry(id.clone()).or_default().extend(reached_runs.get(&key).cloned().unwrap_or_default());
                }
                if std::env::var("ACTS_DEBUG").is_ok_and(|wanted| wanted == *id) {
                    eprintln!("{id} ({file}) calls {call}: {:?}", resolve(&call, file, &reads));
                }
            }
            runs.entry(id.clone()).or_default().extend(def.runs.iter().cloned());
        }
        commands.insert(id.clone(), now);
    }
    assert!(missing.is_empty(), "the scan finds no definition of {} commands: {}", missing.len(), missing.join(", "));
    // Commands that run other commands read what those read.
    loop {
        let mut changed = false;
        for id in ids {
            let mut now = commands[id].clone();
            for other in runs.get(id).into_iter().flatten() {
                if other == "*" {
                    now.extend(commands.values().flat_map(|reads| reads.iter().copied()));
                } else {
                    now.extend(commands.get(other).cloned().unwrap_or_default());
                }
            }
            if commands[id] != now {
                commands.insert(id.clone(), now);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    commands
}

/// The binding's own commands, which run no engine command the scan sees: `file.render` draws an
/// explicit view and reads no editor state; `file.exportMovie` renders the active comp from a copy,
/// so it reads the part.
const BINDING: &[&str] = &["file.render", "file.exportMovie"];

/// The editor state the scan finds, by command, without the kinds that are counted and not
/// declared.
fn declared_kinds(reads: &BTreeSet<&'static str>) -> BTreeSet<&'static str> {
    reads.iter().copied().filter(|read| ["selection", "part", "time"].contains(read)).collect()
}

fn engine_ids(doc: &MotionDocument) -> Vec<String> {
    doc.commands().into_iter().map(|command| command.id).filter(|id| !BINDING.contains(&id.as_str())).collect()
}

/// The reviewed differences between the declarations and the scan (`acts_known_differences.txt`
/// beside this file): every difference must be in it, word for word, and every entry must still
/// differ, so the file only shrinks as the audit after launch removes them.
const KNOWN: &str = include_str!("acts_known_differences.txt");
/// Why a difference is known: the scan does not look for `mode`, `active` and `area`; the code
/// reads what is declared through a path the scan does not follow (a function pointer, a stored
/// closure, a method of another type); the code reads less than is declared; the scan finds a read
/// the command does not make (a path it does not take, state it set itself, pruning).
const CATEGORIES: &[&str] = &["scanner-blind-kind", "scanner-cannot-follow", "over-declared", "scanner-false-read"];
/// The declared kinds the scan does not look for.
const BLIND: &[&str] = &["mode", "active", "area"];

/// A difference of command `.0` in direction `.1` (`declared-wider`, `scan-wider`,
/// `key-table-missing`, `key-table-extra`), written as the file writes it.
type Differences = BTreeMap<(String, String), String>;

fn listed(kinds: &BTreeSet<&str>) -> String {
    kinds.iter().copied().collect::<Vec<_>>().join(", ")
}

/// Checks `found` against the file's entries in `directions` and returns how many there are by
/// category, or what is wrong: a difference not in the file (or written otherwise), an entry that
/// no longer differs, an entry whose category does not fit its difference.
fn check_known(found: &Differences, directions: &[&str]) -> Result<BTreeMap<String, usize>, Vec<String>> {
    let mut wrong = Vec::new();
    let mut entries: BTreeMap<(String, String), (usize, String, String)> = BTreeMap::new();
    for (number, line) in KNOWN.lines().enumerate().map(|(at, line)| (at + 1, line.trim())) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split(" | ").map(str::trim).collect();
        let [command, difference, category, reason] = fields[..] else {
            wrong.push(format!("line {number}: not `command | difference | category | reason`: {line}"));
            continue;
        };
        let direction = difference.split(':').next().unwrap_or("").trim().to_owned();
        if !CATEGORIES.contains(&category) {
            wrong.push(format!("line {number}: unknown category `{category}`"));
        }
        let kinds: Vec<&str> = difference.split_once(':').map(|(_, kinds)| kinds.split(',').map(str::trim).collect()).unwrap_or_default();
        let fits = match (direction.as_str(), category) {
            ("declared-wider", "scanner-blind-kind") => kinds.iter().all(|kind| BLIND.contains(kind)),
            ("declared-wider", "scanner-cannot-follow" | "over-declared") => kinds.iter().all(|kind| !BLIND.contains(kind)),
            // The code reading more than is declared is never known: it is declared.
            ("scan-wider" | "key-table-missing", "scanner-false-read") => true,
            ("key-table-extra", "over-declared") => true,
            _ => false,
        };
        if !fits {
            wrong.push(format!("line {number}: `{category}` does not fit `{difference}`"));
        }
        let located = reason.split_whitespace().any(|word| word.contains(".rs:"));
        if matches!(category, "scanner-cannot-follow" | "scanner-false-read") && !located {
            wrong.push(format!("line {number}: a `{category}` reason names the code (`file.rs:line`): {reason}"));
        }
        if reason.is_empty() {
            wrong.push(format!("line {number}: no reason"));
        }
        if entries.insert((command.to_owned(), direction.clone()), (number, difference.to_owned(), category.to_owned())).is_some() {
            wrong.push(format!("line {number}: {command} has two `{direction}` entries"));
        }
    }
    let mut counts = BTreeMap::new();
    for ((command, direction), difference) in found {
        match entries.get(&(command.clone(), direction.clone())) {
            Some((_, known, category)) if known == difference => *counts.entry(category.clone()).or_insert(0) += 1,
            Some((number, known, _)) => wrong.push(format!("line {number}: {command} now differs as `{difference}`, the file says `{known}`")),
            None => wrong.push(format!("{command} | {difference}: not in the file (declare what the code reads, or review and add it)")),
        }
    }
    for ((command, direction), (number, difference, _)) in &entries {
        if directions.contains(&direction.as_str()) && !found.contains_key(&(command.clone(), direction.clone())) {
            wrong.push(format!("line {number}: {command} | {difference} no longer differs: remove the entry"));
        }
    }
    if wrong.is_empty() { Ok(counts) } else { Err(wrong) }
}

#[test]
fn known_differences_are_checked_strictly() {
    // A difference not in the file fails.
    let found = Differences::from([(("a.b".to_owned(), "scan-wider".to_owned()), "scan-wider: time".to_owned())]);
    let wrong = check_known(&found, &["scan-wider"]).unwrap_err();
    assert!(wrong.iter().any(|line| line.starts_with("a.b | scan-wider: time: not in the file")), "{wrong:?}");
    // An entry that no longer differs fails.
    let wrong = check_known(&Differences::new(), &["key-table-missing"]).unwrap_err();
    assert!(wrong.iter().any(|line| line.contains("edit.paste | key-table-missing no longer differs")), "{wrong:?}");
    // A difference written otherwise than the file says fails.
    let found = Differences::from([(("view.lookAtAll".to_owned(), "scan-wider".to_owned()), "scan-wider: selection, time".to_owned())]);
    let wrong = check_known(&found, &[]).unwrap_err();
    assert!(wrong.iter().any(|line| line.contains("view.lookAtAll now differs as `scan-wider: selection, time`")), "{wrong:?}");
}

#[test]
fn every_command_declares_the_editor_state_its_code_reads() {
    let doc = MotionDocument::create("a.kmotion", 64, 36).unwrap();
    let ids = engine_ids(&doc);
    let derived = derived(&ids);
    let mut found = Differences::new();
    for command in doc.commands() {
        let declared: BTreeSet<&str> = command.reads.iter().map(|state| state.as_str()).collect();
        let expected = match command.id.as_str() {
            "file.exportMovie" => BTreeSet::from(["part"]),
            id => derived.get(id).map(declared_kinds).unwrap_or_default(),
        };
        let wider: BTreeSet<&str> = declared.difference(&expected).copied().collect();
        let narrower: BTreeSet<&str> = expected.difference(&declared).copied().collect();
        if !wider.is_empty() {
            found.insert((command.id.clone(), "declared-wider".into()), format!("declared-wider: {}", listed(&wider)));
        }
        if !narrower.is_empty() {
            found.insert((command.id.clone(), "scan-wider".into()), format!("scan-wider: {}", listed(&narrower)));
        }
    }
    if std::env::var("ACTS_PRINT").is_ok() {
        let table: BTreeMap<&String, Vec<&str>> = derived
            .iter()
            .map(|(id, reads)| (id, declared_kinds(reads).into_iter().collect()))
            .filter(|(_, reads): &(&String, Vec<&str>)| !reads.is_empty())
            .collect();
        println!("{}", serde_json::to_string(&table).unwrap());
        let keys: Vec<&String> = derived.iter().filter(|(_, reads)| reads.contains("keys")).map(|(id, _)| id).collect();
        println!("keys {}", serde_json::to_string(&keys).unwrap());
        let project: Vec<&String> = derived.iter().filter(|(_, reads)| reads.contains("project")).map(|(id, _)| id).collect();
        println!("project {} {}", project.len(), serde_json::to_string(&project).unwrap());
        for ((command, _), difference) in &found {
            println!("difference {command} | {difference}");
        }
    }
    match check_known(&found, &["declared-wider", "scan-wider"]) {
        Ok(counts) => println!("{} known differences between the declared reads and the scan: {counts:?}", counts.values().sum::<usize>()),
        Err(wrong) => panic!("{} differences between the declared reads and the scan are wrong:\n{}", wrong.len(), wrong.join("\n")),
    }
}

/// The commands that read the keyframe selection are exactly the kit's table of them: they read
/// the selection, but `on` carries layer ids, and keyframes have none, so a run selects keyframes
/// with `keyframe.select` (by layer, property and time) in an earlier step.
#[test]
fn the_key_selection_table_is_what_the_code_reads() {
    let doc = MotionDocument::create("a.kmotion", 64, 36).unwrap();
    let ids = engine_ids(&doc);
    let derived = derived(&ids);
    let read: BTreeSet<&str> = derived.iter().filter(|(_, reads)| reads.contains("keys")).map(|(id, _)| id.as_str()).collect();
    let table: BTreeSet<&str> = ids.iter().map(String::as_str).filter(|id| kition_motion_kit::acts::reads_key_selection(id)).collect();
    let mut found = Differences::new();
    for id in read.difference(&table) {
        found.insert(((*id).to_owned(), "key-table-missing".into()), "key-table-missing".into());
    }
    for id in table.difference(&read) {
        found.insert(((*id).to_owned(), "key-table-extra".into()), "key-table-extra".into());
    }
    if std::env::var("ACTS_PRINT").is_ok() {
        for ((command, _), difference) in &found {
            println!("difference {command} | {difference}");
        }
    }
    match check_known(&found, &["key-table-missing", "key-table-extra"]) {
        Ok(counts) => println!("{} known differences between the key-selection table and the scan: {counts:?}", counts.values().sum::<usize>()),
        Err(wrong) => panic!("the key-selection table and the scan differ:\n{}", wrong.join("\n")),
    }
}

/// A command's `targets` is the `layers` (else `layer`) parameter its notation names, an id or a
/// list of ids, except where that parameter is no layer of a composition; every command passes
/// kition-sdk's command checks. With `ACTS_PRINT=1` it prints how many commands' code resolves a
/// number that matches no layer id as a stack position (the id-or-index fallback of
/// `resolve_layer`).
#[test]
fn targets_are_the_layer_parameters_the_notation_names() {
    let doc = MotionDocument::create("a.kmotion", 64, 36).unwrap();
    let commands = doc.commands();
    kition_kit::testing::check_commands(&commands, kition_motion_kit::descriptions::NO_OUTPUT).unwrap();
    if std::env::var("ACTS_PRINT").is_ok() {
        let ids = engine_ids(&doc);
        let derived = derived(&ids);
        println!("commands {}, targets {}", commands.len(), commands.iter().filter(|command| command.targets.is_some()).count());
        println!("fallback {}", ids.iter().filter(|id| derived.get(*id).is_some_and(|reads| reads.contains("layers"))).count());
    }
    let mut wrong = Vec::new();
    for command in &commands {
        let notation = command.params.as_deref().and_then(|params| kition_kit::schema::from_kit_params(params).ok()).map(|converted| converted.schema);
        let named = |name: &str| notation.as_ref().is_some_and(|notation| notation.get(name).is_some());
        let eligible = command
            .schema
            .as_ref()
            .and_then(|schema| ["layers", "layer"].into_iter().find(|name| named(name) && kition_kit::schema::check_targets(schema, name).is_ok()));
        let expected = eligible.filter(|_| !kition_motion_kit::acts::NOT_COMP_LAYERS.contains(&command.id.as_str()));
        if command.targets != expected {
            wrong.push(format!("{}: targets {:?}, its notation gives {expected:?}", command.id, command.targets));
        }
    }
    assert!(wrong.is_empty(), "{} commands:\n{}", wrong.len(), wrong.join("\n"));
    let with = |id: &str| commands.iter().find(|command| command.id == id).and_then(|command| command.targets);
    assert_eq!(with("prop.addKey"), Some("layer"));
    assert_eq!(with("edit.splitLayer"), Some("layers"));
    assert_eq!(with("file.import"), None);
    assert_eq!(with("comp.open"), None);
}
