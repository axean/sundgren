use super::*;
use presets::adhoc_preset;

#[test]
fn wrong_tree_paths_cases() {
    let paths = [
        "/repo/src/a.py",
        "rel.py",
        "/repo/wt/src/a.py",
        "/elsewhere/x.py",
        "/repo/src/a.py",
    ];
    let got = wrong_tree_paths(paths.into_iter(), Path::new("/repo/wt"), Path::new("/repo"));
    assert_eq!(got, ["/repo/src/a.py"]);
    // Component-wise: /repo-other is not inside /repo.
    let got = wrong_tree_paths(
        ["/repo-other/a.py"].into_iter(),
        Path::new("/wt"),
        Path::new("/repo"),
    );
    assert!(got.is_empty());
}

#[test]
fn wrong_tree_message_truncates_and_adds_hints() {
    let mut preset = adhoc_preset(vec!["x".into()], "l".into(), None, false);
    let paths: Vec<String> = (0..22).map(|i| format!("/repo/f{i}.py")).collect();
    let msg = wrong_tree_message("base", &paths, &preset);
    assert!(msg.starts_with(
        "the base run measured your main checkout instead of its worktree \
             (editable install or stale build?):\n  /repo/f0.py\n"
    ));
    assert!(msg.ends_with("  /repo/f19.py\n  ... and 2 more"));
    preset.hints = vec!["reinstall".into()];
    let msg = wrong_tree_message("patched", &paths[..1], &preset);
    assert!(msg.ends_with("  /repo/f0.py\nhints:\n  - reinstall"));
}

#[test]
fn default_python_is_a_path_or_python() {
    let p = default_python();
    assert!(p == "python" || p.starts_with('/'), "{p}");
}

fn config(repo: &Path, preset: Preset) -> RunConfig {
    RunConfig {
        repo: repo.to_path_buf(),
        base: "HEAD".into(),
        patch_path: Some(repo.join("p.patch")),
        workdir: None,
        keep_worktrees: false,
        preset,
        python: "python".into(),
        env: Vec::new(),
        tests: Vec::new(),
        all_tests: false,
        repeat: 1,
        progress: false,
    }
}

#[test]
fn run_pair_end_to_end() {
    let repo = workspace::tests::repo();
    workspace::tests::patch(repo.path(), "sed -i s/^2$/two/ f.txt");
    // "Coverage": line 2 is hit on the base only.
    let cmd = "sh -c 'if grep -qx 2 f.txt; then h=1; else h=0; fi; \
                   printf \"SF:f.txt\\nDA:1,1\\nDA:2,$h\\nend_of_record\\n\" > {lcov}'";
    let preset = adhoc_preset(vec![cmd.into()], "{out}/l.info".into(), None, false);
    let got = run_pair(&config(repo.path(), preset)).unwrap();
    assert_eq!(got.base_ref.len(), 40);
    assert_eq!((got.base_sha.len(), got.patched_sha.len()), (1, 1));
    assert_eq!(got.patch.files.len(), 1);
    assert!(got.selection.run_all);
    // p.patch is untracked, so the tree isn't dirty.
    assert!(got.warnings.is_empty(), "{:?}", got.warnings);
    let out = Command::new("git")
        .args(["worktree", "list"])
        .current_dir(repo.path())
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout).lines().count(), 1);
}

#[test]
fn run_pair_fails_on_wrong_tree_and_cleans_up() {
    let repo = workspace::tests::repo();
    workspace::tests::patch(repo.path(), "sed -i s/^2$/two/ f.txt");
    let main = repo.path().canonicalize().unwrap();
    let cmd = format!(
        "sh -c 'printf \"SF:{}/f.txt\\nDA:1,1\\nend_of_record\\n\" > {{lcov}}'",
        main.display()
    );
    let preset = adhoc_preset(vec![cmd], "{out}/l.info".into(), None, false);
    let e = run_pair(&config(repo.path(), preset)).err().unwrap();
    assert_eq!(e.code, crate::EXIT_FAILURE);
    assert!(
        e.msg
            .starts_with("the base run measured your main checkout"),
        "{}",
        e.msg
    );
}

use std::process::Command;

#[test]
fn all_tests_ignores_the_selector() {
    let repo = workspace::tests::repo();
    let mut preset = adhoc_preset(vec!["x".into()], "l".into(), None, false);
    preset.selector = Some("python.pytest_files".into());
    let mut cfg = config(repo.path(), preset);
    cfg.all_tests = true;
    let got = select(&cfg, &Patch::default(), repo.path()).unwrap();
    assert!(got.run_all);
    assert_eq!(got.reason, "--all-tests given");
}
