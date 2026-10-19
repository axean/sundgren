//! The `run` subcommand: worktrees, preset commands on both sides, then diff. Owned by W5.

use crate::cli::{CliResult, Host, RunArgs};

pub fn cmd_run(_args: &RunArgs, _host: &mut dyn Host) -> CliResult<i32> {
    todo!("W5")
}
