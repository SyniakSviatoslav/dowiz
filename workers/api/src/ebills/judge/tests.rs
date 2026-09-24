//! A CREATE'S ANSWER, RULED (EBILLS-WRITE-PATH §2.1): each ruling beside its
//! twin. The bodies are the measured shape of sale 7783's read-back, cut to
//! the fields the ruling reads.

use super::*;
use serde_json::json;

fn answer(status: u16, ct: &str, body: &str) -> Answer {
    Answer { status, content_type: ct.into(), body: body.into(), ..Answer::default() }
}

fn sale(log: serde_json::Value, fiscal: &str) -> String {
    json!({ "id": 9001, "invOrdNum": 57, "fic": null, "fiscalSatus": fiscal, "status": "CLOSED", "logCis": log }).to_string()
}

#[test]
fn a_200_with_success_is_fiscalised_with_its_codes() {
    let ok = sale(json!([{ "status": "SUCCESS", "iic": "IIC7783", "fic": "FIC-7783" }]), "FINISHED");
    assert_eq!(
        created(&answer(200, "application/json", &ok)),
        Created::Fiscalised { sale_id: 9001, iic: "IIC7783".into(), fic: "FIC-7783".into(), inv_ord_num: "57".into() }
    );
}

/// Its twin: a 200 with ERROR is a sale that EXISTS, not fiscalised -- never
/// "no sale", and never Fiscalised.
#[test]
fn a_200_with_error_is_a_created_sale_that_was_not_fiscalised() {
    let err = sale(json!([{ "status": "ERROR", "faultStringMsg": "Certifikata ka skaduar" }]), "WEBSERVICEERROR");
    assert_eq!(
        created(&answer(200, "application/json", &err)),
        Created::Unfiscalised { sale_id: 9001, fault: "Certifikata ka skaduar".into() }
    );
    let pending = sale(json!([]), "PENDING");
    assert!(matches!(created(&answer(200, "application/json", &pending)), Created::Unfiscalised { sale_id: 9001, .. }));
    let no_fic = sale(json!([{ "status": "SUCCESS" }]), "FINISHED");
    assert!(matches!(created(&answer(200, "application/json", &no_fic)), Created::Unfiscalised { .. }), "SUCCESS without a fic is not codes");
}

/// A 5xx, a redirect, a page, a JSON that is not a sale: whether a sale
/// exists is UNKNOWN -- the ruling that forces a reconcile before any retry.
#[test]
fn what_does_not_show_a_sale_is_unknown_never_no_sale() {
    for a in [
        answer(500, "application/json", r#"{"title":"Internal Server Error"}"#),
        answer(502, "text/html", "<html>bad gateway</html>"),
        answer(302, "", ""),
        answer(200, "text/html", "<!doctype html>"),
        answer(200, "application/json", r#"{"ok":true}"#),
        answer(200, "application/problem+json", r#"{"title":"x"}"#),
        answer(200, "application/json", "not json"),
    ] {
        assert!(matches!(created(&a), Created::Unknown(_)), "{a:?}");
    }
}

/// Refused before a sale existed: a 4xx with its key; 401/403 is the session.
#[test]
fn a_4xx_is_refused_and_401_403_is_the_session() {
    let neg = r#"{"errorKey":"negInventory","title":"stock##23_1_1"}"#;
    assert_eq!(created(&answer(400, "application/problem+json", neg)), Created::Refused(400, "negInventory".into()));
    assert_eq!(created(&answer(404, "application/json", "{}")), Created::Refused(404, "{}".into()));
    assert!(matches!(created(&answer(403, "application/json", r#"{"message":"Could not verify the provided CSRF token"}"#)), Created::Auth(_)));
    assert!(matches!(created(&answer(401, "application/json", "{}")), Created::Auth(_)));
}

/// The detail's `sale` (a reconcile's read) is ruled by the same function.
#[test]
fn a_detail_read_back_is_ruled_like_a_create() {
    let v: serde_json::Value = serde_json::from_str(&sale(json!([{ "status": "SUCCESS", "iic": "I", "fic": "F" }]), "FINISHED")).unwrap();
    assert!(matches!(sale_state(&v), Created::Fiscalised { sale_id: 9001, .. }));
    assert!(matches!(sale_state(&json!({ "fiscalSatus": "FINISHED" })), Created::Unknown(_)), "no id, no sale");
}
