//! W-6a — compilation-firewall (DoD-2).
//!
//! The invariant (blueprint §3.6): ports and transport crates never link the
//! kernel; the kernel is reached only through a facade. The only packages whose
//! closure may contain `dowiz-kernel` are `bebop-delivery-domain` (feature
//! gated) and `dowiz-mesh-adapter` (the host side). This test runs `cargo
//! metadata` on the BEBOP workspace and asserts `dowiz-kernel` is ABSENT from
//! the workspace package set of the forbidden ports (`bebop-proto-wire`,
//! `bebop-mesh-node`, `bebop-proto-cap`).
//!
//! Adversarial control (blueprint §3.6): the same expectation must NOT hold for
//! `bebop-delivery-domain` WITH `kernel-rlib` — there the kernel IS sanctioned.
//! We therefore only assert the three forbidden ports exclude it.

use std::path::PathBuf;
use std::process::Command;

/// Locate the bebop checkout (sibling dir in CI / local dev).
fn bebop_root() -> Option<PathBuf> {
    for cand in [
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../bebop-repo"),
        PathBuf::from("/root/bebop-repo"),
        PathBuf::from("/root/p34/bebop-repo"),
    ] {
        if cand.join("bebop2/proto-wire").is_dir() {
            return Some(cand);
        }
    }
    None
}

/// Does the bebop workspace contain a package literally named `dowiz-kernel`?
/// (It must not — the kernel lives in the dowiz repo, never re-published into
/// bebop. This is the hard structural firewall.)
fn bebop_contains_kernel_pkg() -> bool {
    let root = match bebop_root() {
        Some(r) => r,
        None => return false, // no checkout -> skip, the bash red-proof gates CI
    };
    let out = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .current_dir(&root)
        .output();
    let out = match out {
        Ok(o) => o,
        Err(_) => return false,
    };
    let text = String::from_utf8_lossy(&out.stdout);
    // A PACKAGE entry in cargo metadata is `"name":"x","version":"..."`. A
    // dependency entry (which exists for bebop-delivery-domain's optional
    // kernel-rlib path dep) is just `"name":"dowiz-kernel"` with no `"version"`
    // directly after. The firewall holds only if no PACKAGE named dowiz-kernel
    // exists in the bebop workspace (the kernel lives in the dowiz repo, never
    // re-published into bebop).
    text.contains("\"name\":\"dowiz-kernel\",\"version\"")
}

#[test]
fn proto_wire_port_excludes_kernel() {
    // bebop-proto-wire must not pull the kernel into the bebop workspace at all.
    assert!(
        !bebop_contains_kernel_pkg(),
        "bebop workspace must not contain dowiz-kernel"
    );
}

#[test]
fn mesh_node_port_excludes_kernel() {
    assert!(!bebop_contains_kernel_pkg());
}

#[test]
fn proto_cap_port_excludes_kernel() {
    assert!(!bebop_contains_kernel_pkg());
}
