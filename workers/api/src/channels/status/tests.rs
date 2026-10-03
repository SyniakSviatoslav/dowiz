use super::*;

fn delivery(statuses: Value) -> Value {
    json!({"object": "whatsapp_business_account", "entry": [{"changes": [{"value": {
        "messaging_product": "whatsapp", "statuses": statuses}}]}]})
}

fn sent_reply(log: &mut LogImage, peer: &str, wamid: &str) {
    let rec = json!({"direction": "out", "externalId": wamid, "peer": peer, "text": "yes"}).to_string();
    log.append("m", &format!("whatsapp/{peer}"), &rec).unwrap();
}

fn conv(p: &str) -> String {
    format!("whatsapp/{p}")
}

#[test]
fn a_status_names_its_message_its_recipient_and_its_moment() {
    let got = statuses_of(
        &delivery(json!([
            {"id": "wamid.A", "status": "delivered", "timestamp": "1700000001", "recipient_id": "355"},
            {"id": "wamid.B", "status": "deleted", "timestamp": "1700000002", "recipient_id": "355"},
            {"id": "wamid.C", "status": "read", "recipient_id": "355"}
        ])),
        7,
    );
    assert_eq!(
        got,
        vec![
            Status { peer: "355".into(), wamid: "wamid.A".into(), state: "delivered", at_ms: 1_700_000_001_000 },
            Status { peer: "355".into(), wamid: "wamid.C".into(), state: "read", at_ms: 7 },
        ]
    );
}

#[test]
fn only_a_reply_this_venue_sent_is_marked_and_only_forward() {
    let mut log = LogImage::create().unwrap();
    sent_reply(&mut log, "355", "wamid.A");
    let s = |w: &str, st: &'static str| Status { peer: "355".into(), wamid: w.into(), state: st, at_ms: 1 };
    // Not ours: nothing written.
    assert_eq!(record(&mut log, &[s("wamid.X", "read")], conv, "m"), Ok(0));
    // Ours, forward twice, then a repeat and a step back: two records, the furthest wins.
    assert_eq!(record(&mut log, &[s("wamid.A", "delivered"), s("wamid.A", "read")], conv, "m"), Ok(2));
    assert_eq!(record(&mut log, &[s("wamid.A", "read"), s("wamid.A", "sent")], conv, "m"), Ok(0));
    let best = latest(&log.about(K_STATUS, Some("whatsapp/355"), usize::MAX));
    assert_eq!(best.get("wamid.A").map(String::as_str), Some("read"));
    assert_eq!(best.len(), 1);
}
