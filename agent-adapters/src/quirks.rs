//! quirks.rs — `AgentQuirks` + the ONE reference profile `AgentQuirks::mcp_server()`.
//!
//! MCP's open-world string grammar NEVER enters the signed manifest. The bridge treats
//! discovery output as UNTRUSTED input and produces a draft; the operator's keys sign the
//! final closed-enum manifest. Tool names are free-form strings in MCP, so this profile
//! carries an operator-authored **tool allowlist** `tool_name → (Resource, Action)`;
//! unmapped tools are unreachable (fail-closed drop, never string passthrough). The
//! canonical sorted tool-map is hashed (`sha3_256`) into the drift anchor so post-
//! admission drift (`listChanged`, server substitution — registry-poisoning) is detectable.

use std::collections::BTreeMap;

use dowiz_kernel::event_log::sha3_256;
use dowiz_kernel::ports::agent::{Action, AgentCaps, Resource, Scope};

/// Transport kind — an enumerated config axis (never a free string).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportKind {
    /// Local stdio pipe (config axis 0x01 value 0).
    Stdio,
    /// Streamable HTTP (config axis 0x01 value 1).
    StreamableHttp,
}

impl TransportKind {
    /// The `(axis_id, value_index)` this transport encodes as in the manifest lattice.
    pub fn config_axis(&self) -> (u8, u8) {
        match self {
            TransportKind::Stdio => (0x01, 0),
            TransportKind::StreamableHttp => (0x01, 1),
        }
    }
}

/// Per-bridge behavior deltas (the `Quirks`-pattern generalized for agents). The bridge
/// reads these; it holds no framework knowledge itself beyond this struct.
#[derive(Debug, Clone)]
pub struct AgentQuirks {
    /// Stable profile id (e.g. `"mcp"`). Feeds the `TrackRecord.model` prefix.
    pub profile_id: &'static str,
    /// Enumerated transport axis.
    pub transport_kind: TransportKind,
    /// Pinned protocol epoch (e.g. an MCP spec date). Never a negotiated free string.
    pub protocol_epoch: &'static str,
    /// Operator-authored tool allowlist: `tool_name → (Resource, Action)`. The ONLY tool
    /// names that are reachable; everything else is a fail-closed drop.
    pub tool_allowlist: BTreeMap<String, (Resource, Action)>,
    /// Refuse server-initiated `sampling/createMessage` (RC-2 control inversion). Always
    /// `true` for MCP — an admitted agent must never drive the host's LLM.
    pub refuse_sampling: bool,
    /// Refuse server-initiated elicitation. Always `true` for MCP.
    pub refuse_elicitation: bool,
}

impl AgentQuirks {
    /// The reference MCP-server profile. `tool_allowlist` is operator-authored.
    pub fn mcp_server(tool_allowlist: BTreeMap<String, (Resource, Action)>) -> Self {
        AgentQuirks {
            profile_id: "mcp",
            transport_kind: TransportKind::Stdio,
            protocol_epoch: "mcp-2025-06-18",
            tool_allowlist,
            refuse_sampling: true,
            refuse_elicitation: true,
        }
    }

    /// Map MCP discovery flags → `AgentCaps` (§2.3): `tools ⇒ invoke_tool`, `resources ⇒
    /// read_resource`, `prompts ⇒ render_prompt`, MCP tasks ⇒ `long_task`. MCP has NO
    /// delegation, so `delegate` is ALWAYS `false` for this profile.
    pub fn caps_from_discovery(
        &self,
        has_tools: bool,
        has_resources: bool,
        has_prompts: bool,
        has_tasks: bool,
    ) -> AgentCaps {
        AgentCaps {
            invoke_tool: has_tools,
            read_resource: has_resources,
            render_prompt: has_prompts,
            delegate: false, // MCP has no delegation — always false for this profile.
            long_task: has_tasks,
            streaming: false,
        }
    }

    /// Map a free-form MCP tool name → a closed scope. `None` ⇒ fail-closed drop (the
    /// unmapped tool is unreachable).
    pub fn map_tool(&self, name: &str) -> Option<(Resource, Action)> {
        self.tool_allowlist.get(name).copied()
    }

    /// The draft `action_scopes` for the manifest: the sorted, de-duplicated set of
    /// mapped scopes for the allow-listed tools.
    pub fn draft_action_scopes(&self) -> Scope {
        let mut grants: Vec<(Resource, Action)> = Vec::new();
        for (_name, (r, a)) in &self.tool_allowlist {
            if !grants.contains(&(*r, *a)) {
                grants.push((*r, *a));
            }
        }
        Scope::new(grants)
    }

    /// The drift anchor: `sha3_256` over the canonical sorted tool set the server
    /// advertises AND that is allow-listed. Only names in BOTH the `advertised` set and
    /// the operator allowlist contribute (server substitution / `listChanged` shows up as
    /// a digest mismatch). `advertised` is the live `tools/list` result.
    pub fn tool_map_digest(&self, advertised: &[String]) -> [u8; 32] {
        // Canonical: sorted (name, resource_disc, action_disc) triples, length-prefixed.
        let mut pairs: Vec<(&String, (Resource, Action))> = Vec::new();
        for name in advertised {
            if let Some(scope) = self.map_tool(name) {
                pairs.push((name, scope));
            }
        }
        pairs.sort_by(|a, b| a.0.cmp(b.0));
        let mut buf = Vec::new();
        buf.extend_from_slice(&(pairs.len() as u32).to_le_bytes());
        for (name, (r, a)) in pairs {
            buf.extend_from_slice(&(name.len() as u32).to_le_bytes());
            buf.extend_from_slice(name.as_bytes());
            buf.push(r.discriminant());
            buf.push(a.discriminant());
        }
        sha3_256(&buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allowlist() -> BTreeMap<String, (Resource, Action)> {
        let mut m = BTreeMap::new();
        m.insert("get_menu".to_string(), (Resource::Menu, Action::Read));
        m.insert("read_order".to_string(), (Resource::Order, Action::Read));
        m
    }

    #[test]
    fn mcp_delegate_is_always_false() {
        let q = AgentQuirks::mcp_server(allowlist());
        let caps = q.caps_from_discovery(true, true, true, true);
        assert!(caps.invoke_tool && caps.read_resource && caps.render_prompt && caps.long_task);
        assert!(
            !caps.delegate,
            "MCP has no delegation ⇒ delegate is always false"
        );
    }

    #[test]
    fn unmapped_tool_is_fail_closed_drop() {
        let q = AgentQuirks::mcp_server(allowlist());
        assert_eq!(q.map_tool("get_menu"), Some((Resource::Menu, Action::Read)));
        assert_eq!(
            q.map_tool("delete_everything"),
            None,
            "unmapped ⇒ fail-closed drop"
        );
    }

    #[test]
    fn digest_reacts_to_tool_set_change() {
        let q = AgentQuirks::mcp_server(allowlist());
        let d1 = q.tool_map_digest(&["get_menu".into(), "read_order".into()]);
        // Server drops a tool ⇒ different digest.
        let d2 = q.tool_map_digest(&["get_menu".into()]);
        assert_ne!(
            d1, d2,
            "changing the advertised tool set changes the digest"
        );
        // Advertising an UNMAPPED tool does not change the digest (it is dropped).
        let d3 = q.tool_map_digest(&["get_menu".into(), "read_order".into(), "evil".into()]);
        assert_eq!(d1, d3, "unmapped tools do not enter the digest");
    }
}
