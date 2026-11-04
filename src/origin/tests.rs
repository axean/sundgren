use super::*;
use crate::test_support::{TempDir, commit_file, git};

/// A function of `count` distinct lines, named `name`.
fn function(name: &str, count: usize) -> String {
    let body: String = (0..count)
        .map(|i| format!("    {name}_{i} = measure_{name}(channel={i}, gain=0.{i})\n"))
        .collect();
    format!("def {name}():\n{body}\n")
}

const HEADER: &str = "# Part of the sensorlink project.\n";

/// The other repository: at `v1`, `big.py` holds alpha, beta and gamma, and `noise.py` other
/// code; then main splits big.py: alpha in `alpha.py`, gamma rewritten.
fn other(tmp: &TempDir) -> PathBuf {
    let d = tmp.0.join("other");
    std::fs::create_dir_all(&d).unwrap();
    git(&d, &["init", "-q", "-b", "main"]);
    let big = format!(
        "{HEADER}{}{}{}",
        function("alpha", 10),
        function("beta", 10),
        function("gamma", 10)
    );
    commit_file(&d, "big.py", &big, "big");
    commit_file(&d, "noise.py", &function("delta", 10), "noise");
    commit_file(&d, "notes.txt", &function("alpha", 10), "notes");
    git(&d, &["tag", "v1"]);
    git(&d, &["rm", "-q", "big.py"]);
    commit_file(&d, "alpha.py", &function("alpha", 10), "split");
    commit_file(&d, "gamma.py", &function("gamma2", 10), "rewrite");
    d
}

/// This repository, with `path` holding `text`.
fn this(tmp: &TempDir, path: &str, text: &str) -> PathBuf {
    let d = tmp.0.join("this");
    std::fs::create_dir_all(&d).unwrap();
    git(&d, &["init", "-q", "-b", "main"]);
    commit_file(&d, path, text, "start");
    d.join(path)
}

fn document(file: &Path, other: &Path, revs: &[&str], options: Options) -> OriginDocument {
    let revs: Vec<String> = revs.iter().map(|&r| r.to_owned()).collect();
    let repo = file.parent().unwrap();
    origin_document(repo, &[file.to_path_buf()], other, &revs, options).unwrap()
}

#[test]
fn a_copy_of_two_functions_is_contained_in_the_bigger_file() {
    let tmp = TempDir::new();
    let other = other(&tmp);
    let text = format!("{HEADER}{}{}", function("alpha", 10), function("gamma", 10));
    let file = this(&tmp, "copied.py", &text);
    let doc = document(&file, &other, &["v1", "main"], Options::new(2.0));
    assert_eq!(doc.schema, ORIGIN_SCHEMA);
    let copied = &doc.files[0];
    assert_eq!(copied.path, "copied.py");
    // The header is a comment: left out.
    assert_eq!((copied.lines, copied.comment_lines), (22, 1));

    let at_v1 = &copied.origins[0];
    assert_eq!(at_v1.rev, "v1");
    let best = at_v1.best.as_ref().unwrap();
    assert_eq!(best.path, "big.py");
    assert_eq!((best.exact, best.fuzzy, best.share), (22, 0, 1.0));
    // Only big.py has gamma: alpha is also in notes.txt, which is not a .py file.
    assert_eq!(best.distinctive, 1.0);
    assert_eq!(at_v1.second, None);
    assert_eq!(at_v1.gap, Some(1.0));

    // On main, big.py is gone and gamma rewritten: half of it is in alpha.py.
    let today = copied.origins[1].best.as_ref().unwrap();
    assert_eq!(today.path, "alpha.py");
    assert_eq!(today.share, 0.5);
    assert_eq!(doc.references.len(), 2);
    assert_eq!(doc.references[0].files, 2);
    assert_eq!(doc.references[1].files, 3);
}

#[test]
fn boilerplate_found_everywhere_leaves_no_clear_winner() {
    let tmp = TempDir::new();
    let other = tmp.0.join("other");
    std::fs::create_dir_all(&other).unwrap();
    git(&other, &["init", "-q", "-b", "main"]);
    let common = "import logging\nlog = logging.getLogger(__name__)\n";
    for name in ["one", "two", "three"] {
        commit_file(
            &other,
            &format!("{name}.py"),
            &format!("{common}{}", function(name, 5)),
            name,
        );
    }
    let file = this(&tmp, "mine.py", &format!("{common}{}", function("mine", 2)));
    let doc = document(&file, &other, &["main"], Options::new(2.0));
    let origin = &doc.files[0].origins[0];
    let best = origin.best.as_ref().unwrap();
    assert_eq!(best.share, 0.4);
    assert_eq!(best.distinctive, 0.0);
    assert_eq!(origin.gap, Some(0.0));
}

#[test]
fn edited_lines_are_found_fuzzily_and_listed() {
    let tmp = TempDir::new();
    let other = other(&tmp);
    let edited = function("beta", 10).replace("gain=", "gain = ");
    let file = this(&tmp, "beta.py", &format!("{HEADER}{edited}"));
    let options = Options {
        lines: true,
        ..Options::new(0.9)
    };
    let doc = document(&file, &other, &["v1"], options);
    let origin = &doc.files[0].origins[0];
    let best = origin.best.as_ref().unwrap();
    assert_eq!(
        (best.path.as_str(), best.exact, best.fuzzy),
        ("big.py", 1, 10)
    );
    let lines = origin.lines_found.as_ref().unwrap();
    assert_eq!(lines.len(), 12);
    assert_eq!(lines[0].found, "comment");
    assert_eq!(lines[1].found, "exact");
    assert_eq!(
        lines[1].at,
        Some(At {
            path: "big.py".to_owned(),
            line: 14
        })
    );
    assert_eq!(lines[2].found, "fuzzy");
    assert!(lines[2].edits.is_some());

    // Exact only: the def line alone.
    let doc = document(&file, &other, &["v1"], Options::new(2.0));
    let best = doc.files[0].origins[0].best.as_ref().unwrap();
    assert_eq!((best.exact, best.fuzzy), (1, 0));
    assert!(doc.files[0].origins[0].lines_found.is_none());
}

#[test]
fn nothing_shared_has_no_origin() {
    let tmp = TempDir::new();
    let other = other(&tmp);
    let file = this(&tmp, "new.py", &function("epsilon", 3));
    let doc = document(&file, &other, &["v1"], Options::new(0.9));
    let origin = &doc.files[0].origins[0];
    assert!(origin.best.is_none() && origin.gap.is_none());
    assert_eq!(
        text(&doc).lines().nth(1),
        Some(format!("  v1 ({}): no file shares a line", &origin.oid[..8]).as_str())
    );
}

#[test]
fn reports_as_text() {
    let tmp = TempDir::new();
    let other = other(&tmp);
    let text_of = format!("{}{}", function("alpha", 10), function("delta", 2));
    let file = this(&tmp, "mixed.py", &text_of);
    let doc = document(&file, &other, &["v1"], Options::new(2.0));
    let out = text(&doc);
    let oid = &doc.references[0].oid[..8];
    assert_eq!(
        out,
        format!(
            "mixed.py: 14 lines\n  v1 ({oid}): 79% from big.py (79% distinctive), ahead by 57% \
             of noise.py (21%)\n"
        )
    );
}

#[test]
fn bad_files_and_revisions_are_usage_errors() {
    let tmp = TempDir::new();
    let other = other(&tmp);
    let file = this(&tmp, "a.py", "x = 1\n");
    let repo = file.parent().unwrap();
    let run = |files: &[PathBuf], revs: &[&str]| {
        let revs: Vec<String> = revs.iter().map(|&r| r.to_owned()).collect();
        origin_document(repo, files, &other, &revs, Options::new(0.9)).unwrap_err()
    };
    assert_eq!(
        run(std::slice::from_ref(&file), &["nope"]).code,
        crate::EXIT_USAGE
    );
    let outside = other.join("noise.py");
    let err = run(&[outside], &["v1"]);
    assert_eq!(err.code, crate::EXIT_USAGE);
    assert!(err.msg.contains("is not in the repository"), "{}", err.msg);
    assert_eq!(
        run(&[repo.join("missing.py")], &["v1"]).code,
        crate::EXIT_USAGE
    );
}

#[test]
fn kinds_are_extensions() {
    assert_eq!(kind("a/b.py"), "py");
    assert_eq!(kind("a.d/Makefile"), "Makefile");
    assert_eq!(kind("x.tar.gz"), "gz");
}
