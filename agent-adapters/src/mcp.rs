//! mcp.rs — `McpServerBridge`: the ONE reference `AgentBridge` implementation.
//!
//! Closed-scope narrowing: every host→server call goes through the operator-authored tool
//! allowlist (unmapped ⇒ fail-closed `ToolNotAllowed` drop). Post-admission drift is
//! detected by recomputing the live tool-map digest on each invoke and comparing it to the
//! digest bound at admission (`listChanged` / server substitution ⇒ `DigestDrift`, refuse
//! until re-admission). Server-initiated `sampling/createMessage` / elicitation requests
//! are ALWAYS refused — an admitted agent must never drive the host's LLM (RC-2).

use dowiz_kernel::ports::agent::{
    AgentBridge, AgentCaps, AgentError, AgentInvocation, AgentManifest, AgentResponse, AgentTask,
};
use serde_json::json;

use crate::quirks::AgentQuirks;
use crate::transport::{JsonRpcTransport, RpcChannel};

/// A bridge to one MCP server. Constructed FROM an admission (the admitted manifest, the
/// operator quirks profile, and the tool-map digest bound at admission).
pub struct McpServerBridge<C: RpcChannel> {
    id_str: String,
    manifest: AgentManifest,
    quirks: AgentQuirks,
    admitted_digest: [u8; 32],
    caps: AgentCaps,
    transport: JsonRpcTransport<C>,
}

impl<C: RpcChannel> McpServerBridge<C> {
    /// Build a bridge from an admission. `admitted_digest` is the tool-map digest captured
    /// at admission (from the discovery snapshot the operator signed over).
    pub fn admitted(
        server_id: &str,
        manifest: AgentManifest,
        quirks: AgentQuirks,
        admitted_digest: [u8; 32],
        caps: AgentCaps,
        channel: C,
    ) -> Self {
        McpServerBridge {
            id_str: format!("mcp:{server_id}"),
            manifest,
            quirks,
            admitted_digest,
            caps,
            transport: JsonRpcTransport::new(channel),
        }
    }

    /// The server's currently-advertised tool names (live `tools/list`).
    fn live_tools(&self) -> Result<Vec<String>, AgentError> {
        let result = self.transport.call("tools/list", json!({}))?;
        let tools = result
            .get("tools")
            .and_then(|t| t.as_array())
            .ok_or_else(|| AgentError::BadRequest("tools/list missing tools[]".into()))?;
        Ok(tools
            .iter()
            .filter_map(|t| {
                t.get("name")
                    .and_then(|n| n.as_str())
                    .map(|s| s.to_string())
            })
            .collect())
    }

    /// The live tool-map digest (over the CURRENT advertised ∩ allow-listed set).
    fn live_digest(&self) -> Result<[u8; 32], AgentError> {
        Ok(self.quirks.tool_map_digest(&self.live_tools()?))
    }

    /// Refuse a server-initiated request that would invert control (RC-2). Sampling and
    /// elicitation requests are always refused for the MCP profile.
    pub fn handle_server_request(&self, method: &str) -> Result<(), AgentError> {
        if (self.quirks.refuse_sampling && method.starts_with("sampling/"))
            || (self.quirks.refuse_elicitation && method.starts_with("elicitation/"))
        {
            return Err(AgentError::ControlInversionRefused);
        }
        Ok(())
    }
}

impl<C: RpcChannel> AgentBridge for McpServerBridge<C> {
    fn id(&self) -> &str {
        &self.id_str
    }
    fn caps(&self) -> AgentCaps {
        self.caps
    }
    fn manifest(&self) -> &AgentManifest {
        &self.manifest
    }

    fn invoke(&self, req: &AgentInvocation) -> Result<AgentResponse, AgentError> {
        // Drift check FIRST: the live tool-map must still match the admitted digest.
        if self.live_digest()? != self.admitted_digest {
            return Err(AgentError::DigestDrift);
        }
        let result = match &req.task {
            AgentTask::InvokeTool { name, args } => {
                // Closed-scope narrowing: an unmapped tool is a fail-closed drop.
                if self.quirks.map_tool(name).is_none() {
                    return Err(AgentError::ToolNotAllowed(name.clone()));
                }
                let arg_val: serde_json::Value = if args.is_empty() {
                    json!({})
                } else {
                    serde_json::from_slice(args).unwrap_or_else(|_| json!({}))
                };
                self.transport
                    .call("tools/call", json!({ "name": name, "arguments": arg_val }))?
            }
            AgentTask::ReadResource { uri } => self
                .transport
                .call("resources/read", json!({ "uri": uri }))?,
            AgentTask::RenderPrompt { name } => self
                .transport
                .call("prompts/get", json!({ "name": name }))?,
        };
        let content = serde_json::to_vec(&result).unwrap_or_default();
        let units = (content.len() as u64).max(1);
        Ok(AgentResponse { content, units })
    }

    fn health(&self) -> Result<(), AgentError> {
        self.transport
            .call("ping", json!({}))
            .map(|_| ())
            .map_err(|_| AgentError::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::draft_manifest;
    use crate::transport::MockChannel;
    use dowiz_kernel::ports::agent::{
        Action, BudgetRequest, ExecutionModel, RefSigner, Resource, SignatureVerifier,
    };
    use std::collections::BTreeMap;

    fn allowlist() -> BTreeMap<String, (Resource, Action)> {
        let mut m = BTreeMap::new();
        m.insert("get_menu".to_string(), (Resource::Menu, Action::Read));
        m.insert("read_order".to_string(), (Resource::Order, Action::Read));
        m
    }

    fn tools_list(names: &[&str]) -> serde_json::Value {
        json!({ "tools": names.iter().map(|n| json!({"name": n})).collect::<Vec<_>>() })
    }

    fn build_bridge(mock: MockChannel, advertised: &[&str]) -> McpServerBridge<MockChannel> {
        let v = RefSigner;
        let cls = v.classical_public(&[1u8; 32]);
        let pq = v.pq_public(&[2u8; 32]);
        let quirks = AgentQuirks::mcp_server(allowlist());
        let caps = quirks.caps_from_discovery(true, true, false, false);
        let manifest = draft_manifest(
            cls,
            pq,
            &quirks,
            caps,
            vec![],
            BudgetRequest {
                capacity: 100,
                refill_milli_units_per_sec: 0,
            },
            ExecutionModel::WasmComponent,
            vec![],
            0,
            [3u8; 8],
            9999,
        );
        // Digest bound at admission from the discovery snapshot.
        let admitted_digest =
            quirks.tool_map_digest(&advertised.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        McpServerBridge::admitted("demo", manifest, quirks, admitted_digest, caps, mock)
    }

    fn invoke_tool(name: &str) -> AgentInvocation {
        AgentInvocation {
            task: AgentTask::InvokeTool {
                name: name.into(),
                args: vec![],
            },
            cost_units: 1,
            invoke_depth: 0,
        }
    }

    #[test]
    fn invoke_maps_allow_listed_tool() {
        let mock = MockChannel::new()
            .with_result("tools/list", tools_list(&["get_menu", "read_order"]))
            .with_result("tools/call", json!({"content": "menu-bytes"}));
        let bridge = build_bridge(mock, &["get_menu", "read_order"]);
        let resp = bridge
            .invoke(&invoke_tool("get_menu"))
            .expect("allow-listed tool invokes");
        assert!(resp.units > 0);
    }

    #[test]
    fn unmapped_tool_is_refused() {
        let mock =
            MockChannel::new().with_result("tools/list", tools_list(&["get_menu", "read_order"]));
        let bridge = build_bridge(mock, &["get_menu", "read_order"]);
        assert_eq!(
            bridge.invoke(&invoke_tool("rm_rf")),
            Err(AgentError::ToolNotAllowed("rm_rf".into()))
        );
    }

    // ── §4 criterion 9 — digest drift re-admits ─────────────────────────────────
    #[test]
    fn crit9_tool_list_change_after_admission_forces_readmission() {
        let mock = MockChannel::new()
            .with_result("tools/list", tools_list(&["get_menu", "read_order"]))
            .with_result("tools/call", json!({"content": "ok"}));
        // Admitted with the 2-tool snapshot.
        let bridge = build_bridge(mock, &["get_menu", "read_order"]);
        // Before drift: invoke succeeds.
        assert!(bridge.invoke(&invoke_tool("get_menu")).is_ok());
        // Server DROPS a tool after admission (listChanged / substitution).
        bridge
            .transport
            .channel()
            .set_result("tools/list", tools_list(&["get_menu"]));
        // Next invoke fails the digest check ⇒ refuse until re-admission.
        assert_eq!(
            bridge.invoke(&invoke_tool("get_menu")),
            Err(AgentError::DigestDrift)
        );
    }

    #[test]
    fn server_initiated_sampling_is_refused() {
        let bridge = build_bridge(MockChannel::new(), &["get_menu"]);
        assert_eq!(
            bridge.handle_server_request("sampling/createMessage"),
            Err(AgentError::ControlInversionRefused)
        );
        assert_eq!(
            bridge.handle_server_request("elicitation/create"),
            Err(AgentError::ControlInversionRefused)
        );
        // A normal server request is fine.
        assert_eq!(
            bridge.handle_server_request("notifications/progress"),
            Ok(())
        );
    }
}
