//! Path normalization: raw SF paths -> repo-relative posix paths.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use crate::model::{FileCoverage, Snapshot, merge_file_coverage};

/// `os.path.realpath`, non-strict: symlinks resolved, missing tails kept as they are.
pub fn realpath(path: &str) -> String {
    let p = Path::new(path);
    let mut out = if p.is_absolute() {
        PathBuf::from("/")
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"))
    };
    for c in p.components() {
        match c {
            Component::RootDir | Component::Prefix(_) | Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(name) => {
                out.push(name);
                if out.is_symlink()
                    && let Ok(real) = out.canonicalize()
                {
                    out = real;
                }
            }
        }
    }
    out.to_string_lossy().into_owned()
}

/// (anchor, parts); anchor is "" (relative), "/" or a drive like "C:/".
type Split = (String, Vec<String>);

/// Lexically clean a path: backslashes to '/', drop '.', resolve '..'.
fn split(path: &str) -> Split {
    let p = path.replace('\\', "/");
    let b = p.as_bytes();
    let (anchor, rest) = if b.len() >= 2
        && b[0].is_ascii_alphabetic()
        && b[1] == b':'
        && (b.len() == 2 || b[2] == b'/')
    {
        (
            format!("{}:/", (b[0] as char).to_ascii_uppercase()),
            &p[2..],
        )
    } else if p.starts_with('/') {
        ("/".to_string(), p.as_str())
    } else {
        (String::new(), p.as_str())
    };
    let mut parts: Vec<String> = Vec::new();
    for part in rest.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.last().is_some_and(|l| l != "..") {
                    parts.pop();
                } else if anchor.is_empty() {
                    // Escapes the tree; '..' above an absolute root is dropped.
                    parts.push("..".into());
                }
            }
            _ => parts.push(part.to_string()),
        }
    }
    (anchor, parts)
}

fn join((anchor, parts): &Split) -> String {
    if anchor.is_empty() && parts.is_empty() {
        ".".into()
    } else {
        format!("{anchor}{}", parts.join("/"))
    }
}

/// Clean roots, add realpaths of absolute ones, longest first.
fn prepare<S: AsRef<str>>(roots: &[S]) -> Vec<Split> {
    let mut out: BTreeSet<Split> = BTreeSet::new();
    for root in roots {
        let root = root.as_ref();
        let s = split(root);
        if s.0.is_empty() && s.1.is_empty() {
            continue;
        }
        out.insert(s);
        if root.starts_with('/') {
            out.insert(split(&realpath(root)));
        }
    }
    let mut v: Vec<Split> = out.into_iter().collect();
    v.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.cmp(b)));
    v
}

/// The path below the longest matching root, matching on whole components.
fn strip(s: &Split, roots: &[Split]) -> Option<String> {
    let (anchor, parts) = s;
    roots
        .iter()
        .find(|(ra, rp)| ra == anchor && parts.len() > rp.len() && parts.starts_with(rp))
        .map(|(_, rp)| parts[rp.len()..].join("/"))
}

fn normalize(raw: &str, roots: &[Split]) -> (String, bool) {
    let s = split(raw);
    let cleaned = join(&s);
    let mut rest = strip(&s, roots);
    if rest.is_none() && cleaned.starts_with('/') {
        // /tmp vs /private/tmp and the like.
        rest = strip(&split(&realpath(&cleaned)), roots);
    }
    if let Some(rest) = rest {
        return (rest, false);
    }
    let (anchor, parts) = &s;
    let inside = anchor.is_empty() && parts.first().is_some_and(|p| p != "..");
    (cleaned, !inside)
}

/// Return (path, external). Strips the longest matching root; external if unresolvable.
pub fn normalize_path<S: AsRef<str>>(raw: &str, roots: &[S]) -> (String, bool) {
    normalize(raw, &prepare(roots))
}

/// Re-key a Snapshot by normalized path, merging records that collide.
pub fn normalize_snapshot<S: AsRef<str>>(snapshot: &Snapshot, roots: &[S]) -> Snapshot {
    let prepared = prepare(roots);
    let mut files: BTreeMap<String, FileCoverage> = BTreeMap::new();
    for (raw, fc) in &snapshot.files {
        let (path, external) = normalize(raw, &prepared);
        let fc = FileCoverage {
            path: path.clone(),
            external,
            ..fc.clone()
        };
        let merged = match files.remove(&path) {
            None => fc,
            Some(old) => merge_file_coverage(&old, &fc),
        };
        files.insert(path, merged);
    }
    Snapshot {
        files,
        test_names: snapshot.test_names.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{BranchKey, FunctionRecord};

    #[test]
    fn normalize_paths() {
        let cases: &[(&str, &[&str], &str, bool)] = &[
            ("/repo/src/a.py", &["/repo"], "src/a.py", false),
            // longest root wins, regardless of order
            (
                "/repo/sub/src/a.py",
                &["/repo", "/repo/sub"],
                "src/a.py",
                false,
            ),
            (
                "/repo/sub/src/a.py",
                &["/repo/sub", "/repo"],
                "src/a.py",
                false,
            ),
            // component boundary only
            ("/repo2/a.py", &["/repo"], "/repo2/a.py", true),
            ("/ci/build_x/a.rs", &["/ci/build"], "/ci/build_x/a.rs", true),
            // strip-prefix with and without trailing slash
            ("/ci/build/src/a.rs", &["/ci/build/"], "src/a.rs", false),
            ("/ci/build/src/a.rs", &["/ci/build"], "src/a.rs", false),
            ("shed_py/shed/calc.py", &["shed_py/"], "shed/calc.py", false),
            ("shed_py/shed/calc.py", &["shed_py"], "shed/calc.py", false),
            // relative stays relative, './' dropped
            ("shed/calc.py", &["/repo"], "shed/calc.py", false),
            ("./shed/calc.py", &[], "shed/calc.py", false),
            (".\\shed\\calc.py", &[], "shed/calc.py", false),
            // '..' resolved lexically
            ("/repo/tests/../src/a.py", &["/repo"], "src/a.py", false),
            ("src/../shed/a.py", &[], "shed/a.py", false),
            ("/../repo/a.py", &["/repo"], "a.py", false),
            ("/repo//src///a.py", &["/repo/"], "src/a.py", false),
            // backslashes and drive letters
            (
                "C:\\ci\\repo\\src\\lib.rs",
                &["C:\\ci\\repo"],
                "src/lib.rs",
                false,
            ),
            (
                "c:\\ci\\repo\\src\\lib.rs",
                &["C:/ci/repo/"],
                "src/lib.rs",
                false,
            ),
            ("src\\lib.rs", &[], "src/lib.rs", false),
            ("D:\\deps\\x.rs", &["C:/ci/repo"], "D:/deps/x.rs", true),
            ("C:/x.rs", &["/x"], "C:/x.rs", true),
            ("cd:/x", &[], "cd:/x", false),
            // external absolute: key is the cleaned path
            (
                "/usr/lib/python3.12/json/__init__.py",
                &["/repo"],
                "/usr/lib/python3.12/json/__init__.py",
                true,
            ),
            ("/opt/../opt/lib/x.py", &[], "/opt/lib/x.py", true),
            ("/repo", &["/repo"], "/repo", true),
            // escaping relative
            ("../other/x.py", &["/repo"], "../other/x.py", true),
            ("src/../../x.py", &[], "../x.py", true),
            // empty roots are ignored
            ("/repo/a.py", &["", ".", "./"], "/repo/a.py", true),
        ];
        for &(raw, roots, path, external) in cases {
            assert_eq!(
                normalize_path(raw, roots),
                (path.to_string(), external),
                "{raw}"
            );
        }
    }

    #[test]
    fn symlinked_root() {
        let tmp = tempfile::tempdir().unwrap();
        let base = realpath(tmp.path().to_str().unwrap());
        std::fs::create_dir_all(format!("{base}/real/src")).unwrap();
        std::os::unix::fs::symlink(format!("{base}/real"), format!("{base}/link")).unwrap();
        let (real, link) = (format!("{base}/real"), format!("{base}/link"));
        // Root via the symlink, SF via the real path (llvm-cov resolves symlinks), and back.
        let ok = ("src/a.rs".to_string(), false);
        assert_eq!(normalize_path(&format!("{real}/src/a.rs"), &[&link]), ok);
        assert_eq!(normalize_path(&format!("{link}/src/a.rs"), &[&real]), ok);
        let elsewhere = format!("{base}/elsewhere.rs");
        assert_eq!(
            normalize_path(&elsewhere, &[&link]),
            (elsewhere.clone(), true)
        );
    }

    #[test]
    fn realpath_follows_links() {
        let tmp = tempfile::tempdir().unwrap();
        let base = realpath(tmp.path().to_str().unwrap());
        std::fs::create_dir_all(format!("{base}/real/sub")).unwrap();
        std::os::unix::fs::symlink(format!("{base}/real"), format!("{base}/link")).unwrap();
        std::os::unix::fs::symlink("loop", format!("{base}/loop")).unwrap();
        let rp = |p: &str| realpath(&format!("{base}/{p}"));
        assert_eq!(
            rp("link/sub/missing/x"),
            format!("{base}/real/sub/missing/x")
        );
        assert_eq!(rp("link/../link/sub"), format!("{base}/real/sub"));
        assert_eq!(rp("loop/x"), format!("{base}/loop/x"));
        assert_eq!(realpath("/"), "/");
        assert_eq!(realpath("//a/../.."), "/");
    }

    fn fc(path: &str, lines: &[(i64, i128)], functions: Vec<FunctionRecord>) -> FileCoverage {
        FileCoverage {
            path: path.into(),
            lines: lines.iter().copied().collect(),
            functions,
            ..Default::default()
        }
    }

    #[test]
    fn snapshot_merges_collisions() {
        let f = |hits| FunctionRecord {
            name: "f".into(),
            start: 1,
            end: Some(3),
            hits: Some(hits),
        };
        let snap = Snapshot {
            files: [
                fc("/repo/shed/calc.py", &[(5, 1), (1, 1)], vec![f(1)]),
                fc("./shed/calc.py", &[(5, 2), (2, 0)], vec![f(2)]),
                fc("/usr/lib/x.py", &[(1, 1)], vec![]),
                fc("../up.py", &[(1, 0)], vec![]),
            ]
            .into_iter()
            .map(|fc| (fc.path.clone(), fc))
            .collect(),
            test_names: vec!["t1".into(), "t2".into()],
        };
        let out = normalize_snapshot(&snap, &["/repo"]);
        assert_eq!(out.test_names, ["t1", "t2"]);
        let keys: Vec<_> = out.files.keys().collect();
        assert_eq!(keys, ["../up.py", "/usr/lib/x.py", "shed/calc.py"]);
        let calc = &out.files["shed/calc.py"];
        assert_eq!(calc.path, "shed/calc.py");
        assert!(!calc.external);
        assert_eq!(calc.lines, [(1, 1), (2, 0), (5, 3)].into_iter().collect());
        assert_eq!(calc.functions, [f(3)]);
        assert!(out.files["/usr/lib/x.py"].external);
        assert!(out.files["../up.py"].external);
    }

    #[test]
    fn snapshot_merges_branches() {
        let key = BranchKey {
            line: 2,
            block: "0".into(),
            branch: "0".into(),
        };
        let mut a = fc("/r/x.c", &[], vec![]);
        a.branches.insert(key.clone(), None);
        let mut b = fc("x.c", &[], vec![]);
        b.branches.insert(key.clone(), Some(4));
        let snap = Snapshot {
            files: [("/r/x.c".into(), a), ("x.c".into(), b)]
                .into_iter()
                .collect(),
            test_names: vec![],
        };
        let out = normalize_snapshot(&snap, &["/r"]);
        assert_eq!(out.files["x.c"].branches[&key], Some(4));
    }

    #[test]
    fn fixtures() {
        let load = |name: &str| {
            let path = format!("{}/tests/fixtures/lcov/{name}", env!("CARGO_MANIFEST_DIR"));
            crate::lcov::parse(&std::fs::read_to_string(path).unwrap()).unwrap()
        };
        let snap = load("llvm_cov.lcov");
        let out = normalize_snapshot(&snap, &["/home/runner/work/shed/shed_rs/"]);
        assert_eq!(
            out.files.keys().collect::<Vec<_>>(),
            ["src/lib.rs", "src/util.rs"]
        );
        assert!(out.files.values().all(|fc| !fc.external));
        let raw = &snap.files["/home/runner/work/shed/shed_rs/src/lib.rs"];
        assert_eq!(out.files["src/lib.rs"].lines, raw.lines);
        assert_eq!(out.files["src/lib.rs"].functions, raw.functions);
        // Idempotent.
        let once = normalize_snapshot(&load("tarpaulin.lcov"), &["/builds/shed/shed_rs"]);
        assert_eq!(once.files.keys().collect::<Vec<_>>(), ["src/lib.rs"]);
        assert_eq!(normalize_snapshot(&once, &["/builds/shed/shed_rs"]), once);
    }
}
