//! The Python language pack's Rust side: text edits for suggestions on Python IR.
//!
//! Duplicate functions: `nodes[0]` is kept and every other copy is replaced in place, so its
//! name stays bound where it was:
//!
//! - Same module, same parameter names, kept copy defined earlier: `copy = kept`. An alias keeps
//!   the name, keyword calls and defaults; only `copy.__name__` changes.
//! - Other module, same parameter names: `from <module> import kept as copy`.
//! - Parameter names differ, or the kept copy comes later in the same module: a thin wrapper with
//!   the copy's own header that calls the kept function (`return kept(a, *b, c=c, **d)`), so
//!   keyword calls keep working. In another module the import goes inside the wrapper, which
//!   avoids import cycles at load time.
//! - Same name in the same module (a redefinition): the later copy is removed.
//!
//! Anything that can't be replaced safely gives no edits and a reason: decorators (they run at
//! definition time), methods and nested functions, async generators, and, across modules, code
//! that refers to module globals (they may differ between the modules).

mod header;

use std::collections::BTreeMap;
use std::path::Path;

use sundgren_analyze::Suggestion;
use sundgren_ir::{File, FileEdit, Kind, Node, TextEdit, sha256_hex};

use header::{ParamKind, header_end, params};

/// Builtins whose meaning doesn't depend on the module they're used in.
const BUILTINS: &[&str] = &[
    "ArithmeticError",
    "AssertionError",
    "AttributeError",
    "BaseException",
    "BaseExceptionGroup",
    "BlockingIOError",
    "BrokenPipeError",
    "BufferError",
    "BytesWarning",
    "ChildProcessError",
    "ConnectionAbortedError",
    "ConnectionError",
    "ConnectionRefusedError",
    "ConnectionResetError",
    "DeprecationWarning",
    "EOFError",
    "Ellipsis",
    "EncodingWarning",
    "EnvironmentError",
    "Exception",
    "ExceptionGroup",
    "False",
    "FileExistsError",
    "FileNotFoundError",
    "FloatingPointError",
    "FutureWarning",
    "GeneratorExit",
    "IOError",
    "ImportError",
    "ImportWarning",
    "IndentationError",
    "IndexError",
    "InterruptedError",
    "IsADirectoryError",
    "KeyError",
    "KeyboardInterrupt",
    "LookupError",
    "MemoryError",
    "ModuleNotFoundError",
    "NameError",
    "None",
    "NotADirectoryError",
    "NotImplemented",
    "NotImplementedError",
    "OSError",
    "OverflowError",
    "PendingDeprecationWarning",
    "PermissionError",
    "ProcessLookupError",
    "RecursionError",
    "ReferenceError",
    "ResourceWarning",
    "RuntimeError",
    "RuntimeWarning",
    "StopAsyncIteration",
    "StopIteration",
    "SyntaxError",
    "SyntaxWarning",
    "SystemError",
    "SystemExit",
    "TabError",
    "TimeoutError",
    "True",
    "TypeError",
    "UnboundLocalError",
    "UnicodeDecodeError",
    "UnicodeEncodeError",
    "UnicodeError",
    "UnicodeTranslateError",
    "UnicodeWarning",
    "UserWarning",
    "ValueError",
    "Warning",
    "ZeroDivisionError",
    "abs",
    "aiter",
    "all",
    "anext",
    "any",
    "ascii",
    "bin",
    "bool",
    "breakpoint",
    "bytearray",
    "bytes",
    "callable",
    "chr",
    "classmethod",
    "compile",
    "complex",
    "delattr",
    "dict",
    "dir",
    "divmod",
    "enumerate",
    "filter",
    "float",
    "format",
    "frozenset",
    "getattr",
    "hasattr",
    "hash",
    "hex",
    "id",
    "input",
    "int",
    "isinstance",
    "issubclass",
    "iter",
    "len",
    "list",
    "map",
    "max",
    "memoryview",
    "min",
    "next",
    "object",
    "oct",
    "open",
    "ord",
    "pow",
    "print",
    "property",
    "range",
    "repr",
    "reversed",
    "round",
    "set",
    "setattr",
    "slice",
    "sorted",
    "staticmethod",
    "str",
    "sum",
    "super",
    "tuple",
    "type",
    "zip",
];

/// Fill `edits` for every Python duplicate-function suggestion, or append why there are none.
pub fn add_duplicate_edits(suggestions: &mut [Suggestion], files: &[File], root: &Path) {
    for s in suggestions
        .iter_mut()
        .filter(|s| s.analysis == sundgren_analyze::DUPLICATE_FUNCTION)
    {
        match duplicate_edits(s, files, root) {
            Ok(edits) => s.edits = Some(edits),
            Err(why) => s.message.push_str(&format!(" No edits: {why}.")),
        }
    }
}

struct Copy<'a> {
    file: &'a File,
    node: &'a Node,
    source: Vec<u8>,
}

impl Copy<'_> {
    fn name(&self) -> &str {
        self.node.name.as_deref().unwrap_or("")
    }

    fn header(&self) -> Result<&[u8], String> {
        let start = self.node.span.start.byte;
        let end = header_end(&self.source, start)
            .filter(|&e| e <= self.node.span.end.byte)
            .ok_or_else(|| format!("can't find the header of {}", self.node.id))?;
        Ok(&self.source[start..end])
    }
}

/// Edits that keep `nodes[0]` and replace every other copy, grouped by file.
pub fn duplicate_edits(
    s: &Suggestion,
    files: &[File],
    root: &Path,
) -> Result<Vec<FileEdit>, String> {
    let copies = s
        .nodes
        .iter()
        .map(|id| {
            let file = files
                .iter()
                .find(|f| f.node(id).is_some())
                .ok_or_else(|| format!("{id} is not in the IR"))?;
            if file.language != "python" {
                return Err(format!("{} is not Python", file.path));
            }
            let source = std::fs::read(root.join(&file.path))
                .map_err(|e| format!("can't read {}: {e}", file.path))?;
            if sha256_hex(&source) != file.source_sha256 {
                return Err(format!("{} changed since its IR was built", file.path));
            }
            let node = file.node(id).expect("found above");
            Ok(Copy { file, node, source })
        })
        .collect::<Result<Vec<_>, String>>()?;
    for c in &copies {
        check_replaceable(c)?;
    }
    let (kept, others) = copies.split_first().ok_or("no copies")?;
    let mut by_file: BTreeMap<&str, Vec<TextEdit>> = BTreeMap::new();
    for c in others {
        let edit = replace(kept, c, root)?;
        by_file.entry(c.file.path.as_str()).or_default().push(edit);
    }
    Ok(by_file
        .into_iter()
        .map(|(path, mut edits)| {
            edits.sort_by_key(|e| e.range.start);
            FileEdit {
                path: path.into(),
                edits,
            }
        })
        .collect())
}

fn check_replaceable(c: &Copy<'_>) -> Result<(), String> {
    let n = c.node;
    let root = &c.file.nodes[0].id;
    if n.kind != Kind::Function || n.parent.as_ref() != Some(root) {
        return Err(format!("{} is not a module-level function", n.id));
    }
    if c.source.get(n.span.start.byte) == Some(&b'@') {
        return Err(format!(
            "{} is decorated, and decorators run at definition time",
            n.id
        ));
    }
    if is_async(c)? && subtree(c.file, n).any(|m| label(m).starts_with("Yield")) {
        return Err(format!("{} is an async generator", n.id));
    }
    Ok(())
}

fn is_async(c: &Copy<'_>) -> Result<bool, String> {
    Ok(c.header()?.starts_with(b"async"))
}

fn label(n: &Node) -> &str {
    n.l1.as_ref().map_or("", |l| l.label.as_str())
}

/// The node and its descendants (pre-order makes them contiguous).
fn subtree<'a>(file: &'a File, node: &'a Node) -> impl Iterator<Item = &'a Node> + 'a {
    let start = file
        .nodes
        .iter()
        .position(|n| n.id == node.id)
        .expect("node in file");
    file.nodes[start..]
        .iter()
        .take_while(move |m| m.id == node.id || node.span.contains(&m.span))
}

/// Module globals the function refers to; needs L1.
fn globals(c: &Copy<'_>) -> Result<Vec<String>, String> {
    if !c.file.has_l1() {
        return Err(format!("{} has no L1 to check its globals", c.file.path));
    }
    let mut out = Vec::new();
    for m in subtree(c.file, c.node) {
        let l = label(m);
        if l.starts_with("Global") || l.starts_with("Nonlocal") {
            return Err(format!("{} declares global or nonlocal names", c.node.id));
        }
        if let Some(name) = l.strip_prefix("Name:")
            && !BUILTINS.contains(&name)
            && !out.iter().any(|o| o == name)
        {
            out.push(name.to_string());
        }
    }
    Ok(out)
}

/// The text edit replacing copy `c` by a use of `kept`.
fn replace(kept: &Copy<'_>, c: &Copy<'_>, root: &Path) -> Result<TextEdit, String> {
    let range = c.node.span.bytes();
    let same_file = kept.file.path == c.file.path;
    let kept_first = kept.node.span.end.byte <= c.node.span.start.byte;
    let (k, name) = (kept.name(), c.name());
    if same_file && k == name {
        if !kept_first {
            return Err(format!("{} is redefined before the kept copy", c.node.id));
        }
        return Ok(TextEdit {
            range,
            text: Vec::new(),
        });
    }
    if !same_file {
        let g = globals(c)?;
        if !g.is_empty() {
            return Err(format!(
                "{} refers to module globals ({}), which may differ between modules",
                c.node.id,
                g.join(", ")
            ));
        }
    }
    let kp = params(kept.header()?).ok_or("can't parse the kept parameters")?;
    let cp =
        params(c.header()?).ok_or_else(|| format!("can't parse {}'s parameters", c.node.id))?;
    if kp.len() != cp.len() || kp.iter().zip(&cp).any(|(a, b)| a.kind != b.kind) {
        return Err(format!("{} has other parameter kinds", c.node.id));
    }
    let same_params = kp.iter().zip(&cp).all(|(a, b)| a.name == b.name);
    let import = if same_file {
        None
    } else {
        Some(import_path(root, &kept.file.path, &c.file.path)?)
    };
    let text = match &import {
        _ if same_file && same_params && kept_first => format!("{name} = {k}"),
        Some(module) if same_params => {
            if k == name {
                format!("from {module} import {k}")
            } else {
                format!("from {module} import {k} as {name}")
            }
        }
        _ => {
            if cp.iter().any(|p| p.name == k) {
                return Err(format!("a parameter of {} shadows {k}", c.node.id));
            }
            let args: Vec<String> = cp
                .iter()
                .zip(&kp)
                .map(|(p, q)| match p.kind {
                    ParamKind::Positional => p.name.clone(),
                    ParamKind::VarArgs => format!("*{}", p.name),
                    ParamKind::KwOnly => format!("{}={}", q.name, p.name),
                    ParamKind::KwArgs => format!("**{}", p.name),
                })
                .collect();
            let header = String::from_utf8_lossy(c.header()?).into_owned();
            let mut body = String::new();
            if let Some(module) = &import {
                body.push_str(&format!("    from {module} import {k}\n"));
            }
            let call = if is_async(c)? { "await " } else { "" };
            body.push_str(&format!("    return {call}{k}({})", args.join(", ")));
            format!("{header}\n{body}")
        }
    };
    Ok(TextEdit {
        range,
        text: text.into_bytes(),
    })
}

/// How module `copy_path` imports module `kept_path`: relative within one package, absolute
/// across packages (found by walking up `__init__.py` files under `root`).
fn import_path(root: &Path, kept_path: &str, copy_path: &str) -> Result<String, String> {
    let split = |p: &str| -> (Vec<String>, String) {
        let mut parts: Vec<String> = p.split('/').map(String::from).collect();
        let file = parts.pop().unwrap_or_default();
        (parts, file.strip_suffix(".py").unwrap_or(&file).to_string())
    };
    let (kdir, kstem) = split(kept_path);
    let (cdir, _) = split(copy_path);
    let is_pkg = |dir: &[String]| {
        let mut p = root.to_path_buf();
        p.extend(dir);
        p.join("__init__.py").is_file()
    };
    let ident = |s: &str| {
        s.chars()
            .next()
            .is_some_and(|c| c.is_alphabetic() || c == '_')
            && s.chars().all(|c| c.is_alphanumeric() || c == '_')
    };
    if kdir == cdir {
        return Ok(match (is_pkg(&kdir), kstem.as_str()) {
            (true, "__init__") => ".".into(),
            (true, stem) if ident(stem) => format!(".{stem}"),
            (false, stem) if ident(stem) => stem.into(),
            _ => return Err(format!("can't import {kept_path}")),
        });
    }
    if !is_pkg(&kdir) {
        return Err(format!(
            "can't tell how {copy_path} imports {kept_path} (no package)"
        ));
    }
    // The package chain: the longest run of package directories ending at the kept file's.
    let mut first = kdir.len();
    while first > 0 && is_pkg(&kdir[..first - 1]) {
        first -= 1;
    }
    let mut module: Vec<&str> = kdir[first..].iter().map(String::as_str).collect();
    if kstem != "__init__" {
        module.push(&kstem);
    }
    if module.iter().all(|m| ident(m)) {
        Ok(module.join("."))
    } else {
        Err(format!("can't import {kept_path}"))
    }
}
