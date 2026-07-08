//! Bebop core — the deterministic agent kernel, built ON the `dowiz-core` invariant core.
//!
//! OS-agnostic and hard-deterministic by construction:
//!   - No clock, no RNG, no network in any decision path (Laws 1–3 of the sovereign core).
//!   - The command hash is a pure FNV-1a over the canonical command bytes (no uuid-v4 / entropy).
//!   - Every decision is `kernel::decide` / `kernel::validate` / `kernel::fold` / `kernel::replay`
//!     — the same single door the product uses. The agent core adds NO money number and NO new
//!     transition; it only sequences commands through the door and records the immutable envelope log.
//!
//! This is the "device" the operator asked for: a self-contained, replayable, deterministic core
//! that any OS can host (the CLI in `main.rs` is just one host; a WASM/embedded host would be identical).

use domain::{
    Actor, Command, CommandHash, Context, Envelope, Event, OrderState, Ts,
    canonical_bytes, decide, fold, from_bytes, replay_envelopes, validate,
};

/// A deterministic, non-cryptographic command hash. Pure FNV-1a over the canonical command bytes.
/// Chosen over uuid-v4 because the core must not pull an entropy source (sovereignty Law 2) — the
/// log only CARRIES the hash; determinism of the log is what matters, not collision resistance.
pub fn command_hash(cmd: &Command) -> CommandHash {
    let bytes = canonical_bytes(cmd).expect("Command is always serializable");
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in &bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    CommandHash(format!("{hash:016x}"))
}

/// The result of attempting one command through the door.
pub struct Step {
    pub seq: u64,
    pub cause: CommandHash,
    pub events: Vec<Event>,
    /// Every logical invariant the boundary caught (empty = clean). Mirrors `validate`'s contract.
    /// A non-empty `violations` means NO event was emitted — the command was refused at the gate.
    pub violations: Vec<String>,
    pub state_after: OrderState,
}

/// A running, replayable agent session over one order. Pure — no IO, no time.
pub struct Core {
    state: OrderState,
    log: Vec<Envelope>,
    next_seq: u64,
}

impl Core {
    pub fn new() -> Self {
        Core {
            state: OrderState::genesis(),
            log: Vec::new(),
            next_seq: 0,
        }
    }

    /// Apply a command through the door. Returns the step. A rejected command emits NO event (the
    /// immutable log holds only facts); the rejection is carried on `Step.violations`.
    pub fn apply(&mut self, cmd: Command, ctx: &Context) -> Step {
        let cause = command_hash(&cmd);
        // 1. Validation Layer — the invariant gate BEFORE decide (returns every violation as data).
        let violations = match validate(&cmd, &self.state, ctx) {
            Ok(()) => Vec::new(),
            Err(invs) => invs.iter().map(|i| format!("{i:?}")).collect(),
        };
        let mut events = Vec::new();
        if violations.is_empty() {
            // 2. The single door — decide emits the events (or refuses with a DomainError).
            if let Ok(ev) = decide(&self.state, cmd, ctx) {
                for e in &ev {
                    self.state = fold(&self.state, e);
                    events.push(e.clone());
                }
                // 3. Record only the produced facts — the log is the truth.
                let seq = self.next_seq;
                self.next_seq += 1;
                for e in &events {
                    self.log.push(Envelope {
                        seq,
                        at: event_at(e),
                        cause: cause.clone(),
                        event: e.clone(),
                    });
                }
            }
        }
        Step {
            seq: self.next_seq.saturating_sub(1),
            cause,
            events,
            violations,
            state_after: self.state,
        }
    }

    /// Reconstruct the core from a canonical log (cross-node replication / persistence replay).
    pub fn from_log(bytes: &[u8]) -> Result<Self, String> {
        let envelopes: Vec<Envelope> = from_bytes(bytes).map_err(|e| e.to_string())?;
        let state = replay_envelopes(OrderState::genesis(), &envelopes);
        let next_seq = envelopes.iter().map(|e| e.seq).max().map(|m| m + 1).unwrap_or(0);
        Ok(Core {
            state,
            log: envelopes,
            next_seq,
        })
    }

    /// Export the immutable log as canonical bytes (deterministic — same session ⇒ same bytes).
    pub fn export_log(&self) -> Vec<u8> {
        canonical_bytes(&self.log).expect("log is always serializable")
    }

    pub fn state(&self) -> &OrderState {
        &self.state
    }

    pub fn log(&self) -> &[Envelope] {
        &self.log
    }
}

impl Default for Core {
    fn default() -> Self {
        Self::new()
    }
}

/// Read the timestamp off an event where one exists (StatusChanged carries `at`).
fn event_at(e: &Event) -> Ts {
    match e {
        Event::StatusChanged { at, .. } => *at,
        _ => Ts(0),
    }
}

// Re-export for hosts that want to build context/commands directly.
pub use domain::{Actor as BebopActor, Context as BebopContext, OrderStatus};
