//! cache.rs — SH-2: per-agent cache partitioning, default-deny.
//!
//! By DEFAULT each admitted agent gets a GENUINELY SEPARATE `AgentCache` instance (its
//! own store, capacity, and lock) — NOT a shared store with a keyspace prefix. A separate
//! instance has no shared mutable state between tenants, so it closes the cross-tenant
//! request-existence timing oracle AND the eviction/contention side channels a keyspace
//! prefix would leave open (F6).
//!
//! The single legitimate reason to SHARE a cache — dedup between agents the operator
//! knows are mutually trusting — is opt-in via an OPERATOR-SIGNED `cache_group_id`, read
//! from operator config, NEVER from the manifest (self-selection into a co-tenant's
//! keyspace is the confused-deputy risk). Absent an explicit operator co-scope, the answer
//! is always a separate instance (fail-closed). Note there is deliberately NO cache-group
//! axis in the manifest lattice (`config_axis_domain(0x05) == None`), so an agent CANNOT
//! even encode a self-declared group.
//!
//! Cacheability follows the LLM `CachePolicy` discipline: idempotent `read_resource` calls
//! are cacheable (`Exact`); tool invocations default to `NoCache`.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use dowiz_kernel::event_log::sha3_256;
use dowiz_kernel::ports::agent::AgentTask;

/// A single agent's private response cache (content-addressed by `sha3_256`). Hit/miss
/// counters make the SH-2 partitioning falsifiable WITHOUT timing (crit 13).
pub struct AgentCache {
    store: Mutex<HashMap<[u8; 32], Vec<u8>>>,
    hits: AtomicUsize,
    misses: AtomicUsize,
}

impl Default for AgentCache {
    fn default() -> Self {
        AgentCache {
            store: Mutex::new(HashMap::new()),
            hits: AtomicUsize::new(0),
            misses: AtomicUsize::new(0),
        }
    }
}

impl AgentCache {
    /// A fresh, empty private cache.
    pub fn new() -> Self {
        AgentCache::default()
    }

    /// Whether a task is cacheable: idempotent `read_resource` under `Exact`; tool
    /// invocations are `NoCache`.
    pub fn is_cacheable(task: &AgentTask) -> bool {
        matches!(
            task,
            AgentTask::ReadResource { .. } | AgentTask::RenderPrompt { .. }
        )
    }

    /// Canonical key for a cacheable request.
    pub fn key(canonical_request: &[u8]) -> [u8; 32] {
        sha3_256(canonical_request)
    }

    /// Look up a cached response, counting hit/miss.
    pub fn get(&self, key: &[u8; 32]) -> Option<Vec<u8>> {
        let found = self.store.lock().unwrap().get(key).cloned();
        if found.is_some() {
            self.hits.fetch_add(1, Ordering::SeqCst);
        } else {
            self.misses.fetch_add(1, Ordering::SeqCst);
        }
        found
    }

    /// Insert a response (idempotent).
    pub fn put(&self, key: [u8; 32], value: Vec<u8>) {
        self.store.lock().unwrap().insert(key, value);
    }

    /// Total hits observed.
    pub fn hits(&self) -> usize {
        self.hits.load(Ordering::SeqCst)
    }
    /// Total misses observed.
    pub fn misses(&self) -> usize {
        self.misses.load(Ordering::SeqCst)
    }
}

/// Provisions each admitted agent its cache. DEFAULT: a separate private instance. A
/// shared instance is granted ONLY when the OPERATOR co-scopes agents under a signed
/// `cache_group_id` — never from the manifest.
#[derive(Default)]
pub struct CacheProvisioner {
    /// Operator-signed shared groups. Populated ONLY from operator config.
    groups: HashMap<u16, Arc<AgentCache>>,
}

impl CacheProvisioner {
    /// Empty provisioner.
    pub fn new() -> Self {
        CacheProvisioner::default()
    }

    /// Provision an agent's cache. `operator_group` comes from OPERATOR-SIGNED config
    /// (never the manifest): `Some(g)` co-scopes into the shared instance for group `g`;
    /// `None` (the fail-closed default) yields a fresh private instance.
    pub fn provision(&mut self, operator_group: Option<u16>) -> Arc<AgentCache> {
        match operator_group {
            Some(g) => self
                .groups
                .entry(g)
                .or_insert_with(|| Arc::new(AgentCache::new()))
                .clone(),
            None => Arc::new(AgentCache::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dowiz_kernel::ports::agent::config_axis_domain;

    // ── §4 criterion 13 — per-agent cache by default; sharing is operator-only ──
    #[test]
    fn crit13_no_cross_agent_hit_by_default() {
        let mut prov = CacheProvisioner::new();
        // Two agents, NO operator co-scope ⇒ separate instances.
        let a = prov.provision(None);
        let b = prov.provision(None);
        let key = AgentCache::key(b"identical-canonical-request");
        a.put(key, b"answer".to_vec());
        // Agent B issues the IDENTICAL request: a MISS (separate store) — verified by the
        // store's own miss counter, NOT by timing.
        assert_eq!(b.get(&key), None, "no cross-agent hit by default");
        assert_eq!(b.misses(), 1);
        assert_eq!(a.hits(), 0, "A's store served nothing to B");
    }

    #[test]
    fn crit13_shared_only_via_operator_group() {
        let mut prov = CacheProvisioner::new();
        // Operator co-scopes both agents under group 7 (operator-signed, not manifest).
        let a = prov.provision(Some(7));
        let b = prov.provision(Some(7));
        let key = AgentCache::key(b"identical-canonical-request");
        a.put(key, b"answer".to_vec());
        assert_eq!(
            b.get(&key),
            Some(b"answer".to_vec()),
            "operator co-scope ⇒ shared dedup"
        );
        assert_eq!(b.hits(), 1);
    }

    #[test]
    fn crit13_self_declared_group_is_structurally_impossible() {
        // A manifest cannot even ENCODE a cache group: there is no cache-group config axis
        // (0x05 is unknown ⇒ decode error). Membership is operator config only. This is the
        // structural form of "a manifest that self-declares a shared group is refused."
        assert_eq!(
            config_axis_domain(0x05),
            None,
            "no cache-group axis in the manifest lattice"
        );
    }

    #[test]
    fn tool_invokes_are_nocache_reads_are_cacheable() {
        assert!(!AgentCache::is_cacheable(&AgentTask::InvokeTool {
            name: "x".into(),
            args: vec![]
        }));
        assert!(AgentCache::is_cacheable(&AgentTask::ReadResource {
            uri: "u".into()
        }));
    }
}
