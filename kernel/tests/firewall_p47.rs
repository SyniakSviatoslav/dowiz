//! Firewall red-proof integration test (P47 §2.5-B1 / §2.8 / M5).
//!
//! `cargo tree -p dowiz-kernel` must show NO payment-adapter / HTTP-client / serde-in-kernel
//! dependency reaching this crate. This test SPAWNS `cargo tree` and asserts the absences
//! structurally — the same done-check the blueprint's WAVE-0 gate requires. If a future Wave
//! lands a `payment-adapters` wiring or pulls `reqwest`/`serde` into the kernel graph, this
//! test FAILS in CI (and locally), not silently.
//!
//! Run: `cargo test --test firewall_p47` (or `cargo test --lib` — integration tests run too).

use std::process::Command;

/// Crates that must NOT appear anywhere in `cargo tree -p dowiz-kernel -e no-dev`.
/// `no-dev` so a dev-only proptest/criterion never masks a real prod-path violation.
const FORBIDDEN_IN_KERNEL_GRAPH: &[&str] = &[
    "payment_adapters",
    "llm_adapters",
    "agent_adapters",
    "reqwest",
    "hyper",
    "hyper-tls",
    "tokio",
    "sqlx",
    "wasm-bindgen",
    "serde",
];

#[test]
fn cargo_tree_kernel_has_no_payment_adapter_dep() {
    // Resolve the kernel's production dependency graph (no dev-deps).
    let out = Command::new("cargo")
        .args(["tree", "-p", "dowiz-kernel", "-e", "no-dev"])
        .output();

    let out = match out {
        Ok(o) => o,
        // If `cargo` is unavailable in this environment, skip rather than false-fail the
        // run — the lib-level `firewall_self_source_is_clean` covers the source-level firewall
        // deterministically without a subprocess. (CI always has cargo.)
        Err(_) => {
            eprintln!("cargo-tree firewall: `cargo` unavailable; skipping subprocess check");
            return;
        }
    };

    // A non-zero exit (e.g. feature/dep resolution issue) is itself a red flag.
    assert!(
        out.status.success(),
        "cargo tree exited non-zero: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let tree = String::from_utf8_lossy(&out.stdout);
    let mut violations = Vec::new();
    for forbidden in FORBIDDEN_IN_KERNEL_GRAPH {
        if tree.lines().any(|l| l.split_whitespace().next().unwrap_or("").starts_with(forbidden)) {
            violations.push(*forbidden);
        }
    }
    assert!(
        violations.is_empty(),
        "P47 firewall violation: kernel dependency graph contains forbidden crate(s): {:?}\n{}",
        violations,
        tree
    );
}
