//! Host-capability probe for the microVM isolation tier (DK-06 / MV-04).
//!
//! This is the **fail-closed** gate that decides whether a node is allowed to
//! accept a `native-process` (untrusted, non-WASM) adapter. A microVM sandbox
//! (Firecracker / Cloud Hypervisor / Kata) hard-requires KVM — hardware
//! virtualization on the host. If the host cannot provide that isolation, the
//! node MUST refuse the adapter rather than silently running it unsandboxed.
//!
//! The probe is intentionally cheap, offline, and std-only (no new crates):
//! it reads `/dev/kvm` existence and the CPU virtualization flags in
//! `/proc/cpuinfo`. Real Firecracker/Kata boot is a *follow-up behind this
//! probe* (DK-06 form), gated by the boolean this module advertises.
//!
//! innovate: today we only probe host capability. The actual VMM launch
//! (jailer, seccomp, guest kernel, network tap) is the next unit and must
//! only ever run when [`kvm_available`] is true.

/// Sandbox tiers an adapter can be scheduled into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxTier {
    /// WASM Component — default untrusted-extension path (capability-scoped,
    /// no KVM dependency). Always accepted.
    WasmComponent,
    /// Native process requiring a hardware-isolated microVM (KVM-backed).
    /// Only available on hosts that advertise KVM.
    NativeProcessRequiresKvm,
}

/// Reason an adapter registration was refused by the fail-closed gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdapterRejected {
    /// Static, human-readable reason. Lifetime-bound to the binary; no alloc.
    pub reason: &'static str,
}

impl core::fmt::Display for AdapterRejected {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.reason)
    }
}

/// Probe whether the host can back a microVM with hardware virtualization.
///
/// True only when BOTH hold:
/// 1. `/dev/kvm` exists (KVM device node present), AND
/// 2. the CPU advertises virtualization extensions (`vmx` on Intel,
///    `svm` on AMD) in `/proc/cpuinfo`.
///
/// On the current build/CI host there is no `/dev/kvm` and no virtualization
/// flag, so this returns `false` — which is the **correct** fail-closed posture:
/// a node that cannot isolate must not accept native-process adapters.
pub fn kvm_available() -> bool {
    has_kvm_device() && has_vmx_or_svm()
}

/// Whether a native (`native-process`) adapter may be accepted on this host.
///
/// Native-process adapters need a hardware-isolated microVM, so acceptance is
/// exactly [`kvm_available`]. WASM-component adapters never consult this — see
/// [`register_adapter`].
pub fn can_accept_native_adapter() -> bool {
    kvm_available()
}

/// Register an adapter against the fail-closed isolation gate.
///
/// - `execution == "wasm-component"` → always `Ok(())` (capability-scoped, no
///   KVM dependency).
/// - `execution == "native-process"` → `Ok(())` **only** if [`kvm_available`];
///   otherwise `Err(AdapterRejected::HostCannotIsolate)`. There is deliberately
///   **no fallback** to running the adapter unsandboxed.
/// - any other value → `Err(AdapterRejected::UnknownExecution)`.
///
/// This is the MV-04 RED gate: a node without KVM MUST refuse a native-process
/// adapter, never silently downgrade to an unsandboxed execution.
pub fn register_adapter(execution: &str) -> Result<(), AdapterRejected> {
    match execution {
        "wasm-component" => Ok(()),
        "native-process" => {
            if kvm_available() {
                Ok(())
            } else {
                Err(AdapterRejected {
                    reason: "host cannot isolate: /dev/kvm or CPU virtualization (vmx/svm) unavailable; refusing native-process adapter (no unsandboxed fallback)",
                })
            }
        }
        _other => Err(AdapterRejected {
            reason: "unknown execution model; refuse by default (fail-closed)",
        }),
    }
}

/// `/dev/kvm` present on the host?
fn has_kvm_device() -> bool {
    // std-only; `Path::exists` does a stat. No allocation beyond the Path.
    std::path::Path::new("/dev/kvm").exists()
}

/// CPU advertises hardware virtualization (`vmx` Intel / `svm` AMD)?
fn has_vmx_or_svm() -> bool {
    // `/proc/cpuinfo` is tiny and always present on Linux. We scan for the
    // virtualization flag token. This is a best-effort host probe; if the file
    // cannot be read we conservatively report "no virtualization".
    match std::fs::read_to_string("/proc/cpuinfo") {
        Ok(contents) => {
            // Flags appear as a space-separated list; check for whole tokens to
            // avoid matching substrings like "svmxyz".
            contents
                .split_whitespace()
                .any(|tok| tok == "vmx" || tok == "svm")
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── R1 · on this host kvm_available() is false ──────────────────────────
    // RED→GREEN: this CI/build host has no /dev/kvm and no vmx/svm, so the
    // fail-closed probe must report unavailable (correct posture).
    #[test]
    fn r1_kvm_unavailable_on_this_host() {
        assert!(!kvm_available(), "this host is expected to have NO /dev/kvm and NO vmx/svm; fail-closed posture requires false");
    }

    // ── R2 · native-process adapter is REFUSED without KVM ─────────────────
    // MV-04 RED gate: no silent downgrade to unsandboxed execution.
    #[test]
    fn r2_native_process_refused_without_kvm() {
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

    // ── R3 · wasm-component adapter is always accepted ─────────────────────
    #[test]
    fn r3_wasm_component_accepted() {
        assert_eq!(
            register_adapter("wasm-component"),
            Ok(()),
            "wasm-component adapters are capability-scoped and need no KVM; must always register"
        );
    }

    // ── R4 · can_accept_native_adapter() is false here ─────────────────────
    #[test]
    fn r4_cannot_accept_native_adapter_without_kvm() {
        assert!(!can_accept_native_adapter(), "native-process adapters require KVM; this host has none");
    }

    // ── R5 · unknown execution model is refused (fail-closed default) ──────
    #[test]
    fn r5_unknown_execution_refused() {
        assert!(register_adapter("mystery-runtime").is_err());
    }

    // ── invariant · SandboxTier discriminant identity ──────────────────────
    #[test]
    fn invariant_sandbox_tier_distinct() {
        assert_ne!(SandboxTier::WasmComponent, SandboxTier::NativeProcessRequiresKvm);
    }
}
