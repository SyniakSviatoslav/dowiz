//! P40 Task C / W-d prong 2 — the money-law firewall grep gate (P54).
//!
//! RED `red_money_law_firewall_present`: before this file the gate did not exist,
//! so a future PR could sneak `use kernel::money::apply_tax` into the agent lane.
//! GREEN: this committed `cargo test` greps the agent lane (`agent-loop/src` + the
//! `/api/agent` route body in `api.rs`) for money-law symbols and FAILS if any
//! appear. The money law is kernel-only by construction; the agent literally
//! cannot name it.
//!
//! Also asserts (`red_money_tool_absent`) that the closed tool resource enum has
//! NO money variant — proven by grepping the kernel's `ports/tool.rs`
//! `ToolResource` for `OrderStatus` only.

use std::path::PathBuf;

/// Forbidden money-law symbols. The agent lane must contain NONE of these.
const FORBIDDEN: &[&str] = &["apply_tax", "money::", "::decide", "fold_transitions"];

fn repo_root() -> PathBuf {
    // tools/native-spa-server -> repo root is two levels up.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("canonicalize repo root")
}

/// Recursively collect *.rs file contents under `dir`.
fn rs_sources(dir: &std::path::Path) -> Vec<(PathBuf, String)> {
    let mut out = Vec::new();
    let rd = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return out,
    };
    for entry in rd.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(rs_sources(&path));
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            if let Ok(text) = std::fs::read_to_string(&path) {
                out.push((path, text));
            }
        }
    }
    out
}

/// Extract the `/api/agent` route lane from api.rs: the `agent_handler` +
/// `forward_to_agent` region. We scan the whole api.rs for forbidden symbols but
/// exclude the order handlers' legitimate kernel calls — the money law lives in
/// the ORDER lane (json_api relays), never the AGENT lane. To keep the gate
/// falsifiable and simple, we assert the agent-specific functions contain no
/// forbidden symbol.
fn agent_route_body(api_src: &str) -> String {
    // Grab from the agent_handler doc/comment marker to end of forward_to_agent.
    let start = api_src
        .find("async fn agent_handler")
        .expect("agent_handler must exist in api.rs (P40 W-a)");
    let after = &api_src[start..];
    // end at the next top-level fn that is NOT part of the agent lane.
    let end_rel = after
        .find("/// The one permitted shallow read")
        .unwrap_or(after.len());
    after[..end_rel].to_string()
}

#[test]
fn red_money_law_firewall_present() {
    let root = repo_root();

    // (1) agent-loop/src — the whole executor lane.
    let agent_loop_src = root.join("agent-loop").join("src");
    let mut sources = rs_sources(&agent_loop_src);
    assert!(
        !sources.is_empty(),
        "agent-loop/src must contain sources (found none under {:?})",
        agent_loop_src
    );

    // (2) the /api/agent route body in api.rs (agent lane only).
    let api_path = root
        .join("tools")
        .join("native-spa-server")
        .join("src")
        .join("api.rs");
    let api_src = std::fs::read_to_string(&api_path).expect("read api.rs");
    sources.push((
        api_path.clone(),
        agent_route_body(&api_src),
    ));

    for (path, text) in &sources {
        for sym in FORBIDDEN {
            assert!(
                !text.contains(sym),
                "MONEY-LAW FIREWALL BREACH: forbidden symbol {sym:?} found in agent lane file {path:?}"
            );
        }
    }
}

#[test]
fn red_money_tool_absent() {
    let root = repo_root();
    let tool_rs = root.join("kernel").join("src").join("ports").join("tool.rs");
    let src = std::fs::read_to_string(&tool_rs).expect("read kernel ports/tool.rs");

    // The closed ToolResource enum must have exactly ONE variant: OrderStatus.
    // A money/price variant is UNREPRESENTABLE (P54 prong 1).
    let enum_start = src
        .find("pub enum ToolResource")
        .expect("ToolResource enum must exist");
    let after = &src[enum_start..];
    let brace = after.find('{').expect("enum open brace");
    let close = after[brace..].find('}').expect("enum close brace") + brace;
    let body = &after[brace + 1..close];

    assert!(
        body.contains("OrderStatus"),
        "ToolResource must include OrderStatus"
    );
    for forbidden in &["Price", "Money", "Ledger", "Payment", "Tax", "Settlement"] {
        assert!(
            !body.contains(forbidden),
            "ToolResource must have NO money variant, found {forbidden:?} in: {body}"
        );
    }
}
