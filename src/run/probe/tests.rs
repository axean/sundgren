use super::*;

#[test]
fn parse_reads_the_last_marker_line() {
    let out = "collected 2 items\nSUNDGREN-PROBE [[\"old\", \"/x\"]]\n\
               SUNDGREN-PROBE [[\"srcpkg\", \"/repo/src/srcpkg/__init__.py\"]]\n";
    assert_eq!(
        parse(out),
        vec![(
            "srcpkg".to_owned(),
            "/repo/src/srcpkg/__init__.py".to_owned()
        )]
    );
}

#[test]
fn parse_without_a_marker_or_with_bad_json_finds_nothing() {
    assert!(parse("").is_empty());
    assert!(parse("ModuleNotFoundError: No module named 'pytest'\n").is_empty());
    assert!(parse("SUNDGREN-PROBE not json\n").is_empty());
}

#[test]
fn applies_only_to_pytest_with_python() {
    let preset = |commands: &[&str]| {
        let commands = commands.iter().map(|c| (*c).to_owned()).collect();
        crate::run::presets::adhoc_preset(commands, "l.info".into(), None, false)
    };
    assert!(applies(&preset(&["{python} -m coverage run -m pytest -q"])));
    assert!(!applies(&preset(&[
        "cargo llvm-cov --lcov --output-path {lcov}"
    ])));
    assert!(!applies(&preset(&["pytest -q"])));
}
