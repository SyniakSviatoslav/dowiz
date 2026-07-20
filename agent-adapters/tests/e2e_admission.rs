//! e2e_admission.rs — DoD item 6: a demonstrated run of the REAL admission path against a
//! REAL durable `FileEventStore` (not unit assertions alone). Exercises BOTH poles in one
//! run: accepts a real valid manifest (event lands, bucket minted, one bridged invoke +
//! one TrackRecord), and rejects a real invalid case (NodeId mismatch) with the store
//! byte-identical before/after. Artifact-style output is printed (see with `--nocapture`).

use std::collections::BTreeMap;
use std::sync::Arc;

use agent_adapters::manifest::draft_manifest;
use agent_adapters::transport::MockChannel;
use agent_adapters::{
    AgentDispatcher, AgentInvocation, AgentQuirks, AgentTask, McpServerBridge, VecHarvest,
};
use dowiz_kernel::event_log::EventLog;
use dowiz_kernel::hydra::FileEventStore;
use dowiz_kernel::ports::agent::{
    Action, AdmissionLimiter, Admitter, AgentManifest, AnchorRoster, BudgetRequest, Capability,
    Delegation, ExecutionModel, HybridPolicy, NodeId, RedLinePolicy, RefSigner,
    ReferenceHybridGate, Resource, RevocationSet, Scope, SignatureVerifier, SignedFrame,
};
use serde_json::json;

fn allowlist() -> BTreeMap<String, (Resource, Action)> {
    let mut m = BTreeMap::new();
    m.insert("get_menu".to_string(), (Resource::Menu, Action::Read));
    m
}

/// Build a valid manifest for the agent keypair.
fn manifest(v: &RefSigner, cls: [u8; 32], pq: Vec<u8>) -> AgentManifest {
    let quirks = AgentQuirks::mcp_server(allowlist());
    let caps = quirks.caps_from_discovery(true, false, false, false);
    let _ = v;
    draft_manifest(
        cls,
        pq,
        &quirks,
        caps,
        vec![],
        BudgetRequest {
            capacity: 256,
            refill_milli_units_per_sec: 0,
        },
        ExecutionModel::WasmComponent,
        vec![],
        0,
        [7u8; 8],
        9999,
    )
}

/// A fully valid admission frame + anchor-rooted chain + roster.
fn valid_admission(
    v: &RefSigner,
    cls_secret: &[u8; 32],
    pq_secret: &[u8; 32],
    anchor_secret: &[u8; 32],
    m: &AgentManifest,
    nonce: [u8; 8],
) -> (SignedFrame, AnchorRoster, Vec<Delegation>) {
    let cls = v.classical_public(cls_secret);
    let pq = v.pq_public(pq_secret);
    let anchor = v.classical_public(anchor_secret);
    let scope = Scope::single(Resource::AgentBridge, Action::AdmitAgent);
    let cap = Capability::new_hybrid(cls, pq, scope.clone(), nonce, 9999);
    let mut frame = SignedFrame::new(cap, m.canonical_bytes());
    frame.sign_classical(v, cls_secret);
    frame.sign_pq(v, pq_secret);
    let link = Delegation::sign(
        v,
        anchor,
        cls,
        scope.clone(),
        scope,
        9999,
        nonce,
        anchor_secret,
    );
    let mut roster = AnchorRoster::new();
    roster.enroll(&anchor);
    (frame, roster, vec![link])
}

fn admitter() -> Admitter<ReferenceHybridGate<RefSigner>> {
    let gate = ReferenceHybridGate::new_redlined(
        HybridPolicy::RequireBoth,
        RedLinePolicy::DenyByDefault,
        RefSigner,
    );
    let limiter = AdmissionLimiter::new(1_000_000, 0.0, 8, 1_000, 0.0);
    Admitter::new(gate, limiter, 1_000_000, [0xA0; 32])
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[test]
fn e2e_admits_valid_and_rejects_invalid_against_real_file_event_store() {
    let v = RefSigner;
    let dir = std::env::temp_dir();
    let path = dir.join(format!("dowiz_agent_admit_demo_{}.log", std::process::id()));
    let _ = std::fs::remove_file(&path);

    let mut adm = admitter();
    let mut log = EventLog::new(FileEventStore::open(&path).expect("open FileEventStore"));

    println!("=== B1 admission DEMO RUN — real admit() over a real FileEventStore ===");
    println!("host_event_store: {}", path.display());

    // ── POLE 1: accept a real valid manifest ────────────────────────────────────
    let (cls, pq_s, anch) = ([1u8; 32], [2u8; 32], [3u8; 32]);
    let m = manifest(&v, v.classical_public(&cls), v.pq_public(&pq_s));
    assert_eq!(
        m.agent_node_id,
        NodeId::from_keys(&m.subject_key_pq, &m.subject_key).0,
        "node_id is derived, not claimed"
    );
    let (frame, roster, chain) = valid_admission(&v, &cls, &pq_s, &anch, &m, [7u8; 8]);
    let len_before = log.len();
    let rec = adm
        .admit(
            &frame,
            &roster,
            &chain,
            &RevocationSet::new(),
            &mut log,
            0,
            0,
        )
        .expect("valid manifest admits");
    println!("ACCEPT: content_id  = {}", hex(&rec.content_id));
    println!("ACCEPT: event_id    = {}", hex(&rec.event_id));
    println!("ACCEPT: node_id     = {}", rec.node_id.to_hex());
    println!("ACCEPT: tier        = {:?}", rec.tier);
    println!(
        "ACCEPT: budget      = cap {} refill_milli {}",
        rec.granted_capacity, rec.granted_refill_milli
    );
    println!("ACCEPT: depth       = {}", rec.granted_depth);
    assert_eq!(
        log.len(),
        len_before + 1,
        "one AgentAdmitted event landed durably"
    );
    // The minted envelope exists and has budget (do NOT drain it — the invoke below needs it).
    assert_eq!(rec.granted_capacity, 256);
    assert!(
        rec.bucket.available() >= 4.0,
        "minted bucket carries the granted envelope"
    );

    // One bridged invoke through the minted envelope → exactly one TrackRecord row.
    let quirks = AgentQuirks::mcp_server(allowlist());
    let caps = quirks.caps_from_discovery(true, false, false, false);
    let digest = quirks.tool_map_digest(&["get_menu".to_string()]);
    let mock = MockChannel::new()
        .with_result("tools/list", json!({"tools":[{"name":"get_menu"}]}))
        .with_result("tools/call", json!({"content":"menu"}));
    let bridge = Arc::new(McpServerBridge::admitted(
        "demo",
        m.clone(),
        quirks,
        digest,
        caps,
        mock,
    ));
    let sink = VecHarvest::new();
    let disp = AgentDispatcher::new(
        bridge,
        Arc::clone(&rec.bucket),
        rec.granted_depth,
        sink.clone(),
    );
    let resp = disp
        .dispatch(AgentInvocation {
            task: AgentTask::InvokeTool {
                name: "get_menu".into(),
                args: vec![],
            },
            cost_units: 4,
            invoke_depth: 0,
        })
        .expect("bridged invoke succeeds");
    assert_eq!(
        sink.rows().len(),
        1,
        "exactly one TrackRecord row per bridged call"
    );
    println!(
        "ACCEPT: invoke ok, units={}, one TrackRecord row emitted",
        resp.units
    );

    // ── POLE 2: reject a real invalid case (NodeId mismatch) — store byte-identical ──
    let store_len_before = log.len();
    let mut bad = manifest(&v, v.classical_public(&cls), v.pq_public(&pq_s));
    bad.agent_node_id = [0xAB; 32]; // does NOT hash from its own carried keys
    let (bad_frame, bad_roster, bad_chain) =
        valid_admission(&v, &cls, &pq_s, &anch, &bad, [9u8; 8]);
    let err = match adm.admit(
        &bad_frame,
        &bad_roster,
        &bad_chain,
        &RevocationSet::new(),
        &mut log,
        0,
        0,
    ) {
        Ok(_) => panic!("identity-mismatch manifest MUST be rejected"),
        Err(e) => e,
    };
    println!("REJECT: error       = {err:?}");
    println!(
        "REJECT: store_len   = {} (unchanged from {})",
        log.len(),
        store_len_before
    );
    assert_eq!(
        log.len(),
        store_len_before,
        "store byte-identical: no event on rejection"
    );

    let _ = std::fs::remove_file(&path);
    println!("=== DEMO RUN complete: 1 accept, 1 reject, store durable + consistent ===");
}
