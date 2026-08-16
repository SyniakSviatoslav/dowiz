//! hydra_closed_loop — kernel re-export shim for the `no_std` core engine.
//!
//! The pure closed-loop engine lives in `dowiz_core::hydra_closed_loop`; this
//! module re-exports it so existing kernel call sites (`crate::hydra_closed_loop::…`
//! and `dowiz_kernel::hydra_closed_loop::…`) keep working unchanged. The only
//! std-dependent piece is the runtime-probe golden test below, which spawns a
//! kernel binary and therefore stays in the std kernel crate.

pub use dowiz_core::hydra_closed_loop::*;

#[cfg(test)]
mod tests {
    /// Cryptographic golden test: running the probe in verification mode must
    /// emit a byte-exact stable stdout sequence. Any change to probe output
    /// formatting or cycle behavior fails this SHA3-256 KAT.
    #[test]
    fn hydra_runtime_probe_golden_sha3_256() {
        use std::process::Command;

        let binary = std::path::PathBuf::from("/root/dowiz/kernel/target/debug/hydra_runtime_probe");
        let output = Command::new(&binary)
            .args(["--verify-golden", "--cycles", "4"])
            .output()
            .expect("failed to spawn hydra_runtime_probe");

        assert!(
            output.status.success(),
            "probe verify-golden must exit 0: {:?}",
            output
        );

        let expected_hash: [u8; 32] = [
            0xce, 0x55, 0x58, 0xcc, 0x67, 0x25, 0x5d, 0xa6,
            0x48, 0x14, 0x97, 0x43, 0xcb, 0x20, 0x10, 0x6a,
            0xd3, 0x1f, 0xff, 0x3c, 0x0c, 0x0d, 0x93, 0x33,
            0x4f, 0xbe, 0xf2, 0x91, 0xed, 0x87, 0xe3, 0x31,
        ];
        let actual_hash = crate::event_log::sha3_256(&output.stdout);
        assert_eq!(
            actual_hash, expected_hash,
            "probe golden output hash mismatch"
        );
    }
}
