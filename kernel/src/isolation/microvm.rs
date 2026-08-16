//! Host-capability probe for the microVM isolation tier (DK-06 / MV-04) — std side.
//!
//! The pure value types (`SandboxTier`, `AdapterRejected`) and the fail-closed
//! `register_adapter` seam live in `dowiz_core::isolation::microvm`. This module
//! supplies the real std host probe (reads `/dev/kvm` + `/proc/cpuinfo`) and installs
//! it via [`set_kvm_probe`].
//!
//! The probe is intentionally cheap, offline, and std-only (no new crates). Real
//! Firecracker/Kata boot is a follow-up behind this probe (DK-06 form), gated by the
//! boolean this module advertises.

pub use dowiz_core::isolation::microvm::{
    register_adapter, set_kvm_probe, AdapterRejected, SandboxTier,
};

/// Probe whether the host can back a microVM with hardware virtualization.
///
/// True only when BOTH hold:
/// 1. `/dev/kvm` exists (KVM device node present), AND
/// 2. the CPU advertises virtualization extensions (`vmx` on Intel, `svm` on AMD)
///    in `/proc/cpuinfo`.
pub fn kvm_available() -> bool {
    has_kvm_device() && has_vmx_or_svm()
}

/// Whether a native (`native-process`) adapter may be accepted on this host.
pub fn can_accept_native_adapter() -> bool {
    kvm_available()
}

/// Install the std KVM probe so the core's fail-closed `register_adapter`
/// reflects the real host capability. Idempotent; call once at runtime startup.
/// Until this runs, `register_adapter("native-process")` fails closed (refuses),
/// which is the safe default.
pub fn init() {
    set_kvm_probe(kvm_available);
}

/// `/dev/kvm` present on the host?
fn has_kvm_device() -> bool {
    std::path::Path::new("/dev/kvm").exists()
}

/// CPU advertises hardware virtualization (`vmx` Intel / `svm` AMD)?
fn has_vmx_or_svm() -> bool {
    match crate::vfs::read_to_string("/proc/cpuinfo") {
        Ok(contents) => contents
            .split_whitespace()
            .any(|tok| tok == "vmx" || tok == "svm"),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r1_kvm_unavailable_on_this_host() {
        assert!(!kvm_available(), "this host is expected to have NO /dev/kvm and NO vmx/svm; fail-closed posture requires false");
    }

    #[test]
    fn r2_native_process_refused_without_kvm() {
        // No probe installed in this test process → core seam fails closed.
        let res = register_adapter("native-process");
        assert!(
            matches!(res, Err(AdapterRejected { reason: _ })),
            "native-process adapter MUST be refused on a host without KVM; got {:?}",
            res
        );
        if let Err(rej) = res {
            assert_eq!(rej.reason.contains("isolate"), true);
        }
    }

    #[test]
    fn r3_wasm_component_accepted() {
        assert_eq!(register_adapter("wasm-component"), Ok(()));
    }

    #[test]
    fn r4_cannot_accept_native_adapter_without_kvm() {
        assert!(!can_accept_native_adapter());
    }

    #[test]
    fn r5_unknown_execution_refused() {
        assert!(register_adapter("mystery-runtime").is_err());
    }

    #[test]
    fn invariant_sandbox_tier_distinct() {
        assert_ne!(
            SandboxTier::WasmComponent,
            SandboxTier::NativeProcessRequiresKvm
        );
    }
}
