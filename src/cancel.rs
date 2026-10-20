//! Cancellation: SIGINT and SIGTERM set a flag. Child commands run in their own process
//! group; waiting on one stops the group once the flag is set.

use std::io::{Read, Seek};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};

use crate::{Error, Result};

const POLL: Duration = Duration::from_millis(10);
/// Between SIGTERM and SIGKILL of a command's process group.
const GRACE: Duration = Duration::from_secs(1);

/// The signal number received, or 0.
static SIGNAL: LazyLock<Arc<AtomicUsize>> = LazyLock::new(Arc::default);

/// Record SIGINT and SIGTERM instead of dying, so the run can stop its commands and clean up.
pub fn install() -> Result<()> {
    for sig in [libc::SIGINT, libc::SIGTERM] {
        signal_hook::flag::register_usize(sig, Arc::clone(&SIGNAL), sig as usize)?;
    }
    Ok(())
}

/// The signal that cancelled the run, if any.
pub fn signal() -> Option<i32> {
    signal_in(&SIGNAL)
}

fn signal_in(flag: &AtomicUsize) -> Option<i32> {
    match flag.load(Ordering::SeqCst) {
        0 => None,
        sig => Some(sig as i32),
    }
}

/// 130 for SIGINT, 143 for SIGTERM.
pub fn exit_code(sig: i32) -> i32 {
    128 + sig
}

fn cancelled(sig: i32) -> Error {
    Error {
        code: exit_code(sig),
        msg: "cancelled".into(),
    }
}

/// Fail if a signal arrived.
pub fn check() -> Result<()> {
    match signal() {
        Some(sig) => Err(cancelled(sig)),
        None => Ok(()),
    }
}

/// Spawn `cmd` as the leader of a new process group. On Linux it also gets SIGKILL if we die
/// (its own children don't).
pub fn spawn(cmd: &mut Command) -> std::io::Result<Child> {
    cmd.process_group(0);
    #[cfg(target_os = "linux")]
    {
        let parent = std::process::id() as libc::pid_t;
        // SAFETY: prctl and getppid are async-signal-safe; nothing allocates.
        unsafe {
            cmd.pre_exec(move || {
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                // We died before the prctl took effect.
                if libc::getppid() != parent {
                    return Err(std::io::Error::from_raw_os_error(libc::ESRCH));
                }
                Ok(())
            });
        }
    }
    cmd.spawn()
}

/// Wait for a child from `spawn`; on a signal stop its group and fail.
pub fn wait(child: &mut Child) -> Result<ExitStatus> {
    wait_on(child, &SIGNAL)
}

fn wait_on(child: &mut Child, flag: &AtomicUsize) -> Result<ExitStatus> {
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        if let Some(sig) = signal_in(flag) {
            stop(child);
            return Err(cancelled(sig));
        }
        std::thread::sleep(POLL);
    }
}

/// Like `Command::output`, but cancellable. Output goes through unnamed temp files, so a
/// process that keeps a pipe open can't block us.
pub fn output(cmd: &mut Command) -> Result<Output> {
    let mut out = tempfile::tempfile()?;
    let mut err = tempfile::tempfile()?;
    cmd.stdin(Stdio::null())
        .stdout(out.try_clone()?)
        .stderr(err.try_clone()?);
    let status = wait(&mut spawn(cmd)?)?;
    let read = |f: &mut std::fs::File| -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        f.rewind()?;
        f.read_to_end(&mut buf)?;
        Ok(buf)
    };
    Ok(Output {
        status,
        stdout: read(&mut out)?,
        stderr: read(&mut err)?,
    })
}

/// SIGTERM the child's group, SIGKILL it if anything is left after `GRACE`, reap the child.
fn stop(child: &mut Child) {
    let pgid = child.id() as libc::pid_t;
    killpg(pgid, libc::SIGTERM);
    let deadline = Instant::now() + GRACE;
    let mut reaped = false;
    while Instant::now() < deadline {
        reaped = reaped || !matches!(child.try_wait(), Ok(None));
        if reaped && !group_alive(pgid) {
            return;
        }
        std::thread::sleep(POLL);
    }
    killpg(pgid, libc::SIGKILL);
    let _ = child.wait();
}

fn killpg(pgid: libc::pid_t, sig: libc::c_int) {
    // SAFETY: plain syscall; the group is one we created.
    unsafe {
        libc::killpg(pgid, sig);
    }
}

/// True while any process is left in the group.
pub fn group_alive(pgid: libc::pid_t) -> bool {
    // SAFETY: signal 0 only checks for existence.
    unsafe { libc::killpg(pgid, 0) == 0 }
}

#[cfg(test)]
mod tests;
