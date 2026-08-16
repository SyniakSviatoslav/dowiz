//! Item 64 — capability-secure declarative composition root (production wiring).
//!
//! no_std pure core. This module holds the parts of the composition root that are
//! pure data / pure derivation and therefore safe in `no_std`:
//!
//!   (i)  the init order declared as a **DAG of plain data** (`NodeId`/`Capability`/
//!        `InitNode`/`DAG` — source order is irrelevant, the real order is derived
//!        by topological sort, never by declaration order),
//!   (ii) [`init_order`] — Kahn topo-sort + cycle detection (fail-closed: a cyclic
//!        declaration is a typed `InitError::CyclicDependency`, never a boot),
//!   (iii) [`check_capabilities`] — fail-closed capability check over the derived
//!        order (a node requiring a capability no upstream node provides refuses),
//!   (iv)  the `InitError` vocabulary — with `StoreIo(String)` (the rendered io
//!        error string, so the enum stays `no_std`-compatible), its `Display`
//!        impl, and its `core::error::Error` impl (the no_std analogue of
//!        `std::error::Error`, which it satisfies transitively),
//!   (v)  [`CoreWriteToken`] — the zero-sized in-process write capability (item 65).
//!
//! The std-only durable-store wiring (`BootConfig`/`ProductionRoot`/`boot`, i.e.
//! the `FileEventStore`/`EventLog<FileEventStore>` construction) lives in the
//! kernel shim `dowiz_kernel::compose`, which starts with
//! `pub use dowiz_core::compose::*;` and adds the std-only constructor.

use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;

use crate::event_log::ChainDefect;

// ---------------------------------------------------------------------------
// Closed id/capability vocabulary (scope.rs discipline: pinned discriminants)
// ---------------------------------------------------------------------------

/// Init-graph node id. A closed enum with pinned `repr(u8)` discriminants so a
/// reorder/rename is a mechanically-caught diff (not a silent renumber).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NodeId {
    /// Opens the durable `FileEventStore` and wraps it in an `EventLog`.
    DurableStore = 0,
    /// Re-verifies the durable chain before it is trusted (`verify_chain`).
    AuditChain = 1,
    /// Sole minter of item 65's in-process capability tokens.
    CapabilityMint = 2,
}

/// Capabilities a node PROVIDES on init, or REQUIRES before it may init. Closed
/// enum — a capability no upstream node provides is a fail-closed startup error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Capability {
    /// A live, durable `EventLog<FileEventStore>` exists.
    DurableEventLog,
    /// The durable chain has been verified (genesis reachable, no fork/cycle).
    VerifiedAuditChain,
    /// The root has minted a zero-sized `CoreWriteToken` (item 65 seam).
    CoreWriteToken,
}

/// One node in the init DAG: its id, the nodes it must come *after*, and the
/// capabilities it requires / provides. Pure data — no runtime graph mutation.
#[derive(Debug, Clone, Copy)]
pub struct InitNode {
    pub id: NodeId,
    /// Predecessor node ids (declares the ordering edges).
    pub requires: &'static [NodeId],
    /// Capabilities this node needs satisfied *before* it inits.
    pub requires_caps: &'static [Capability],
    /// Capabilities this node makes available to successors.
    pub provides: &'static [Capability],
}

/// The declared production init DAG. Order here is **meaningless** — `init_order`
/// derives the real sequence from the edges. (Acceptance #3: a permuted
/// declaration yields the identical derived order.)
///
/// `pub` (rather than the original private `const`) so the kernel shim's `boot`
/// can feed it to [`init_order`]/[`check_capabilities`] — the DAG remains the
/// single source of truth, declared once here in the core.
pub const DAG: &[InitNode] = &[
    InitNode {
        id: NodeId::DurableStore,
        requires: &[],
        requires_caps: &[],
        provides: &[Capability::DurableEventLog],
    },
    InitNode {
        id: NodeId::AuditChain,
        requires: &[NodeId::DurableStore],
        requires_caps: &[Capability::DurableEventLog],
        provides: &[Capability::VerifiedAuditChain],
    },
    InitNode {
        id: NodeId::CapabilityMint,
        requires: &[NodeId::AuditChain],
        requires_caps: &[Capability::VerifiedAuditChain],
        provides: &[Capability::CoreWriteToken],
    },
];

// ---------------------------------------------------------------------------
// Init errors (fail-closed: absence is a typed startup error, not a None)
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum InitError {
    /// A cyclic `requires` edge was declared — init order is undefined.
    CyclicDependency(NodeId),
    /// A node required a capability no already-initialized upstream node provides.
    CapabilityAbsent {
        node: NodeId,
        capability: Capability,
    },
    /// The durable store could not be opened (IO / permission / fsync surface).
    /// Carries the rendered error string (not the non-`no_std` `io::Error`
    /// itself) so the enum stays core-compatible; the kernel shim renders the
    /// `io::Error` via `e.to_string()` at the open site.
    StoreIo(String),
    /// The durable chain failed verify-chain (corruption / fork / cycle at rest).
    ChainBroken(ChainDefect),
}

impl core::fmt::Display for InitError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            InitError::CyclicDependency(n) => write!(f, "cyclic init dependency at {n:?}"),
            InitError::CapabilityAbsent { node, capability } => {
                write!(
                    f,
                    "node {node:?} missing required capability {capability:?}"
                )
            }
            InitError::StoreIo(e) => write!(f, "durable store open failed: {e}"),
            InitError::ChainBroken(d) => write!(f, "durable chain broken: {d:?}"),
        }
    }
}

// `core::error::Error` is the no_std analogue of `std::error::Error` (they are
// the SAME trait — `std` re-exports `core`'s). Implementing it here keeps
// `InitError` usable as `Box<dyn std::error::Error>` in std hosts WITHOUT the
// kernel shim having to add `impl std::error::Error` for a *foreign* type,
// which the orphan rule (E0117) forbids.
impl core::error::Error for InitError {}

// ---------------------------------------------------------------------------
// Pure graph derivation (Kahn topo-sort + cycle detection over the node DAG)
// ---------------------------------------------------------------------------

/// Map a node id to its index in `dag` (the DAG is the source of truth for ids).
fn index_of(dag: &[InitNode], id: NodeId) -> usize {
    dag.iter()
        .position(|n| n.id == id)
        .expect("NodeId referenced by an edge must exist in the DAG")
}

/// Derive the init order from the DAG edges (deliverable (i)/(iii)). A cyclic
/// declaration returns `InitError::CyclicDependency` — the boot fails closed, the
/// cycle is never "successfully" booted. Order is a pure function of the edges,
/// independent of declaration order (acceptance #3).
pub fn init_order(dag: &[InitNode]) -> Result<Vec<NodeId>, InitError> {
    let n = dag.len();
    let mut indeg = vec![0usize; n];
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, node) in dag.iter().enumerate() {
        for &req in node.requires {
            let j = index_of(dag, req);
            adj[j].push(i);
            indeg[i] += 1;
        }
    }
    // Stable ascending source queue ⇒ deterministic, lowest-id-first order.
    let mut queue: Vec<usize> = (0..n).filter(|&i| indeg[i] == 0).collect();
    let mut order: Vec<NodeId> = Vec::with_capacity(n);
    while let Some(u) = queue.first().copied() {
        queue.remove(0);
        order.push(dag[u].id);
        let mut ready = Vec::new();
        for &v in &adj[u] {
            indeg[v] -= 1;
            if indeg[v] == 0 {
                ready.push(v);
            }
        }
        queue.extend(ready);
        queue.sort_unstable(); // keep ascending ⇒ order invariant under permutation
    }
    if order.len() != n {
        // At least one node remains with non-zero in-degree ⇒ a cycle.
        let stuck = dag
            .iter()
            .enumerate()
            .find(|(i, _)| indeg[*i] != 0)
            .map(|(_, n)| n.id)
            .unwrap_or(NodeId::DurableStore);
        return Err(InitError::CyclicDependency(stuck));
    }
    Ok(order)
}

/// Fail-closed capability check (deliverable (ii)): walk the derived order and
/// assert every node's `requires_caps` were `provides`-satisfied by an already
/// initialized upstream node. Absence ⇒ `InitError::CapabilityAbsent`.
pub fn check_capabilities(dag: &[InitNode], order: &[NodeId]) -> Result<(), InitError> {
    let mut provided: BTreeSet<Capability> = BTreeSet::new();
    for &id in order {
        let node = &dag[index_of(dag, id)];
        for &cap in node.requires_caps {
            if !provided.contains(&cap) {
                return Err(InitError::CapabilityAbsent {
                    node: id,
                    capability: cap,
                });
            }
        }
        for &cap in node.provides {
            provided.insert(cap);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// In-process write capability (item 65 token — minted solely by the root)
// ---------------------------------------------------------------------------

/// Zero-sized in-process write capability (item 65 token — minted solely by the
/// root). Defined here as the seam; item 65 attaches the real attenuation model.
/// The token TYPE is public so item 65 (and callers) can name it; the minting
/// function (`ProductionRoot::mint_core_write`, in the kernel shim) is the
/// visibility-gated sole constructor.
#[derive(Debug, Clone, Copy)]
pub struct CoreWriteToken;

#[cfg(test)]
mod tests {
    use super::*;

    /// Acceptance #3 — a permuted declaration yields the identical derived order.
    #[test]
    fn permuted_declaration_yields_identical_order() {
        const PERMUTED: &[InitNode] = &[
            InitNode {
                id: NodeId::CapabilityMint,
                requires: &[NodeId::AuditChain],
                requires_caps: &[Capability::VerifiedAuditChain],
                provides: &[Capability::CoreWriteToken],
            },
            InitNode {
                id: NodeId::AuditChain,
                requires: &[NodeId::DurableStore],
                requires_caps: &[Capability::DurableEventLog],
                provides: &[Capability::VerifiedAuditChain],
            },
            InitNode {
                id: NodeId::DurableStore,
                requires: &[],
                requires_caps: &[],
                provides: &[Capability::DurableEventLog],
            },
        ];
        let a = init_order(DAG).unwrap();
        let b = init_order(PERMUTED).unwrap();
        assert_eq!(a, b, "order must come from the DAG, not declaration order");
        assert_eq!(
            a,
            vec![
                NodeId::DurableStore,
                NodeId::AuditChain,
                NodeId::CapabilityMint
            ]
        );
    }

    /// Acceptance #4 — a node requiring an unsatisfied capability refuses init.
    #[test]
    fn unsatisfied_capability_refuses_boot() {
        // CapabilityMint requires CoreWriteToken, but nothing provides it.
        const UNDERPROVISIONED: &[InitNode] = &[
            InitNode {
                id: NodeId::DurableStore,
                requires: &[],
                requires_caps: &[],
                provides: &[Capability::DurableEventLog],
            },
            InitNode {
                id: NodeId::CapabilityMint,
                requires: &[NodeId::DurableStore],
                requires_caps: &[Capability::CoreWriteToken],
                provides: &[],
            },
        ];
        let order = init_order(UNDERPROVISIONED).unwrap();
        let err = check_capabilities(UNDERPROVISIONED, &order)
            .expect_err("unsatisfied capability must fail boot");
        assert!(
            matches!(
                err,
                InitError::CapabilityAbsent {
                    node: NodeId::CapabilityMint,
                    capability: Capability::CoreWriteToken
                }
            ),
            "expected CapabilityAbsent, got {err:?}"
        );
    }
}
