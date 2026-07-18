//! ports/agent/manifest.rs — the signed `AgentManifest` (B1 §2.1): a strict,
//! canonical, re-derivable TLV whose closed enums make a free-form value
//! **structurally unrepresentable** (rejected at decode, before any gate runs).
//!
//! Compile firewall (mirrors `ports/llm.rs:3-7`): ZERO network / HTTP / JSON / serde.
//! TLV is plain byte encode/decode — no serde on the signed path (bebop2 discipline).
//!
//! ## Field list (canonical, fixed T order; unknown T ⇒ decode error, fail-closed)
//! ```text
//!   T=0x01 agent_node_id    : 32 B   NodeId = SHA3-256(pq_pub ‖ classical_pub)
//!   T=0x02 subject_key      : 32 B   Ed25519 public key (classical leg)
//!   T=0x03 subject_key_pq   : 1952 B ML-DSA-65 public key — MANDATORY
//!   T=0x04 agent_caps       : 1 B    fail-closed bitmap
//!   T=0x05 action_scopes    : var    Scope (closed (Resource,Action) pairs)
//!   T=0x06 resource_needs   : var    closed ResourceNeed list (egress = u16 index)
//!   T=0x07 cost_denomination: 1 B    closed enum; only TokenBucketUnits(0x01)
//!   T=0x08 budget_request   : 16 B   capacity u64 ‖ refill_milli_units_per_sec u64
//!   T=0x09 validation_policy: 1 B    closed enum; floor = RequireBoth(0x01)
//!   T=0x0A execution_model  : 1 B    WasmComponent(0x01) | NativeProcess(0x02)
//!   T=0x0B config_axes      : var    (axis_id u8, value_index u8) — bounded lattice
//!   T=0x0C depth_request    : 1 B
//!   T=0x0D quirks_profile   : 1 B    closed registry; McpServer(0x01)
//!   T=0x0E nonce            : 8 B
//!   T=0x0F expiry           : 8 B
//! ```
//!
//! ## Forward-compat tradeoff (blueprint §(c), FLAGGED, not resolved here)
//! The envelope carries a FIXED 16-byte domain+version magic; there is deliberately no
//! *negotiable* `manifest_version` field. Closed-enum unrepresentability (strong safety)
//! is bought at the cost of zero forward-compat headroom beyond the 2 spare `AgentCaps`
//! bits: the first schema evolution is a coordinated decoder upgrade. The blueprint's
//! recommended cheap hedge — a negotiable version TLV added before ship — is an OPERATOR
//! RULING, not decided in the numbered field list, so it is flagged here, not applied.

use crate::event_log::sha3_256;

use super::cap::{HybridPolicy, ML_DSA_65_PK_LEN};
use super::scope::Scope;

/// Fixed 16-byte envelope magic + version tag (`\x01`). Not a negotiable version field.
const DOMAIN_MANIFEST: &[u8; 16] = b"dowiz.agentmfst\x01";

// Canonical field tags (pinned; ascending; part of the contract).
const T_NODE_ID: u8 = 0x01;
const T_SUBJECT_KEY: u8 = 0x02;
const T_SUBJECT_KEY_PQ: u8 = 0x03;
const T_AGENT_CAPS: u8 = 0x04;
const T_ACTION_SCOPES: u8 = 0x05;
const T_RESOURCE_NEEDS: u8 = 0x06;
const T_COST_DENOM: u8 = 0x07;
const T_BUDGET_REQUEST: u8 = 0x08;
const T_VALIDATION_POLICY: u8 = 0x09;
const T_EXECUTION_MODEL: u8 = 0x0A;
const T_CONFIG_AXES: u8 = 0x0B;
const T_DEPTH_REQUEST: u8 = 0x0C;
const T_QUIRKS_PROFILE: u8 = 0x0D;
const T_NONCE: u8 = 0x0E;
const T_EXPIRY: u8 = 0x0F;

/// Fail-closed capability bitmap (the `Caps`-shape generalized). An undeclared bit is
/// `false`; the caller must never assume presence. Bits 6/7 are reserved (2 spare
/// forward-compat capabilities) and MUST be clear — a set reserved bit fails decode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AgentCaps {
    /// Executes tool calls.
    pub invoke_tool: bool,
    /// Serves resource reads.
    pub read_resource: bool,
    /// Serves prompt templates.
    pub render_prompt: bool,
    /// May request sub-agent invocation. `false` ⇒ granted depth is 0.
    pub delegate: bool,
    /// Async task lifecycle.
    pub long_task: bool,
    /// Partial results.
    pub streaming: bool,
}

impl AgentCaps {
    const B_INVOKE_TOOL: u8 = 1 << 0;
    const B_READ_RESOURCE: u8 = 1 << 1;
    const B_RENDER_PROMPT: u8 = 1 << 2;
    const B_DELEGATE: u8 = 1 << 3;
    const B_LONG_TASK: u8 = 1 << 4;
    const B_STREAMING: u8 = 1 << 5;
    const RESERVED_MASK: u8 = 0b1100_0000; // bits 6,7 — must be clear.

    /// Encode to the fail-closed bitmap byte.
    pub fn to_bits(&self) -> u8 {
        let mut b = 0u8;
        if self.invoke_tool {
            b |= Self::B_INVOKE_TOOL;
        }
        if self.read_resource {
            b |= Self::B_READ_RESOURCE;
        }
        if self.render_prompt {
            b |= Self::B_RENDER_PROMPT;
        }
        if self.delegate {
            b |= Self::B_DELEGATE;
        }
        if self.long_task {
            b |= Self::B_LONG_TASK;
        }
        if self.streaming {
            b |= Self::B_STREAMING;
        }
        b
    }

    /// Decode from the bitmap byte. Fail-closed: a set reserved bit (an undeclared
    /// future capability) yields `None` — an old decoder must not silently admit a
    /// capability it does not understand.
    pub fn from_bits(b: u8) -> Option<AgentCaps> {
        if b & Self::RESERVED_MASK != 0 {
            return None;
        }
        Some(AgentCaps {
            invoke_tool: b & Self::B_INVOKE_TOOL != 0,
            read_resource: b & Self::B_READ_RESOURCE != 0,
            render_prompt: b & Self::B_RENDER_PROMPT != 0,
            delegate: b & Self::B_DELEGATE != 0,
            long_task: b & Self::B_LONG_TASK != 0,
            streaming: b & Self::B_STREAMING != 0,
        })
    }
}

/// A closed resource need. Egress hosts are u16 INDEXES into the operator's allowlist,
/// never hostname strings (a hostname string is structurally unrepresentable).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceNeed {
    /// Egress to operator-allowlist host index `n`.
    EgressHost(u16),
    /// Read access to operator-allowlist mount index `n`.
    FilesystemRead(u16),
    /// Wall-clock read.
    WallClock,
}

impl ResourceNeed {
    const K_EGRESS: u8 = 0x01;
    const K_FS_READ: u8 = 0x02;
    const K_WALLCLOCK: u8 = 0x03;
}

/// Cost denomination — closed; only value today is `TokenBucketUnits`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CostDenomination {
    /// 1 unit ≈ 1 token-equivalent (`Usage::cost` parity).
    TokenBucketUnits,
}

/// Validation policy — closed, with an UNRELAXABLE floor. There is deliberately no
/// variant weaker than `RequireBoth`: no `ClassicalOnly`/`ClassicalUntilPqAudit` code
/// point exists, so relaxation below the hybrid floor is impossible at the wire layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationPolicy {
    /// Require BOTH the classical and post-quantum legs (the floor).
    RequireBoth,
}

impl ValidationPolicy {
    const B_REQUIRE_BOTH: u8 = 0x01;

    /// The `effective_policy(declared)` map (§2.1): can only return `RequireBoth` (or a
    /// future *narrowing* variant). Relaxation below the floor is unrepresentable.
    pub fn effective(self) -> HybridPolicy {
        match self {
            ValidationPolicy::RequireBoth => HybridPolicy::RequireBoth,
        }
    }
}

/// Execution model — maps 1:1 onto `register_adapter` / `SandboxTier`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionModel {
    /// WASM component — capability-scoped, no KVM dependency; always admits.
    WasmComponent,
    /// Native process — hardware-isolated microVM; admits only under `kvm_available()`.
    NativeProcess,
}

impl ExecutionModel {
    const B_WASM: u8 = 0x01;
    const B_NATIVE: u8 = 0x02;
    /// The exact string `register_adapter` expects (`isolation::microvm`).
    pub fn as_adapter_str(self) -> &'static str {
        match self {
            ExecutionModel::WasmComponent => "wasm-component",
            ExecutionModel::NativeProcess => "native-process",
        }
    }
}

/// Quirks profile — a closed registry (never a string).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuirksProfile {
    /// The MCP-server bridge profile (B1 §2.3).
    McpServer,
}

impl QuirksProfile {
    const B_MCP: u8 = 0x01;
}

/// Integer budget request (no floats). `refill` is in milli-units/sec so a sub-unit
/// rate is expressible without a float on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BudgetRequest {
    /// Bucket capacity (burst) in token-bucket units.
    pub capacity: u64,
    /// Refill in milli-units per second.
    pub refill_milli_units_per_sec: u64,
}

// ── Enumerable config lattice (E3 Phase-A) ────────────────────────────────────────
// Fixed in-code axes with bounded domains. An out-of-range value index or an unknown
// axis id fails TLV decode — rejection is structural, at parse time, before any gate.
// (cache_group_id is deliberately NOT here: SH-2 co-scope membership is read from
// operator-signed config at admission, NEVER declared by the manifest.)

/// Domain size for a config axis, or `None` if the axis id is unknown.
pub fn config_axis_domain(axis_id: u8) -> Option<u8> {
    Some(match axis_id {
        0x01 => 2, // transport: { stdio=0, streamable_http=1 }
        0x02 => 2, // protocol_epoch: { pinned set of 2 }
        0x03 => 2, // cache_policy: { Exact=0, NoCache=1 }
        0x04 => 3, // max_concurrent: { 1, 2, 4 }
        _ => return None,
    })
}

/// Why manifest decode failed (fail-closed; NOTHING downstream runs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestParseError {
    /// Envelope magic/version mismatch.
    BadMagic,
    /// Tag out of the pinned fixed order, or an unknown tag.
    UnexpectedTag { expected: u8, got: u8 },
    /// Buffer ended mid-field.
    Truncated,
    /// A fixed-width field had the wrong length.
    BadLength(&'static str),
    /// A config axis id is not in the fixed registry.
    UnknownConfigAxis(u8),
    /// A config value index is out of its axis's bounded domain.
    ConfigValueOutOfRange { axis: u8, value: u8 },
    /// A resource-need kind byte is unknown.
    UnknownResourceNeed(u8),
    /// A reserved `AgentCaps` bit was set (undeclared future capability).
    ReservedCapBit,
    /// A validation-policy byte below the `RequireBoth` floor (no such code point).
    WeakPolicy(u8),
    /// Unknown cost-denomination byte.
    BadCostDenom(u8),
    /// Unknown execution-model byte.
    BadExecModel(u8),
    /// Unknown quirks-profile byte.
    BadQuirks(u8),
    /// A `Scope` TLV was malformed (unknown resource/action, bad length).
    BadScope,
    /// Bytes remain after the last field.
    TrailingBytes,
}

impl std::fmt::Display for ManifestParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "manifest parse error: {self:?}")
    }
}

/// The signed agent discovery artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentManifest {
    /// `NodeId = SHA3-256(pq_pub ‖ classical_pub)` (T=0x01).
    pub agent_node_id: [u8; 32],
    /// Ed25519 public key (T=0x02).
    pub subject_key: [u8; 32],
    /// ML-DSA-65 public key, 1952 B, MANDATORY (T=0x03).
    pub subject_key_pq: Vec<u8>,
    /// Fail-closed capability bitmap (T=0x04).
    pub agent_caps: AgentCaps,
    /// Closed `(Resource, Action)` scopes (T=0x05).
    pub action_scopes: Scope,
    /// Closed resource-need list (T=0x06).
    pub resource_needs: Vec<ResourceNeed>,
    /// Cost denomination (T=0x07).
    pub cost_denomination: CostDenomination,
    /// Integer budget request (T=0x08).
    pub budget_request: BudgetRequest,
    /// Validation policy (T=0x09), floor `RequireBoth`.
    pub validation_policy: ValidationPolicy,
    /// Execution model (T=0x0A).
    pub execution_model: ExecutionModel,
    /// Bounded config-lattice axes (T=0x0B).
    pub config_axes: Vec<(u8, u8)>,
    /// Requested sub-delegation depth (T=0x0C).
    pub depth_request: u8,
    /// Quirks profile (T=0x0D).
    pub quirks_profile: QuirksProfile,
    /// Nonce (T=0x0E).
    pub nonce: [u8; 8],
    /// Expiry monotonic tick (T=0x0F).
    pub expiry: u64,
}

impl AgentManifest {
    /// Canonical TLV bytes (deterministic, re-derivable). This is exactly what a
    /// `SignedFrame` payload carries and what `content_id` hashes.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(DOMAIN_MANIFEST);
        put_field(&mut out, T_NODE_ID, &self.agent_node_id);
        put_field(&mut out, T_SUBJECT_KEY, &self.subject_key);
        put_field(&mut out, T_SUBJECT_KEY_PQ, &self.subject_key_pq);
        put_field(&mut out, T_AGENT_CAPS, &[self.agent_caps.to_bits()]);
        put_field(
            &mut out,
            T_ACTION_SCOPES,
            &self.action_scopes.to_tlv_bytes(),
        );
        put_field(
            &mut out,
            T_RESOURCE_NEEDS,
            &encode_resource_needs(&self.resource_needs),
        );
        put_field(
            &mut out,
            T_COST_DENOM,
            &[match self.cost_denomination {
                CostDenomination::TokenBucketUnits => 0x01,
            }],
        );
        let mut budget = Vec::with_capacity(16);
        budget.extend_from_slice(&self.budget_request.capacity.to_le_bytes());
        budget.extend_from_slice(&self.budget_request.refill_milli_units_per_sec.to_le_bytes());
        put_field(&mut out, T_BUDGET_REQUEST, &budget);
        put_field(
            &mut out,
            T_VALIDATION_POLICY,
            &[match self.validation_policy {
                ValidationPolicy::RequireBoth => ValidationPolicy::B_REQUIRE_BOTH,
            }],
        );
        put_field(
            &mut out,
            T_EXECUTION_MODEL,
            &[match self.execution_model {
                ExecutionModel::WasmComponent => ExecutionModel::B_WASM,
                ExecutionModel::NativeProcess => ExecutionModel::B_NATIVE,
            }],
        );
        put_field(
            &mut out,
            T_CONFIG_AXES,
            &encode_config_axes(&self.config_axes),
        );
        put_field(&mut out, T_DEPTH_REQUEST, &[self.depth_request]);
        put_field(
            &mut out,
            T_QUIRKS_PROFILE,
            &[match self.quirks_profile {
                QuirksProfile::McpServer => QuirksProfile::B_MCP,
            }],
        );
        put_field(&mut out, T_NONCE, &self.nonce);
        put_field(&mut out, T_EXPIRY, &self.expiry.to_le_bytes());
        out
    }

    /// Content-address: `SHA3-256(canonical_bytes)` — the natural key for the WORM
    /// admission event + the semantic-idempotency map.
    pub fn content_id(&self) -> [u8; 32] {
        sha3_256(&self.canonical_bytes())
    }

    /// Strict TLV decode (fail-closed). Unknown T, out-of-domain config axis, missing
    /// mandatory field, a policy byte below the floor, or trailing bytes ⇒ error, and
    /// NOTHING downstream runs.
    pub fn decode(bytes: &[u8]) -> Result<AgentManifest, ManifestParseError> {
        if bytes.len() < 16 || &bytes[..16] != DOMAIN_MANIFEST {
            return Err(ManifestParseError::BadMagic);
        }
        let mut c = Cursor::new(&bytes[16..]);

        let agent_node_id: [u8; 32] = c.field_fixed(T_NODE_ID, 32)?.try_into().unwrap();
        let subject_key: [u8; 32] = c.field_fixed(T_SUBJECT_KEY, 32)?.try_into().unwrap();
        let subject_key_pq = c.field_fixed(T_SUBJECT_KEY_PQ, ML_DSA_65_PK_LEN)?.to_vec();

        let caps_byte = c.field_fixed(T_AGENT_CAPS, 1)?[0];
        let agent_caps =
            AgentCaps::from_bits(caps_byte).ok_or(ManifestParseError::ReservedCapBit)?;

        let scope_bytes = c.field_var(T_ACTION_SCOPES)?;
        let action_scopes =
            Scope::from_tlv_bytes(scope_bytes).ok_or(ManifestParseError::BadScope)?;

        let needs_bytes = c.field_var(T_RESOURCE_NEEDS)?;
        let resource_needs = decode_resource_needs(needs_bytes)?;

        let cost_byte = c.field_fixed(T_COST_DENOM, 1)?[0];
        let cost_denomination = match cost_byte {
            0x01 => CostDenomination::TokenBucketUnits,
            other => return Err(ManifestParseError::BadCostDenom(other)),
        };

        let budget_bytes = c.field_fixed(T_BUDGET_REQUEST, 16)?;
        let budget_request = BudgetRequest {
            capacity: u64::from_le_bytes(budget_bytes[0..8].try_into().unwrap()),
            refill_milli_units_per_sec: u64::from_le_bytes(budget_bytes[8..16].try_into().unwrap()),
        };

        let policy_byte = c.field_fixed(T_VALIDATION_POLICY, 1)?[0];
        let validation_policy = match policy_byte {
            ValidationPolicy::B_REQUIRE_BOTH => ValidationPolicy::RequireBoth,
            other => return Err(ManifestParseError::WeakPolicy(other)),
        };

        let exec_byte = c.field_fixed(T_EXECUTION_MODEL, 1)?[0];
        let execution_model = match exec_byte {
            ExecutionModel::B_WASM => ExecutionModel::WasmComponent,
            ExecutionModel::B_NATIVE => ExecutionModel::NativeProcess,
            other => return Err(ManifestParseError::BadExecModel(other)),
        };

        let axes_bytes = c.field_var(T_CONFIG_AXES)?;
        let config_axes = decode_config_axes(axes_bytes)?;

        let depth_request = c.field_fixed(T_DEPTH_REQUEST, 1)?[0];

        let quirks_byte = c.field_fixed(T_QUIRKS_PROFILE, 1)?[0];
        let quirks_profile = match quirks_byte {
            QuirksProfile::B_MCP => QuirksProfile::McpServer,
            other => return Err(ManifestParseError::BadQuirks(other)),
        };

        let nonce: [u8; 8] = c.field_fixed(T_NONCE, 8)?.try_into().unwrap();
        let expiry = u64::from_le_bytes(c.field_fixed(T_EXPIRY, 8)?.try_into().unwrap());

        if !c.at_end() {
            return Err(ManifestParseError::TrailingBytes);
        }

        Ok(AgentManifest {
            agent_node_id,
            subject_key,
            subject_key_pq,
            agent_caps,
            action_scopes,
            resource_needs,
            cost_denomination,
            budget_request,
            validation_policy,
            execution_model,
            config_axes,
            depth_request,
            quirks_profile,
            nonce,
            expiry,
        })
    }
}

// ── TLV primitives ────────────────────────────────────────────────────────────────

fn put_field(out: &mut Vec<u8>, tag: u8, value: &[u8]) {
    out.push(tag);
    out.extend_from_slice(&(value.len() as u32).to_le_bytes());
    out.extend_from_slice(value);
}

fn encode_resource_needs(needs: &[ResourceNeed]) -> Vec<u8> {
    let mut out = (needs.len() as u16).to_le_bytes().to_vec();
    for n in needs {
        match n {
            ResourceNeed::EgressHost(i) => {
                out.push(ResourceNeed::K_EGRESS);
                out.extend_from_slice(&i.to_le_bytes());
            }
            ResourceNeed::FilesystemRead(i) => {
                out.push(ResourceNeed::K_FS_READ);
                out.extend_from_slice(&i.to_le_bytes());
            }
            ResourceNeed::WallClock => out.push(ResourceNeed::K_WALLCLOCK),
        }
    }
    out
}

fn decode_resource_needs(bytes: &[u8]) -> Result<Vec<ResourceNeed>, ManifestParseError> {
    if bytes.len() < 2 {
        return Err(ManifestParseError::Truncated);
    }
    let n = u16::from_le_bytes([bytes[0], bytes[1]]) as usize;
    let mut i = 2;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        if i >= bytes.len() {
            return Err(ManifestParseError::Truncated);
        }
        let kind = bytes[i];
        i += 1;
        match kind {
            ResourceNeed::K_EGRESS | ResourceNeed::K_FS_READ => {
                if i + 2 > bytes.len() {
                    return Err(ManifestParseError::Truncated);
                }
                let idx = u16::from_le_bytes([bytes[i], bytes[i + 1]]);
                i += 2;
                out.push(if kind == ResourceNeed::K_EGRESS {
                    ResourceNeed::EgressHost(idx)
                } else {
                    ResourceNeed::FilesystemRead(idx)
                });
            }
            ResourceNeed::K_WALLCLOCK => out.push(ResourceNeed::WallClock),
            other => return Err(ManifestParseError::UnknownResourceNeed(other)),
        }
    }
    if i != bytes.len() {
        return Err(ManifestParseError::TrailingBytes);
    }
    Ok(out)
}

fn encode_config_axes(axes: &[(u8, u8)]) -> Vec<u8> {
    let mut out = (axes.len() as u16).to_le_bytes().to_vec();
    for (a, v) in axes {
        out.push(*a);
        out.push(*v);
    }
    out
}

fn decode_config_axes(bytes: &[u8]) -> Result<Vec<(u8, u8)>, ManifestParseError> {
    if bytes.len() < 2 {
        return Err(ManifestParseError::Truncated);
    }
    let n = u16::from_le_bytes([bytes[0], bytes[1]]) as usize;
    if bytes.len() != 2 + n * 2 {
        return Err(ManifestParseError::Truncated);
    }
    let mut out = Vec::with_capacity(n);
    for k in 0..n {
        let axis = bytes[2 + k * 2];
        let value = bytes[2 + k * 2 + 1];
        let domain = config_axis_domain(axis).ok_or(ManifestParseError::UnknownConfigAxis(axis))?;
        if value >= domain {
            return Err(ManifestParseError::ConfigValueOutOfRange { axis, value });
        }
        out.push((axis, value));
    }
    Ok(out)
}

/// A minimal fail-closed byte cursor for the fixed-order TLV decode.
struct Cursor<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Cursor<'a> {
    fn new(b: &'a [u8]) -> Self {
        Cursor { b, i: 0 }
    }
    fn at_end(&self) -> bool {
        self.i == self.b.len()
    }
    /// Read a field header, requiring the exact expected tag (fixed order).
    fn header(&mut self, expected: u8) -> Result<usize, ManifestParseError> {
        if self.i + 5 > self.b.len() {
            return Err(ManifestParseError::Truncated);
        }
        let got = self.b[self.i];
        if got != expected {
            return Err(ManifestParseError::UnexpectedTag { expected, got });
        }
        let len = u32::from_le_bytes(self.b[self.i + 1..self.i + 5].try_into().unwrap()) as usize;
        self.i += 5;
        if self.i + len > self.b.len() {
            return Err(ManifestParseError::Truncated);
        }
        Ok(len)
    }
    /// Read a fixed-width field, enforcing the declared length.
    fn field_fixed(&mut self, tag: u8, want: usize) -> Result<&'a [u8], ManifestParseError> {
        let len = self.header(tag)?;
        if len != want {
            return Err(ManifestParseError::BadLength(match tag {
                T_NODE_ID => "agent_node_id",
                T_SUBJECT_KEY => "subject_key",
                T_SUBJECT_KEY_PQ => "subject_key_pq",
                T_BUDGET_REQUEST => "budget_request",
                T_NONCE => "nonce",
                T_EXPIRY => "expiry",
                _ => "fixed-field",
            }));
        }
        let v = &self.b[self.i..self.i + len];
        self.i += len;
        Ok(v)
    }
    /// Read a variable-width field.
    fn field_var(&mut self, tag: u8) -> Result<&'a [u8], ManifestParseError> {
        let len = self.header(tag)?;
        let v = &self.b[self.i..self.i + len];
        self.i += len;
        Ok(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::agent::scope::{Action, Resource};

    fn sample() -> AgentManifest {
        AgentManifest {
            agent_node_id: [9u8; 32],
            subject_key: [1u8; 32],
            subject_key_pq: vec![7u8; ML_DSA_65_PK_LEN],
            agent_caps: AgentCaps {
                invoke_tool: true,
                read_resource: true,
                ..Default::default()
            },
            action_scopes: Scope::new(vec![
                (Resource::AgentBridge, Action::AdmitAgent),
                (Resource::Menu, Action::Read),
            ]),
            resource_needs: vec![ResourceNeed::EgressHost(3), ResourceNeed::WallClock],
            cost_denomination: CostDenomination::TokenBucketUnits,
            budget_request: BudgetRequest {
                capacity: 4096,
                refill_milli_units_per_sec: 8000,
            },
            validation_policy: ValidationPolicy::RequireBoth,
            execution_model: ExecutionModel::WasmComponent,
            config_axes: vec![(0x01, 0), (0x04, 2)],
            depth_request: 2,
            quirks_profile: QuirksProfile::McpServer,
            nonce: [5u8; 8],
            expiry: 9999,
        }
    }

    #[test]
    fn canonical_roundtrip_is_deterministic() {
        let m = sample();
        let a = m.canonical_bytes();
        let b = m.canonical_bytes();
        assert_eq!(a, b, "canonical encoding is deterministic");
        assert_eq!(AgentManifest::decode(&a).unwrap(), m);
        assert_eq!(
            AgentManifest::decode(&a).unwrap().content_id(),
            m.content_id()
        );
    }

    #[test]
    fn free_form_config_axis_dies_at_decode() {
        // Unknown axis id.
        let mut m = sample();
        m.config_axes = vec![(0x7F, 0)];
        let bytes = m.canonical_bytes();
        assert_eq!(
            AgentManifest::decode(&bytes),
            Err(ManifestParseError::UnknownConfigAxis(0x7F))
        );
        // Out-of-domain value index (axis 0x01 has domain size 2 ⇒ index 5 illegal).
        let mut m2 = sample();
        m2.config_axes = vec![(0x01, 5)];
        assert_eq!(
            AgentManifest::decode(&m2.canonical_bytes()),
            Err(ManifestParseError::ConfigValueOutOfRange {
                axis: 0x01,
                value: 5
            })
        );
    }

    #[test]
    fn floor_is_unrelaxable_over_all_256_policy_bytes() {
        // §4 criterion 3: no decodable byte yields a policy weaker than RequireBoth.
        // Build a valid manifest, then overwrite ONLY the validation-policy value byte
        // with every possible byte; each either decodes to RequireBoth or errors.
        let m = sample();
        let good = m.canonical_bytes();
        // Locate the validation-policy field value (fixed order; we re-encode with a
        // known layout, so find the T_VALIDATION_POLICY tag and its 1-byte value).
        let pos = find_field_value_pos(&good, T_VALIDATION_POLICY);
        for byte in 0u16..=255 {
            let byte = byte as u8;
            let mut variant = good.clone();
            variant[pos] = byte;
            match AgentManifest::decode(&variant) {
                Ok(parsed) => assert_eq!(
                    parsed.validation_policy.effective(),
                    HybridPolicy::RequireBoth,
                    "any decodable policy byte must be RequireBoth-or-stronger (byte={byte:#x})"
                ),
                Err(ManifestParseError::WeakPolicy(b)) => assert_eq!(b, byte),
                Err(_) => {} // other structural errors are fine — still not a weak pass.
            }
        }
    }

    /// Find the byte offset of a single-byte field's VALUE (header is tag + u32 len).
    fn find_field_value_pos(bytes: &[u8], tag: u8) -> usize {
        let mut i = 16; // skip envelope magic
        loop {
            let t = bytes[i];
            let len = u32::from_le_bytes(bytes[i + 1..i + 5].try_into().unwrap()) as usize;
            if t == tag {
                return i + 5;
            }
            i += 5 + len;
        }
    }

    #[test]
    fn reserved_cap_bit_fails_closed() {
        // A set reserved bit is not decodable (fail-closed).
        assert_eq!(AgentCaps::from_bits(0b0100_0000), None);
        assert_eq!(AgentCaps::from_bits(0b1000_0000), None);
        assert!(AgentCaps::from_bits(0b0011_1111).is_some());
    }

    #[test]
    fn unknown_tag_and_truncation_fail_closed() {
        let m = sample();
        let mut bytes = m.canonical_bytes();
        // Corrupt the first field tag ⇒ UnexpectedTag.
        bytes[16] = 0x7E;
        assert!(matches!(
            AgentManifest::decode(&bytes),
            Err(ManifestParseError::UnexpectedTag { .. })
        ));
        // Truncate ⇒ Truncated / BadMagic.
        assert!(AgentManifest::decode(&m.canonical_bytes()[..20]).is_err());
        // Trailing bytes ⇒ TrailingBytes.
        let mut trailing = m.canonical_bytes();
        trailing.push(0xFF);
        assert_eq!(
            AgentManifest::decode(&trailing),
            Err(ManifestParseError::TrailingBytes)
        );
    }

    #[test]
    fn wrong_pq_width_fails_closed() {
        let mut m = sample();
        m.subject_key_pq = vec![7u8; 100]; // not 1952
        assert!(matches!(
            AgentManifest::decode(&m.canonical_bytes()),
            Err(ManifestParseError::BadLength("subject_key_pq"))
        ));
    }
}
