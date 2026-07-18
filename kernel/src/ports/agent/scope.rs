//! ports/agent/scope.rs — the CLOSED resource/action namespace the agent-bridge
//! admission path understands (B1 §2.1).
//!
//! Compile firewall (mirrors `ports/llm.rs:3-7`): ZERO network / HTTP / JSON / serde.
//! Discriminants are pinned bytes (never compiler-chosen) so the wire/signing mapping
//! is stable across compiler versions — the same discipline as bebop2 `scope.rs`.
//!
//! This is a PORT-LOCAL enum, generalizing the bebop2 `Resource`/`Action` closed set
//! for the agent-bridge admission decision. It intentionally does NOT link bebop2's
//! `proto-cap` (the dowiz kernel carries no such dependency); the real bebop2
//! `HybridGate`/`AnchorRoster` are injected at the integration boundary through the
//! `AdmissionGate`/`SignatureVerifier` seams (see `admission.rs` / `cap.rs`), exactly
//! as `ports/llm.rs` is a trait seam whose concrete backend lives elsewhere.
//!
//! ⚠ DISCRIMINANT-ALLOCATION CANON-DIFF (B1 DoD item 7, UNRATIFIED): B1 pins
//! `Resource::AgentBridge = 0x12` — the next byte after the bebop2 high-water mark
//! `Resource::Migration = 0x11`. B2 (`WorkReceipt`) independently proposed `0x12`, a
//! genuine collision the lead-agent Wave-0 discriminant ruling must resolve before any
//! shared `scope.rs` edit lands. `Action::{AdmitAgent, InvokeAgent}` were left
//! "currently-unnumbered" by the blueprint; provisional bytes `0x1F`/`0x20` are chosen
//! here (past bebop2's `0x18` high-water mark and B2's proposed `0x19–0x1E` action
//! block) and are FLAGGED as pending that same ruling. The `discriminants_are_pinned`
//! wire-stability test catches an accidental renumber mechanically.

/// A protocol resource an agent capability may target. Closed set ⇒ the admission
/// gate is exhaustively checkable; an unknown discriminant byte fails decode (fail-closed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Resource {
    /// A transport route / channel.
    Route,
    /// A ledger entry (append / read) — RED-LINE (money).
    Ledger,
    /// A restaurant / courier menu (catalog read).
    Menu,
    /// A customer order (create / read / mutate).
    Order,
    /// An analytics / reporting projection (read-only).
    Analytics,
    /// A customer / account record.
    Customer,
    /// A knowledge / embedding corpus (RAG).
    Corpus,
    /// Authentication / authority change — RED-LINE (auth).
    Auth,
    /// Secrets exposure — RED-LINE (secrets).
    Secret,
    /// Schema / data migration — RED-LINE (destructive bulk).
    Migration,
    /// **NEW (B1):** the agent-bridge admission surface. A capability scoped
    /// `(AgentBridge, AdmitAgent)` authorizes admitting an agent manifest;
    /// `(AgentBridge, InvokeAgent)` authorizes one delegated sub-agent invocation.
    /// Pinned `0x12` — see the CANON-DIFF note above.
    AgentBridge,
}

impl Resource {
    /// Explicit discriminant byte (pinned; not compiler-chosen).
    pub fn discriminant(&self) -> u8 {
        match self {
            Resource::Route => 0x01,
            Resource::Ledger => 0x02,
            Resource::Menu => 0x05,
            Resource::Order => 0x06,
            Resource::Analytics => 0x07,
            Resource::Customer => 0x08,
            Resource::Corpus => 0x09,
            Resource::Auth => 0x0F,
            Resource::Secret => 0x10,
            Resource::Migration => 0x11,
            Resource::AgentBridge => 0x12,
        }
    }

    /// Inverse of [`Resource::discriminant`]. `None` for unknown bytes ⇒ decode is
    /// fail-closed (no default/panic on a malformed scope).
    pub fn from_discriminant(b: u8) -> Option<Resource> {
        Some(match b {
            0x01 => Resource::Route,
            0x02 => Resource::Ledger,
            0x05 => Resource::Menu,
            0x06 => Resource::Order,
            0x07 => Resource::Analytics,
            0x08 => Resource::Customer,
            0x09 => Resource::Corpus,
            0x0F => Resource::Auth,
            0x10 => Resource::Secret,
            0x11 => Resource::Migration,
            0x12 => Resource::AgentBridge,
            _ => return None,
        })
    }

    /// Whether this resource is a RED-LINE class (money / auth / secrets / migrations).
    /// A manifest whose scope touches one of these is denied unless operator-allow-listed.
    pub fn is_red_line(&self) -> bool {
        matches!(
            self,
            Resource::Ledger | Resource::Auth | Resource::Secret | Resource::Migration
        )
    }
}

/// An action permitted on a [`Resource`]. Closed set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    /// Authorize a send on the resource.
    Send,
    /// Authorize a read/query of the resource.
    Read,
    /// Authorize an append/write to the resource.
    Append,
    /// Render a view / template (read-only presentation).
    Render,
    /// Create a new order (mutation).
    CreateOrder,
    /// Read a precomputed projection (read-only).
    ReadProjection,
    /// Settlement recorded (ledger i64) — RED-LINE.
    SettlementRecorded,
    /// Authenticate / change authority — RED-LINE.
    Authenticate,
    /// Deploy / expose a secret — RED-LINE.
    DeploySecret,
    /// Run a schema / data migration — RED-LINE.
    RunMigration,
    /// **NEW (B1):** admit an agent manifest (the admission-frame verb). Provisional
    /// `0x1F` — see the CANON-DIFF note above.
    AdmitAgent,
    /// **NEW (B1):** invoke a delegated sub-agent (the F10 depth-witness verb — each
    /// such link in a delegation chain is one unit of cryptographically-witnessed
    /// invocation depth). Provisional `0x20`.
    InvokeAgent,
}

impl Action {
    /// Explicit discriminant byte (pinned; not compiler-chosen).
    pub fn discriminant(&self) -> u8 {
        match self {
            Action::Send => 0x01,
            Action::Read => 0x02,
            Action::Append => 0x03,
            Action::Render => 0x04,
            Action::CreateOrder => 0x05,
            Action::ReadProjection => 0x06,
            Action::SettlementRecorded => 0x13,
            Action::Authenticate => 0x16,
            Action::DeploySecret => 0x17,
            Action::RunMigration => 0x18,
            Action::AdmitAgent => 0x1F,
            Action::InvokeAgent => 0x20,
        }
    }

    /// Inverse of [`Action::discriminant`]. `None` for unknown bytes ⇒ fail-closed.
    pub fn from_discriminant(b: u8) -> Option<Action> {
        Some(match b {
            0x01 => Action::Send,
            0x02 => Action::Read,
            0x03 => Action::Append,
            0x04 => Action::Render,
            0x05 => Action::CreateOrder,
            0x06 => Action::ReadProjection,
            0x13 => Action::SettlementRecorded,
            0x16 => Action::Authenticate,
            0x17 => Action::DeploySecret,
            0x18 => Action::RunMigration,
            0x1F => Action::AdmitAgent,
            0x20 => Action::InvokeAgent,
            _ => return None,
        })
    }

    /// Whether this action is a RED-LINE verb.
    pub fn is_red_line(&self) -> bool {
        matches!(
            self,
            Action::SettlementRecorded
                | Action::Authenticate
                | Action::DeploySecret
                | Action::RunMigration
        )
    }
}

/// A set of authorized `(resource, action)` pairs (UCAN "narrow-only" attenuation
/// works over set-subset, mirroring bebop2 `Scope`/`Effect`). No score, no rating.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Scope {
    /// Set of authorized `(resource, action)` pairs.
    pub grants: Vec<(Resource, Action)>,
}

impl Scope {
    /// Construct a scope from an explicit set of pairs.
    pub fn new(grants: Vec<(Resource, Action)>) -> Self {
        Scope { grants }
    }

    /// Single-pair convenience constructor.
    pub fn single(resource: Resource, action: Action) -> Self {
        Scope {
            grants: vec![(resource, action)],
        }
    }

    /// Fixed-layout canonical encoding: `len(u16 LE) || (resource_u8, action_u8)*`.
    /// Self-delimiting and fail-closed on a truncated tail. No serde.
    pub fn to_tlv_bytes(&self) -> Vec<u8> {
        let n = self.grants.len() as u16;
        let mut out = n.to_le_bytes().to_vec();
        for (r, a) in &self.grants {
            out.push(r.discriminant());
            out.push(a.discriminant());
        }
        out
    }

    /// Strict decode of [`Scope::to_tlv_bytes`]. Fail-closed: an unknown resource/action
    /// byte, a wrong length, or trailing bytes yield `None`.
    pub fn from_tlv_bytes(bytes: &[u8]) -> Option<Scope> {
        if bytes.len() < 2 {
            return None;
        }
        let n = u16::from_le_bytes([bytes[0], bytes[1]]) as usize;
        if bytes.len() != 2 + n * 2 {
            return None;
        }
        let mut grants = Vec::with_capacity(n);
        for i in 0..n {
            let r = Resource::from_discriminant(bytes[2 + i * 2])?;
            let a = Action::from_discriminant(bytes[2 + i * 2 + 1])?;
            grants.push((r, a));
        }
        Some(Scope { grants })
    }

    /// Whether every pair in `self` appears in `super_scope` (narrow-or-equal). An empty
    /// scope is a subset of anything (least privilege).
    pub fn is_subset_of(&self, super_scope: &Scope) -> bool {
        self.grants.iter().all(|p| super_scope.grants.contains(p))
    }

    /// Whether ANY grant in this scope is a RED-LINE (resource OR action).
    pub fn touches_red_line(&self) -> bool {
        self.grants
            .iter()
            .any(|(r, a)| r.is_red_line() || a.is_red_line())
    }
}

/// The armed red-line policy (mirrors bebop2 `RedLinePolicy`). Deny-by-default is the
/// production posture; `AllowList` narrows to operator-enumerated verbs-on-objects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedLinePolicy {
    /// Reject any capability whose scope touches a red line unless allow-listed below.
    DenyByDefault,
    /// Only the enumerated scopes may pass the red line.
    AllowList(Vec<Scope>),
}

impl RedLinePolicy {
    /// `Ok(())` iff `scope` is permitted under this policy. A non-red-line scope always
    /// passes; a red-line scope passes only if explicitly allow-listed.
    pub fn check(&self, scope: &Scope) -> Result<(), ()> {
        if !scope.touches_red_line() {
            return Ok(());
        }
        match self {
            RedLinePolicy::DenyByDefault => Err(()),
            RedLinePolicy::AllowList(allowed) => {
                // Every red-line grant must be covered by an allow-listed scope.
                if scope.is_subset_of_any(allowed) {
                    Ok(())
                } else {
                    Err(())
                }
            }
        }
    }
}

impl Scope {
    fn is_subset_of_any(&self, supers: &[Scope]) -> bool {
        // Union the allow-listed scopes and check subset against the union.
        let mut union = Scope::default();
        for s in supers {
            for g in &s.grants {
                if !union.grants.contains(g) {
                    union.grants.push(*g);
                }
            }
        }
        self.is_subset_of(&union)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discriminants_are_pinned() {
        // Wire/signing contract — changing any of these is a breaking change. The new
        // B1 variants are pinned here so a merge/renumber is caught mechanically
        // (B1 migration step 1 wire-stability pin).
        assert_eq!(Resource::AgentBridge.discriminant(), 0x12);
        assert_eq!(Action::AdmitAgent.discriminant(), 0x1F);
        assert_eq!(Action::InvokeAgent.discriminant(), 0x20);
        // A few inherited-from-bebop2 pins (alignment if ever unified).
        assert_eq!(Resource::Ledger.discriminant(), 0x02);
        assert_eq!(Resource::Migration.discriminant(), 0x11);
        assert_eq!(Action::SettlementRecorded.discriminant(), 0x13);
    }

    #[test]
    fn resource_action_roundtrip_fail_closed() {
        for r in [Resource::AgentBridge, Resource::Route, Resource::Auth] {
            assert_eq!(Resource::from_discriminant(r.discriminant()), Some(r));
        }
        // Unknown byte fails closed.
        assert_eq!(Resource::from_discriminant(0xEE), None);
        assert_eq!(Action::from_discriminant(0xEE), None);
    }

    #[test]
    fn scope_tlv_roundtrip_and_reject_garbage() {
        let s = Scope::new(vec![
            (Resource::AgentBridge, Action::AdmitAgent),
            (Resource::Menu, Action::Read),
        ]);
        let bytes = s.to_tlv_bytes();
        assert_eq!(Scope::from_tlv_bytes(&bytes), Some(s));
        // Truncated / unknown-byte tails fail closed.
        assert_eq!(Scope::from_tlv_bytes(&bytes[..bytes.len() - 1]), None);
        let mut bad = bytes.clone();
        *bad.last_mut().unwrap() = 0xEE; // unknown action byte
        assert_eq!(Scope::from_tlv_bytes(&bad), None);
    }

    #[test]
    fn red_line_classification() {
        assert!(Scope::single(Resource::Ledger, Action::SettlementRecorded).touches_red_line());
        assert!(Scope::single(Resource::Auth, Action::Authenticate).touches_red_line());
        assert!(!Scope::single(Resource::Menu, Action::Read).touches_red_line());
        // AgentBridge admission itself is NOT a red line (it is the gated surface).
        assert!(!Scope::single(Resource::AgentBridge, Action::AdmitAgent).touches_red_line());
    }

    #[test]
    fn deny_by_default_rejects_red_line_allows_clean() {
        let p = RedLinePolicy::DenyByDefault;
        assert!(p
            .check(&Scope::single(Resource::Ledger, Action::SettlementRecorded))
            .is_err());
        assert!(p
            .check(&Scope::single(Resource::Menu, Action::Read))
            .is_ok());
    }

    #[test]
    fn allow_list_narrows_precisely() {
        let p = RedLinePolicy::AllowList(vec![Scope::single(
            Resource::Ledger,
            Action::SettlementRecorded,
        )]);
        assert!(p
            .check(&Scope::single(Resource::Ledger, Action::SettlementRecorded))
            .is_ok());
        // A different red-line scope is still denied.
        assert!(p
            .check(&Scope::single(Resource::Auth, Action::Authenticate))
            .is_err());
    }
}
