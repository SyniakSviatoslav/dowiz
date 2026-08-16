//! Item 64 — capability-secure declarative composition root (production wiring).
//!
//! std host shim. The pure init-DAG / order-derivation / capability-check core
//! (`NodeId`, `Capability`, `InitNode`, `DAG`, `InitError`, `init_order`,
//! `check_capabilities`, `CoreWriteToken`) lives in `dowiz_core::compose` and is
//! re-exported below. This module keeps the std-only durable-store wiring:
//!
//!   (i)  [`BootConfig`] (operator-supplied `PathBuf` store path),
//!   (ii) [`ProductionRoot`] — the live `EventLog<FileEventStore>` the production
//!        `boot` constructs (item 2's proof condition),
//!   (iii) [`boot`] — derive order → fail-closed capability check → open the
//!        durable `FileEventStore` → wrap in `EventLog` → `verify_chain`.
//!
//! This is the ALWAYS-COMPILED floor that closes item 2's proven defect: until
//! now *no production code path* constructed the durable `FileEventStore`/`EventLog`
//! — every construction site lived under `#[cfg(test)]` or a `tests/` binary. The
//! organism's audit trail was correct, tested, and unreachable.

pub use dowiz_core::compose::*;

use std::path::PathBuf;

use crate::event_log::{ChainDefect, EventLog};
use crate::hydra::FileEventStore;

// NOTE: no `impl std::error::Error for InitError {}` is needed (or legal) here.
// `InitError` is defined in `dowiz_core` (foreign to this crate), and `std`'s
// `Error` is `core::error::Error` — implementing a foreign trait for a foreign
// type would trip the orphan rule (E0117). The core's
// `impl core::error::Error for InitError` already makes `InitError` satisfy
// `std::error::Error` in this std host.

// ---------------------------------------------------------------------------
// Production boot — the ONLY non-test constructor of the durable store
// ---------------------------------------------------------------------------

/// Operator-supplied boot configuration. The store path is the one operator
/// decision point the blueprint flags (item 64 §7); this root does not choose it.
#[derive(Debug, Clone)]
pub struct BootConfig {
    /// Filesystem path for the durable `FileEventStore` (audit log).
    pub store_path: PathBuf,
}

/// The production composition result. Holds the live durable `EventLog` — the
/// exact value item 2's proof condition required a production binary to construct.
pub struct ProductionRoot {
    log: EventLog<FileEventStore>,
}

impl ProductionRoot {
    /// Borrow the wired durable audit log.
    pub fn log(&self) -> &EventLog<FileEventStore> {
        &self.log
    }

    /// Borrow the wired durable audit log mutably (for appending).
    pub fn log_mut(&mut self) -> &mut EventLog<FileEventStore> {
        &mut self.log
    }
}

impl core::fmt::Debug for ProductionRoot {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ProductionRoot")
            .field("events", &self.log.len())
            .finish()
    }
}

impl ProductionRoot {
    /// Re-verify the durable chain on demand (the same check `boot` ran).
    pub fn verify_chain(&self) -> Result<(), ChainDefect> {
        self.log.verify_chain()
    }

    /// Item 65 seam: the ONLY site that may mint a `CoreWriteToken`. Visibility is
    /// the enforcement — `pub(crate)` so only `compose/` (and item 65, once it
    /// lands here) can construct the zero-sized capability.
    #[allow(dead_code)] // consumed by item 65 (sole minter) — seam until then.
    pub(crate) fn mint_core_write(&self) -> CoreWriteToken {
        CoreWriteToken
    }
}

/// Build the production composition root: derive the init order, fail-closed
/// capability check, open the durable `FileEventStore`, wrap it in `EventLog`,
/// verify the chain, and surface `StoreIo`/chain defects instead of swallowing.
pub fn boot(cfg: &BootConfig) -> Result<ProductionRoot, InitError> {
    let order = init_order(DAG)?;
    check_capabilities(DAG, &order)?;

    // Walk the derived order, running each node's constructor. Only `DurableStore`
    // produces a value; the others operate on / verify it (fail-closed).
    let mut log: Option<EventLog<FileEventStore>> = None;
    let mut minted = false;
    for &id in &order {
        match id {
            NodeId::DurableStore => {
                // The exact line the wiring-gap blueprint says is missing everywhere:
                // a PRODUCTION (non-#[cfg(test)]) construction of the durable store.
                let store = FileEventStore::open(&cfg.store_path)
                    .map_err(|e| InitError::StoreIo(e.to_string()))?;
                log = Some(EventLog::new(store));
            }
            NodeId::AuditChain => {
                // Re-verify the durable chain before it is trusted (item 48 home).
                log.as_ref()
                    .expect("AuditChain runs after DurableStore (order is DAG-derived)")
                    .verify_chain()
                    .map_err(InitError::ChainBroken)?;
            }
            NodeId::CapabilityMint => {
                // Sole minter seam (deliverable (iv)). No-op beyond recording that
                // the token could be minted here; item 65 will consume `mint_core_write`.
                minted = true;
            }
        }
    }
    debug_assert!(minted, "CapabilityMint must be reached in a valid DAG");
    debug_assert!(
        log.is_some(),
        "DurableStore must have produced the live EventLog"
    );
    Ok(ProductionRoot {
        log: log.expect("post-condition: durable store constructed in derived order"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unique temp path so parallel test runs never collide on the durable file.
    fn temp_store_path(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let pid = std::process::id();
        std::env::temp_dir().join(format!("dowiz-compose-{tag}-{pid}-{nanos}.log"))
    }

    /// PRIMARY PROOF (item 2's discharge): a production (`boot`, non-`#[cfg(test)]`)
    /// path constructs the durable store and yields a live `EventLog<FileEventStore>`.
    /// This is the cited production line the wiring-gap blueprint required.
    #[test]
    fn production_composition_root_builds_durable_store() {
        let path = temp_store_path("prod");
        let _ = crate::vfs::remove_file(&path); // start clean
        let cfg = BootConfig {
            store_path: path.clone(),
        };

        let root = boot(&cfg).expect("composition root must boot a production store");
        // The log exists and is empty (fresh durable file, chain verified).
        assert!(root.log().is_empty(), "fresh durable store must be empty");
        assert_eq!(root.log().len(), 0);

        // The durable store is REAL: appending an event persists it and a second
        // boot (replay) must see the same event — proving non-test durability.
        use crate::event_log::MeshEvent;
        let ev = MeshEvent {
            prev: [0u8; 32],
            actor_pubkey: [7u8; 32],
            actor_seq: 1,
            payload: b"composition-root-boot".to_vec(),
        };
        let id = {
            let mut root = root;
            let out = root
                .log_mut()
                .append(ev)
                .expect("durable append must succeed");
            match out {
                crate::event_log::AppendOutcome::Committed(id) => id,
                crate::event_log::AppendOutcome::Duplicate(id) => id,
            }
        };

        // Re-open via a fresh boot (simulates a restart) — replay must recover it.
        let root2 = boot(&cfg).expect("re-boot must replay the durable store");
        assert!(
            !root2.log().is_empty(),
            "replayed durable store must not be empty"
        );
        assert_eq!(
            root2.log().len(),
            1,
            "exactly one durable event must survive"
        );
        assert!(
            root2.log().contains(&id),
            "committed event id must survive replay"
        );
        // And the chain must still verify after replay.
        root2
            .verify_chain()
            .expect("replayed durable chain must verify");

        let _ = crate::vfs::remove_file(&path);
    }

    /// Acceptance #1 — grep-verifiable: a production path constructs the store.
    /// (The string the wiring-gap falsifier greps for lives in `boot` above; this
    /// assertion pins that `boot` returns a concrete `EventLog<FileEventStore>`.)
    #[test]
    fn boot_returns_concrete_event_log_type() {
        let path = temp_store_path("concrete");
        let _ = crate::vfs::remove_file(&path);
        let root = boot(&BootConfig {
            store_path: path.clone(),
        })
        .expect("boot");
        // Compile-time + run-time proof the value IS `EventLog<FileEventStore>`.
        fn assert_type(_: &EventLog<FileEventStore>) {}
        assert_type(root.log());
        let _ = crate::vfs::remove_file(&path);
    }

    /// Acceptance #2 — a planted cyclic DAG must fail closed, not boot.
    #[test]
    fn cyclic_init_dependency_refuses_boot() {
        // A <-> B cycle: B requires A, A requires B.
        const CYCLE: &[InitNode] = &[
            InitNode {
                id: NodeId::AuditChain,
                requires: &[NodeId::CapabilityMint],
                requires_caps: &[],
                provides: &[Capability::VerifiedAuditChain],
            },
            InitNode {
                id: NodeId::CapabilityMint,
                requires: &[NodeId::AuditChain],
                requires_caps: &[],
                provides: &[Capability::CoreWriteToken],
            },
        ];
        let err = boot_with_dag(CYCLE).expect_err("cyclic DAG must fail boot");
        assert!(
            matches!(err, InitError::CyclicDependency(_)),
            "expected CyclicDependency, got {err:?}"
        );
    }

    /// Helper: boot against an arbitrary DAG (used by the cycle test).
    fn boot_with_dag(dag: &[InitNode]) -> Result<ProductionRoot, InitError> {
        let order = init_order(dag)?;
        check_capabilities(dag, &order)?;
        let cfg = BootConfig {
            store_path: temp_store_path("cycle"),
        };
        let store =
            FileEventStore::open(&cfg.store_path).map_err(|e| InitError::StoreIo(e.to_string()))?;
        Ok(ProductionRoot {
            log: EventLog::new(store),
        })
    }
}
