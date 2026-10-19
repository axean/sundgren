use super::*;
use crate::EXIT_USAGE;

#[test]
fn utc_format() {
    assert_eq!(format_utc(0), "1970-01-01T00:00:00Z");
    assert_eq!(format_utc(951_782_400), "2000-02-29T00:00:00Z");
    assert_eq!(format_utc(1_792_454_399), "2026-10-19T23:59:59Z");
}

#[test]
fn env_parsing() {
    let items = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let want = [("A", "2"), ("B", "x=y"), ("C", "")].map(|(k, v)| (k.into(), v.into()));
    assert_eq!(
        parse_env(&items(&["A=1", "B=x=y", "A=2", "C="])).unwrap(),
        want
    );
    assert_eq!(parse_env(&items(&["=1"])).unwrap_err().code, EXIT_USAGE);
    assert_eq!(parse_env(&items(&["NOEQ"])).unwrap_err().code, EXIT_USAGE);
}

#[test]
fn default_preset_by_markers() {
    let dir = tempfile::tempdir().unwrap();
    let dir = dir.path();
    assert_eq!(default_preset_name(dir).unwrap_err().code, EXIT_USAGE);
    std::fs::write(dir.join("pytest.ini"), "").unwrap();
    assert_eq!(default_preset_name(dir).unwrap(), "pytest");
    std::fs::write(dir.join("Cargo.toml"), "").unwrap();
    assert!(default_preset_name(dir).is_err());
    std::fs::remove_file(dir.join("pytest.ini")).unwrap();
    assert_eq!(default_preset_name(dir).unwrap(), "cargo-llvm-cov");
}

#[test]
fn cli_parses_like_argparse() {
    let cli = Cli::try_parse_from([
        "sundgren", "run", "--patch", "p", "--tests", "a", "b", "--env", "K=V", "--repe", "2",
        "--keep",
    ])
    .unwrap();
    let Command::Run(r) = cli.command else {
        panic!()
    };
    assert_eq!(r.tests, ["a", "b"]);
    assert_eq!(r.repeat, 2);
    assert!(r.keep_worktrees);
    assert_eq!(r.output.format, Format::Json);
    let err = Cli::try_parse_from([
        "sundgren", "run", "--patch", "p", "--preset", "x", "--cmd", "y",
    ]);
    assert_eq!(err.unwrap_err().exit_code(), EXIT_USAGE);
    let err = Cli::try_parse_from(["sundgren", "diff", "a"]);
    assert_eq!(err.unwrap_err().exit_code(), EXIT_USAGE);
}

const LINES: &str = "line 1\nline 2\nline 3\nline 4\nline 5\nline 6\nline 7\nline 8\n\
                         line 9\nline 10\nline 11\nline 12\nline 13\nline 14\nline 15\n\
                         line 16\nline 17\nline 18\nline 19\nline 20\n";
const LCOV: &str = "printf 'SF:lines.txt\\nDA:1,1\\nend_of_record\\n' > cov.lcov";

fn git(repo: &Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

/// A one-commit repo and a patch that inserts two lines after line 12.
fn repo(tmp: &Path, config: &[&str]) -> (PathBuf, PathBuf) {
    let repo = tmp.join("repo");
    std::fs::create_dir(&repo).unwrap();
    git(&repo, &["init", "-q"]);
    for (k, v) in [
        ("user.name", "T"),
        ("user.email", "t@e"),
        ("commit.gpgsign", "false"),
    ] {
        git(&repo, &["config", k, v]);
    }
    std::fs::write(repo.join("lines.txt"), LINES).unwrap();
    git(&repo, &["add", "lines.txt"]);
    git(&repo, &["commit", "-q", "-m", "init"]);
    let edited = LINES.replace("line 12\n", "line 12\nnew a\nnew b\n");
    std::fs::write(repo.join("lines.txt"), edited).unwrap();
    let patch = tmp.join("p.patch");
    std::fs::write(
        &patch,
        git(&repo, &[config, &["diff", "--no-color"]].concat()),
    )
    .unwrap();
    git(&repo, &["checkout", "--", "lines.txt"]);
    (repo, patch)
}

fn sundgren(args: &[&str]) -> Result<()> {
    run(&Cli::try_parse_from([&["sundgren"], args].concat()).unwrap())
}

fn run_cmd(repo: &Path, patch: &Path, out: &Path, extra: &[&str]) -> Result<()> {
    let (repo, patch, out) = (
        repo.to_str().unwrap(),
        patch.to_str().unwrap(),
        out.to_str().unwrap(),
    );
    let args = [
        "run", "--repo", repo, "--patch", patch, "-o", out, "--cmd", LCOV,
    ];
    sundgren(&[&args[..], &["--shell", "--lcov", "cov.lcov"], extra].concat())
}

fn report(out: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(out).unwrap()).unwrap()
}

#[test]
fn offset_patch_warns_and_uses_applied_hunks() {
    let tmp = tempfile::tempdir().unwrap();
    let (repo, patch) = repo(tmp.path(), &[]);
    let offset = tmp.path().join("offset.patch");
    let text = std::fs::read_to_string(&patch).unwrap();
    let shifted = text.replace("@@ -10,6 +10,8 @@", "@@ -13,6 +13,8 @@");
    assert_ne!(shifted, text);
    std::fs::write(&offset, shifted).unwrap();
    let out = tmp.path().join("r.json");
    run_cmd(&repo, &offset, &out, &[]).unwrap();
    let r = report(&out);
    let msg = "patch applied at an offset in lines.txt; line maps use the applied diff";
    assert_eq!(
        r["warnings"],
        json!([{"code": "patch_offset", "message": msg}])
    );
    let sha = sha256_file(&offset).unwrap();
    let path = offset.to_str().unwrap();
    assert_eq!(
        r["meta"]["patch"],
        json!({"path": path, "sha256": sha, "files": ["lines.txt"]})
    );
    let exact = tmp.path().join("exact.json");
    run_cmd(&repo, &patch, &exact, &[]).unwrap();
    let e = report(&exact);
    assert_eq!(e["warnings"], json!([]));
    assert_eq!(e["files"], r["files"]);
}

#[test]
fn mnemonic_prefix_patch() {
    let tmp = tempfile::tempdir().unwrap();
    let (repo, patch) = repo(tmp.path(), &["-c", "diff.mnemonicPrefix=true"]);
    assert!(
        std::fs::read_to_string(&patch)
            .unwrap()
            .contains("--- i/lines.txt")
    );
    let out = tmp.path().join("r.json");
    run_cmd(&repo, &patch, &out, &[]).unwrap();
    let r = report(&out);
    assert_eq!(r["warnings"], json!([]));
    assert_eq!(r["meta"]["patch"]["files"], json!(["lines.txt"]));
}

#[test]
fn git_failures_exit_codes() {
    let tmp = tempfile::tempdir().unwrap();
    let (repo, patch) = repo(tmp.path(), &[]);
    let out = tmp.path().join("r.json");
    let err = run_cmd(&repo, &patch, &out, &["--base", "no-such-ref"]).unwrap_err();
    assert_eq!(err.code, EXIT_USAGE);
    assert!(err.msg.contains("no-such-ref"), "{}", err.msg);

    let workdir = tmp.path().join("wd");
    std::fs::create_dir_all(workdir.join("base")).unwrap();
    // git worktree add refuses a non-empty dir.
    std::fs::write(workdir.join("base/taken"), "x").unwrap();
    let err = run_cmd(
        &repo,
        &patch,
        &out,
        &["--workdir", workdir.to_str().unwrap()],
    );
    let err = err.unwrap_err();
    assert_eq!(err.code, crate::EXIT_FAILURE);
    assert!(err.msg.starts_with("git: "), "{}", err.msg);

    let bad = tmp.path().join("bad.patch");
    let text = std::fs::read_to_string(&patch).unwrap();
    std::fs::write(&bad, text.replace(" line 11\n", " line eleven\n")).unwrap();
    let err = run_cmd(&repo, &bad, &out, &[]).unwrap_err();
    assert_eq!(err.code, crate::EXIT_PATCH);
}

#[test]
fn run_usage_errors() {
    let tmp = tempfile::tempdir().unwrap();
    let (repo, patch) = repo(tmp.path(), &[]);
    let (r, p) = (repo.to_str().unwrap(), patch.to_str().unwrap());
    for args in [
        &["run", "--repo", r, "--patch", p, "--repeat", "0"][..],
        &["run", "--repo", r, "--patch", p, "--lcov", "x"],
        &["run", "--repo", r, "--patch", p, "--cmd", "x"],
        &["run", "--repo", r, "--patch", "/nonexistent.patch"],
        &["run", "--repo", "/", "--patch", p],
        &["run", "--repo", r, "--patch", p, "--preset", "nope"],
        &["run", "--repo", r, "--patch", p], // no Python or Rust marker
        &[
            "run", "--repo", r, "--patch", p, "--cmd", "x", "--lcov", "l", "--env", "X",
        ],
    ] {
        assert_eq!(sundgren(args).unwrap_err().code, EXIT_USAGE, "{args:?}");
    }
}

#[test]
fn diff_mode_patch_read_as_bytes() {
    let tmp = tempfile::tempdir().unwrap();
    let lcov = tmp.path().join("a.lcov");
    std::fs::write(&lcov, "SF:a.txt\nDA:1,1\nDA:2,1\nend_of_record\n").unwrap();
    let patch = tmp.path().join("crlf.patch");
    let bytes: &[u8] = b"diff --git a/a.txt b/a.txt\r\n--- a/a.txt\r\n+++ b/a.txt\r\n\
                             @@ -1,2 +1,3 @@\r\n caf\xe9\r\n+new\r\n x\r\n";
    std::fs::write(&patch, bytes).unwrap();
    let out = tmp.path().join("r.json");
    let (l, p, o) = (
        lcov.to_str().unwrap(),
        patch.to_str().unwrap(),
        out.to_str().unwrap(),
    );
    sundgren(&["diff", l, l, "--patch", p, "-o", o]).unwrap();
    assert_eq!(report(&out)["meta"]["patch"]["files"], json!(["a.txt"]));
}

#[test]
fn diff_input_errors() {
    let tmp = tempfile::tempdir().unwrap();
    let good = tmp.path().join("a.lcov");
    std::fs::write(&good, "SF:a\nDA:1,1\n").unwrap();
    let bad = tmp.path().join("b.lcov");
    std::fs::write(&bad, "SF:a\nDA:x\n").unwrap();
    let junk = tmp.path().join("j.xml");
    std::fs::write(&junk, "<coverage/>").unwrap();
    let out = tmp.path().join("r.json");
    let [g, b, j, o] = [&good, &bad, &junk, &out].map(|p| p.to_str().unwrap());
    assert_eq!(
        sundgren(&["diff", g, "/nonexistent"]).unwrap_err().code,
        EXIT_USAGE
    );
    assert_eq!(
        sundgren(&["diff", g, b]).unwrap_err().code,
        crate::EXIT_FAILURE
    );
    let missing = sundgren(&["diff", g, g, "--patch", "/nonexistent"]);
    assert_eq!(missing.unwrap_err().code, EXIT_USAGE);
    let missing = sundgren(&["diff", g, g, "--base-junit", "/nonexistent"]);
    assert_eq!(missing.unwrap_err().code, EXIT_USAGE);
    sundgren(&[
        "diff",
        g,
        g,
        "--base-junit",
        j,
        "--patched-junit",
        j,
        "-o",
        o,
    ])
    .unwrap();
    let w = &report(&out)["warnings"];
    assert_eq!(w[0]["code"], "junit_unreadable");
    assert_eq!(w[1], json!(null));
}
