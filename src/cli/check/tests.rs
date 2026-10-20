use super::*;
use crate::run::presets::adhoc_preset;

fn problems(exits: &[i32]) -> Problems {
    let mut p = Problems::default();
    for (i, &exit) in exits.iter().enumerate() {
        p.add("error", exit, format!("p{i}"));
    }
    p
}

#[test]
fn exit_code_precedence() {
    assert_eq!(problems(&[]).exit_code(), EXIT_OK);
    assert_eq!(problems(&[1, 126]).exit_code(), EXIT_FAILURE);
    assert_eq!(problems(&[1, 3]).exit_code(), EXIT_PATCH);
    let p = problems(&[1, 3, 2]);
    assert_eq!(p.exit_code(), EXIT_USAGE);
    assert_eq!(p.decisive().unwrap().message, "p2");
}

#[test]
fn patch_message_is_one_line() {
    assert_eq!(
        patch_message(
            "patch does not apply:\nerror: patch failed: f:1\nerror: f: patch does not apply"
        ),
        "the patch does not apply to the base: error: patch failed: f:1; error: f: patch does not apply"
    );
    assert_eq!(
        patch_message("patch does not apply:"),
        "the patch does not apply to the base"
    );
}

#[test]
fn python_is_reported_only_when_used() {
    let preset = |cmd: &str| adhoc_preset(vec![cmd.into()], "l".into(), None, false);
    assert!(uses_python(&preset("{python} -m x")));
    assert!(!uses_python(&preset("cargo test")));
    let mut p = preset("cargo test");
    p.checks = vec!["{python} -V".into()];
    assert!(uses_python(&p));
}
