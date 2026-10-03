//! The MCP surface through its routes (W-COV C2): the public description, a JSON-RPC session
//! over an owner's token, role keys minted by staff and couriers, listed, revoked and refused.

use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use serde_json::{json, Value};

fn at(path: &str) -> String {
    format!("https://alpha.{PLATFORM_HOST}{path}")
}

fn rpc(site: &Site, token: &str, body: Value) -> crate::wire::Reply {
    site.run(crate::mcp::rpc, post(&at("/api/mcp"), &body).bearer(token).on("alpha"), &[])
}

#[test]
fn an_owner_initialises_lists_the_tools_and_a_batch_is_answered_in_order() {
    let site = Site::new();
    let owner = site.venue("alpha", "a@x.test");
    let d = site.run(crate::mcp::describe, get(&at("/api/mcp")).on("alpha"), &[]);
    assert_eq!(d.status_code(), 200, "{}", d.body_str());
    let init = rpc(&site, &owner, json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}));
    assert_eq!(init.status_code(), 200, "{}", init.body_str());
    assert_eq!(init.body_value()["id"], 1);
    let tools = rpc(&site, &owner, json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}));
    assert!(tools.body_value()["result"]["tools"].as_array().is_some_and(|t| !t.is_empty()), "{}", tools.body_str());
    let batch = rpc(&site, &owner, json!([{"jsonrpc": "2.0", "id": 3, "method": "tools/list"}, {"jsonrpc": "2.0", "id": 4, "method": "nope/nope"}]));
    let arr = batch.body_value();
    assert_eq!(arr[0]["id"], 3);
    assert!(arr[1]["error"].is_object(), "an unknown method is an error, not silence: {arr}");
    let note = rpc(&site, &owner, json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
    assert_eq!(note.status_code(), 202, "a notification has no answer");
    let junk = site.run(crate::mcp::rpc, crate::wire::Call::new(&at("/api/mcp"), worker::Method::Post).unwrap().with_body(b"{".to_vec()).bearer(&owner).on("alpha"), &[]);
    assert!(junk.body_value()["error"].is_object(), "{}", junk.body_str());
    let anon = site.run(crate::mcp::rpc, post(&at("/api/mcp"), &json!({"jsonrpc": "2.0", "id": 1, "method": "initialize"})).on("alpha"), &[]);
    assert_eq!(anon.status_code(), 401);
    let call = rpc(&site, &owner, json!({"jsonrpc": "2.0", "id": 5, "method": "tools/call", "params": {"name": "no_such_tool", "arguments": {}}}));
    assert!(call.body_str().contains("error") || call.body_str().contains("isError"), "{}", call.body_str());
}

#[test]
fn role_keys_are_minted_listed_used_and_revoked() {
    let site = Site::new();
    let owner = site.venue("alpha", "a@x.test");
    let waiter = site.staff("alpha", &owner, "w@x.test", "waiter");
    let r = site.run(crate::mcp::keys::staff_mint, post(&at("/api/staff/mcp/keys"), &json!({"label": "tablet"})).bearer(&waiter).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let v = r.body_value();
    let key = ["key", "token", "secret"].iter().find_map(|k| v[*k].as_str()).unwrap_or_else(|| panic!("{v}")).to_string();
    let id = v["id"].as_str().map(str::to_string);
    let list = site.run(crate::mcp::keys::staff_list, get(&at("/api/staff/mcp/keys")).bearer(&waiter).on("alpha"), &[]);
    assert!(list.body_str().contains("tablet"), "{}", list.body_str());
    let owner_view = site.run(crate::mcp::owner_list, get(&at("/api/owner/mcp/keys")).bearer(&owner).on("alpha"), &[]);
    assert!(owner_view.body_str().contains("tablet"), "the owner sees every key: {}", owner_view.body_str());
    let used = rpc(&site, &key, json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}));
    assert_eq!(used.status_code(), 200, "the role key opens a session: {}", used.body_str());
    let id = id.or_else(|| list.body_value().to_string().split("\"id\":\"").nth(1).map(|s| s.split('"').next().unwrap().to_string())).expect("id");
    let r = site.run(crate::mcp::keys::staff_revoke, post(&at("/api/staff/mcp/keys/revoke"), &json!({"id": id})).bearer(&waiter).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let after = rpc(&site, &key, json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}));
    assert_eq!(after.status_code(), 401, "a revoked key: {}", after.body_str());
    let (rider, _) = site.courier("alpha", &owner, "+355697777777");
    let r = site.run(crate::mcp::keys::courier_mint, post(&at("/api/courier/mcp/keys"), &json!({"label": "watch"})).bearer(&rider).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let l = site.run(crate::mcp::keys::courier_list, get(&at("/api/courier/mcp/keys")).bearer(&rider).on("alpha"), &[]);
    assert!(l.body_str().contains("watch"));
    let wrong_role = site.run(crate::mcp::keys::courier_mint, post(&at("/api/courier/mcp/keys"), &json!({"label": "x"})).bearer(&waiter).on("alpha"), &[]);
    assert!(wrong_role.status_code() >= 400, "a waiter minted a courier key: {}", wrong_role.body_str());
}

/// W-INT2 #30: what an agent's key reaches. A tool is dispatched in process through
/// `crate::route` (live-only), so natively the link is proven in two halves: every tool of every
/// role names a route the router really serves, with the same verb; and a key nobody minted is
/// refused at the door while the venue's owner token answers.
#[test]
fn every_mcp_tool_is_a_route_the_router_serves_and_an_unknown_key_is_refused() {
    let router = include_str!("../../lib.rs");
    let all = super::tools::OWNER_TOOLS.iter().chain(super::tools::STAFF_TOOLS).chain(super::tools::COURIER_TOOLS);
    let mut n = 0;
    for t in all {
        let mut path = String::new();
        let mut hole = false;
        for c in t.path.chars() {
            match c {
                '{' => {
                    hole = true;
                    path.push(':');
                }
                '}' => hole = false,
                c => path.push(c),
            }
        }
        assert!(!hole, "{}", t.path);
        let verb = match t.verb {
            super::tools::Verb::Get => "get",
            super::tools::Verb::Post => "post",
        };
        let line = format!(".{verb}_async(\"{path}\"");
        assert!(router.contains(&line), "tool {} calls {line} which the router does not serve", t.name);
        n += 1;
    }
    assert!(n >= 30, "{n} tools");

    let site = Site::new();
    let owner = site.venue("alpha", "a@x.test");
    let forged = rpc(&site, "dowiz_mcp_not-a-real-key", json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}));
    assert_eq!(forged.status_code(), 401, "{}", forged.body_str());
    let ok = rpc(&site, &owner, json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}));
    assert_eq!(ok.status_code(), 200, "{}", ok.body_str());
}
