use super::*;

/// `script` under sh, in its own group and running.
fn sh_group(script: &str) -> (Child, libc::pid_t) {
    let mut child = spawn(
        Command::new("sh")
            .arg("-c")
            .arg(script)
            .stdin(Stdio::null())
            .stdout(Stdio::null()),
    )
    .unwrap();
    let pgid = child.id() as libc::pid_t;
    // Let sh start its children.
    std::thread::sleep(Duration::from_millis(100));
    assert!(child.try_wait().unwrap().is_none());
    (child, pgid)
}

#[test]
fn wait_returns_the_status() {
    let mut child = spawn(Command::new("sh").args(["-c", "exit 3"])).unwrap();
    let flag = AtomicUsize::new(0);
    assert_eq!(wait_on(&mut child, &flag).unwrap().code(), Some(3));
}

#[test]
fn signal_stops_the_whole_group() {
    let (mut child, pgid) = sh_group("sleep 30 & sleep 30; wait");
    let flag = AtomicUsize::new(libc::SIGTERM as usize);
    let start = Instant::now();
    let err = wait_on(&mut child, &flag).unwrap_err();
    assert_eq!(err.code, 143);
    assert!(
        start.elapsed() < Duration::from_millis(500),
        "{:?}",
        start.elapsed()
    );
    assert!(!group_alive(pgid));
}

#[test]
fn sigkill_after_grace_when_sigterm_is_ignored() {
    // Ignored signals stay ignored across exec, so the sleeps ignore SIGTERM too.
    let (mut child, pgid) = sh_group("trap '' TERM; sleep 30 & sleep 30; wait");
    let flag = AtomicUsize::new(libc::SIGINT as usize);
    let start = Instant::now();
    let err = wait_on(&mut child, &flag).unwrap_err();
    assert_eq!(err.code, 130);
    let took = start.elapsed();
    assert!(
        took >= GRACE && took < GRACE + Duration::from_millis(500),
        "{took:?}"
    );
    // SIGKILL is sent; the orphaned sleep may take a moment to be reaped by init.
    let deadline = Instant::now() + Duration::from_secs(1);
    while group_alive(pgid) && Instant::now() < deadline {
        std::thread::sleep(POLL);
    }
    assert!(!group_alive(pgid));
}

#[test]
fn output_captures_both_streams() {
    let out = output(Command::new("sh").args(["-c", "echo out; echo err >&2; exit 1"])).unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(out.stdout, b"out\n");
    assert_eq!(out.stderr, b"err\n");
}

#[test]
fn exit_codes() {
    assert_eq!(exit_code(libc::SIGINT), 130);
    assert_eq!(exit_code(libc::SIGTERM), 143);
}
