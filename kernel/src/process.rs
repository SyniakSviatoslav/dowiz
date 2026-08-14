//! process.rs — subprocess seam (ledger item 4: process → kexec).
//!
//! The no_std audit found a small set of `std::process::Command` call sites in
//! otherwise no_std-ready modules (`span_metrics/breach`'s `perf record`,
//! `living_knowledge`'s `node --version` availability check). A kernel module
//! has no `std::process` — it execs via `kexec`/`call_usermodehelper`. This
//! module is the single seam, in the same shape as [`crate::clock`]: a
//! no_std-compatible [`Process`] trait (`&str` cmd + `String` args + i32 exit
//! code — no `Command`/`Child`/`Stdio`), a userspace [`StdProcess`] impl, and a
//! [`run`] free function that is the single authority. The kernel port swaps the
//! impl, not the call sites.
//!
//! # Out of scope (documented follow-up)
//! The `living_knowledge` sh-bridge (`spawn` + `stdin/stdout/stderr` pipes +
//! `wait4(2)` rusage) is a *bidirectional pipe* pattern — a `kexec` port needs a
//! different fd-passing design, so it stays std (it already uses a raw `wait4`
//! syscall in the native path, the same style as `fdr::pmu`).

use alloc::string::String;
use alloc::vec::Vec;

/// Result of running a command to completion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    /// Exit code, or `-1` if the process was signaled / could not be spawned.
    pub code: i32,
    /// Captured stdout.
    pub stdout: Vec<u8>,
    /// Captured stderr.
    pub stderr: Vec<u8>,
}

impl CommandOutput {
    /// Whether the command exited successfully (code 0).
    pub fn success(&self) -> bool {
        self.code == 0
    }
}

/// The subprocess abstraction. no_std-compatible signature.
pub trait Process {
    /// Run `cmd` with `args` to completion, capturing stdout/stderr.
    fn run(&self, cmd: &str, args: &[String]) -> CommandOutput;
}

/// The userspace subprocess impl (`std::process::Command`).
pub struct StdProcess;

impl Process for StdProcess {
    fn run(&self, cmd: &str, args: &[String]) -> CommandOutput {
        match std::process::Command::new(cmd).args(args).output() {
            Ok(o) => CommandOutput {
                code: o.status.code().unwrap_or(-1),
                stdout: o.stdout,
                stderr: o.stderr,
            },
            Err(_) => CommandOutput {
                code: -1,
                stdout: Vec::new(),
                stderr: Vec::new(),
            },
        }
    }
}

/// Single authority for "run a command". Kernel port swaps to
/// `kexec`/`call_usermodehelper`.
pub fn run(cmd: &str, args: &[String]) -> CommandOutput {
    StdProcess.run(cmd, args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_echo_captures_stdout() {
        // `sh -c 'echo hi'` is universally available on the test hosts.
        let out = run("sh", &["-c".into(), "echo hi".into()]);
        assert!(out.success(), "echo must exit 0");
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "hi");
    }

    #[test]
    fn run_missing_command_is_not_success() {
        let out = run("/definitely/not/a/real/binary", &[]);
        assert!(!out.success(), "missing binary must report failure");
    }
}
