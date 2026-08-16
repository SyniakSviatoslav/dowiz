//! Sandbox-tier value types + the fail-closed `register_adapter` seam.
//!
//! The pure decision types (`SandboxTier`, `AdapterRejected`) live here in the
//! no_std core. The host KVM probe (`kvm_available`) is std-only — it reads
//! `/dev/kvm` and the CPU virtualization flags in `/proc/cpuinfo` — so it stays
//! in the kernel and is injected via [`set_kvm_probe`]. With no probe installed,
//! [`register_adapter`] fails closed (native-process adapters are refused),
//! which is the correct no_std posture: a node that cannot isolate must not
//! accept an unsandboxed native adapter.

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

/// The host-capability probe signature (`true` = the host can back a microVM
/// with hardware virtualization).
pub type KvmProbeFn = fn() -> bool;

static KVM_PROBE: crate::spinlock::SpinLock<Option<KvmProbeFn>> =
    crate::spinlock::SpinLock::new(None);

/// Inject the host KVM probe (called once by the kernel's `isolation::init`).
/// No probe installed = fail-closed (native-process adapters are refused).
pub fn set_kvm_probe(f: KvmProbeFn) {
    if let Ok(mut g) = KVM_PROBE.lock() {
        *g = Some(f);
    }
}

/// Whether the host can back a microVM with hardware virtualization.
/// `false` unless the kernel has injected a probe that reports `true`.
pub fn kvm_available() -> bool {
    if let Ok(g) = KVM_PROBE.lock() {
        if let Some(f) = *g {
            return f();
        }
    }
    false
}

/// Register an adapter against the fail-closed isolation gate.
///
/// - `execution == "wasm-component"` → always `Ok(())` (capability-scoped, no
///   KVM dependency).
/// - `execution == "native-process"` → `Ok(())` **only** if the host probe
///   reports KVM available; otherwise `Err(AdapterRejected::HostCannotIsolate)`.
///   There is deliberately **no fallback** to running the adapter unsandboxed.
/// - any other value → `Err(AdapterRejected::UnknownExecution)`.
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
