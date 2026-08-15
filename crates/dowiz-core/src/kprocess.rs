#![allow(unused)]
//! kprocess.rs — subprocess seam (ledger item 4: process → kprocess).
//!
//! Pure no_std: `Process` trait + `ProcessResult` error type.
//! Std-gated: `StdProcess` impl + `run` free function.

use alloc::string::String;
use alloc::vec::Vec;

/// Result of a subprocess run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessResult {
    pub exit_code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub success: bool,
}

impl ProcessResult {
    pub fn success(&self) -> bool { self.success }
    pub fn exit_code(&self) -> i32 { self.exit_code }
}

/// no_std-compatible process abstraction.
pub trait Process {
    fn run(&self, cmd: &str, args: &[String]) -> ProcessResult;
}

#[cfg(feature = "std")]
mod std_impl {
    use super::*;
    use std::process::Command;

    pub struct StdProcess;

    impl Process for StdProcess {
        fn run(&self, cmd: &str, args: &[String]) -> ProcessResult {
            let output = Command::new(cmd).args(args).output();
            match output {
                Ok(o) => ProcessResult {
                    exit_code: o.status.code().unwrap_or(-1),
                    stdout: o.stdout,
                    stderr: o.stderr,
                    success: o.status.success(),
                },
                Err(_) => ProcessResult {
                    exit_code: -1,
                    stdout: vec![],
                    stderr: vec![],
                    success: false,
                },
            }
        }
    }
}

#[cfg(feature = "std")]
pub use std_impl::StdProcess;

/// Convenience: run a command with StdProcess (std-gated).
#[cfg(feature = "std")]
pub fn run(cmd: &str, args: &[String]) -> ProcessResult {
    std_impl::StdProcess.run(cmd, args)
}
