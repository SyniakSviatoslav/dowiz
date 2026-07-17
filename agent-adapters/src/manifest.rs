//! manifest.rs — draft-manifest construction (adapter side).
//!
//! The kernel port owns the canonical TLV codec + value types; this module maps UNTRUSTED
//! MCP discovery output → a DRAFT `AgentManifest` (closed-enum values only). The operator
//! then encodes it (`AgentManifest::canonical_bytes`) and signs the frame — MCP's open-
//! world grammar never enters the signed artifact. Only the closed-enum scopes drawn from
//! the operator's tool allowlist reach the manifest.

use dowiz_kernel::ports::agent::{
    AgentCaps, AgentManifest, BudgetRequest, CostDenomination, ExecutionModel, NodeId,
    QuirksProfile, ResourceNeed, ValidationPolicy,
};

use crate::quirks::AgentQuirks;

/// Build a draft manifest from the agent's public keys + discovery-derived caps + the
/// operator's quirks profile. `validation_policy` is fixed at the `RequireBoth` floor;
/// `agent_node_id` is derived (never claimed). `extra_axes` lets the operator add bounded
/// config beyond the transport axis (each still validated at decode).
#[allow(clippy::too_many_arguments)]
pub fn draft_manifest(
    subject_key: [u8; 32],
    subject_key_pq: Vec<u8>,
    quirks: &AgentQuirks,
    caps: AgentCaps,
    resource_needs: Vec<ResourceNeed>,
    budget_request: BudgetRequest,
    execution_model: ExecutionModel,
    extra_axes: Vec<(u8, u8)>,
    depth_request: u8,
    nonce: [u8; 8],
    expiry: u64,
) -> AgentManifest {
    let agent_node_id = NodeId::from_keys(&subject_key_pq, &subject_key).0;
    let mut config_axes = vec![quirks.transport_kind.config_axis()];
    config_axes.extend(extra_axes);
    AgentManifest {
        agent_node_id,
        subject_key,
        subject_key_pq,
        agent_caps: caps,
        action_scopes: quirks.draft_action_scopes(),
        resource_needs,
        cost_denomination: CostDenomination::TokenBucketUnits,
        budget_request,
        validation_policy: ValidationPolicy::RequireBoth,
        execution_model,
        config_axes,
        depth_request,
        quirks_profile: QuirksProfile::McpServer,
        nonce,
        expiry,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dowiz_kernel::ports::agent::{Action, RefSigner, Resource, SignatureVerifier, ML_DSA_65_PK_LEN};
    use std::collections::BTreeMap;

    #[test]
    fn draft_has_derived_node_id_and_closed_scopes() {
        let v = RefSigner;
        let cls = v.classical_public(&[1u8; 32]);
        let pq = v.pq_public(&[2u8; 32]);
        let mut allow = BTreeMap::new();
        allow.insert("get_menu".to_string(), (Resource::Menu, Action::Read));
        let quirks = AgentQuirks::mcp_server(allow);
        let caps = quirks.caps_from_discovery(true, true, false, false);
        let m = draft_manifest(
            cls,
            pq.clone(),
            &quirks,
            caps,
            vec![],
            BudgetRequest { capacity: 100, refill_milli_units_per_sec: 1000 },
            ExecutionModel::WasmComponent,
            vec![],
            0,
            [3u8; 8],
            9999,
        );
        // node_id is DERIVED from the two keys, never claimed.
        assert_eq!(m.agent_node_id, NodeId::from_keys(&pq, &cls).0);
        assert_eq!(m.subject_key_pq.len(), ML_DSA_65_PK_LEN);
        // Only the closed-enum allow-listed scope reached the manifest.
        assert_eq!(m.action_scopes.grants, vec![(Resource::Menu, Action::Read)]);
        // The transport axis is present and its round-trip decodes (bounded lattice).
        assert!(m.config_axes.contains(&(0x01, 0)));
        assert_eq!(AgentManifest::decode(&m.canonical_bytes()).unwrap(), m);
    }
}
