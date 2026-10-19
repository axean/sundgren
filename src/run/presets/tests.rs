use super::*;

fn err(preset: &str) -> String {
    let doc: Table = format!("[presets.bad]\n{preset}").parse().unwrap();
    parse_presets(&doc, "f.toml", "x").unwrap_err().msg
}

fn repo(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (name, text) in files {
        std::fs::write(dir.path().join(name), text).unwrap();
    }
    dir
}

#[test]
fn builtins() {
    let all = all_presets(repo(&[]).path()).unwrap();
    assert_eq!(
        all.keys().collect::<Vec<_>>(),
        ["cargo-llvm-cov", "cargo-tarpaulin", "pytest"]
    );
    let pytest = &all["pytest"];
    assert_eq!(pytest.selector.as_deref(), Some("python.pytest_files"));
    assert_eq!(pytest.junit.as_deref(), Some("{out}/junit.xml"));
    assert_eq!(pytest.hints.len(), 2);
    assert_eq!(all["cargo-tarpaulin"].tests_style, TestsStyle::Filter);
    assert_eq!(pytest.source, "built-in");
}

#[test]
fn minimal_preset_gets_defaults() {
    let doc: Table = "[presets.p]\ncommands = ['run']\nlcov = 'x'"
        .parse()
        .unwrap();
    let got = parse_presets(&doc, "f.toml", "sundgren.toml").unwrap();
    assert_eq!(
        got,
        [Preset {
            name: "p".into(),
            description: String::new(),
            commands: vec!["run".into()],
            env: vec![],
            lcov: "x".into(),
            junit: None,
            selector: None,
            tests_style: TestsStyle::Args,
            shell: false,
            hints: vec![],
            source: "sundgren.toml".into(),
        }]
    );
}

#[test]
fn validation_messages() {
    assert_eq!(
        err("lcov = 'x'"),
        "f.toml: preset 'bad': missing required key 'commands'"
    );
    assert_eq!(
        err("commands = ['x']"),
        "f.toml: preset 'bad': missing required key 'lcov'"
    );
    assert_eq!(
        err("commands = ['x']\nlcov = 1"),
        "f.toml: preset 'bad': 'lcov' must be a string"
    );
    assert_eq!(
        err("commands = ['x']\nlcov = ''"),
        "f.toml: preset 'bad': 'lcov' must be a non-empty string"
    );
    assert_eq!(
        err("commands = []\nlcov = 'x'"),
        "f.toml: preset 'bad': 'commands' must not be empty"
    );
    assert_eq!(
        err("commands = [1]\nlcov = 'x'"),
        "f.toml: preset 'bad': 'commands' must be a list of strings"
    );
    assert_eq!(
        err("commands = ['x']\nlcov = 'x'\nzz = 1\naa = 2"),
        "f.toml: preset 'bad': unknown key(s) aa, zz"
    );
    assert_eq!(
        err("commands = ['x']\nlcov = 'x'\ntests_style = 'x'"),
        "f.toml: preset 'bad': 'tests_style' must be 'args' or 'filter'"
    );
    assert_eq!(
        err("commands = ['x']\nlcov = 'x'\nshell = 'y'"),
        "f.toml: preset 'bad': 'shell' must be a boolean"
    );
    assert_eq!(
        err("commands = ['x']\nlcov = 'x'\nenv = { A = 1 }"),
        "f.toml: preset 'bad': 'env' must be a table of strings"
    );
    let doc: Table = "presets = 1".parse().unwrap();
    assert_eq!(
        parse_presets(&doc, "f.toml", "x").unwrap_err().msg,
        "f.toml: 'presets' must be a table"
    );
}

#[test]
fn user_presets_override_builtins_and_sundgren_toml_wins() {
    let dir = repo(&[
        (
            "pyproject.toml",
            "[tool.sundgren.presets.pytest]\ncommands = ['a']\nlcov = 'l'\n\
                 [tool.sundgren.presets.both]\ncommands = ['py']\nlcov = 'l'\n",
        ),
        (
            "sundgren.toml",
            "[presets.both]\ncommands = ['st']\nlcov = 'l'\n",
        ),
    ]);
    let all = all_presets(dir.path()).unwrap();
    assert_eq!(all["pytest"].commands, ["a"]);
    assert_eq!(all["pytest"].source, "pyproject.toml");
    assert_eq!(all["both"].commands, ["st"]);
    assert_eq!(all["both"].source, "sundgren.toml");
}

#[test]
fn bad_files_are_usage_errors() {
    let e = all_presets(repo(&[("sundgren.toml", "presets = [")]).path()).unwrap_err();
    assert_eq!(e.code, crate::EXIT_USAGE);
    let e = all_presets(repo(&[("pyproject.toml", "[tool]\nsundgren = 1\n")]).path()).unwrap_err();
    assert!(
        e.msg.ends_with("[tool.sundgren] must be a table"),
        "{}",
        e.msg
    );
    // A pyproject.toml without [tool.sundgren] is fine.
    assert!(all_presets(repo(&[("pyproject.toml", "[project]\nname = 'x'\n")]).path()).is_ok());
}
