//! RFC 8292 §2.4's example verifies under its own key with the verifier used
//! here, our tokens have exactly the RFC's shape, and a mismatched secret is
//! refused.
use super::*;
use p256::ecdsa::signature::Verifier;
use p256::ecdsa::VerifyingKey;

// RFC 8292 §2.4 Figure 1, copied from rfc-editor.org/rfc/rfc8292.txt.
const RFC_T: &str = "eyJ0eXAiOiJKV1QiLCJhbGciOiJFUzI1NiJ9.eyJhdWQiOiJodHRwczovL3B1c2guZXhhbXBsZS5uZXQiLCJleHAiOjE0NTM1MjM3NjgsInN1YiI6Im1haWx0bzpwdXNoQGV4YW1wbGUuY29tIn0.i3CYb7t4xfxCDquptFOepC9GAu_HLGkMlMuCGSK2rpiUfnK9ojFwDXb1JrErtmysazNjjvW2L9OkSSHzvoD1oA";
const RFC_K: &str = "BA1Hxzyi1RUM1b5wjxsn7nGxAszw2u61m164i3MrAIxHF6YK5h4SDYic-dRuU_RCPCfA5aq9ojSwk5Y2EmClBPs";

/// A fixed TEST key (never the live one): the RFC 8291 sender key.
const TEST_D: &str = "yfWPiYE-n46HLnH0KqZOF1fJJU3MYrct3AELtAQ-oRw";
const TEST_PUB: &str = "BP4z9KsN6nGRTbVYI_c7VJSPQTBtkgcy27mlmlMoZIIgDll6e3vCYLocInmYWAmS6TlzAC8wEqKK6PBru3jl7A8";

fn verifies(jwt: &str, k: &str) -> bool {
    let (input, sig) = jwt.rsplit_once('.').unwrap();
    let key = VerifyingKey::from_sec1_bytes(&B64.decode(k).unwrap()).unwrap();
    let Ok(sig) = Signature::from_slice(&B64.decode(sig).unwrap()) else { return false };
    key.verify(input.as_bytes(), &sig).is_ok()
}

#[test]
fn the_rfc_8292_example_token_verifies_under_its_key_and_a_flipped_one_does_not() {
    assert!(verifies(RFC_T, RFC_K));
    let mut bent = RFC_T.to_string();
    bent.replace_range(40..41, if &bent[40..41] == "A" { "B" } else { "A" });
    assert!(!verifies(&bent, RFC_K));
}

#[test]
fn our_token_has_the_rfc_shape_byte_for_byte_and_verifies() {
    let s = Signer::new(TEST_D, TEST_PUB).unwrap();
    let jwt = s.jwt("https://push.example.net", 1453523768);
    let parts: Vec<&str> = jwt.split('.').collect();
    assert_eq!(parts.len(), 3);
    // The RFC's header segment exactly; our claims in the RFC's order.
    assert_eq!(parts[0], RFC_T.split('.').next().unwrap());
    let claims: serde_json::Value = serde_json::from_slice(&B64.decode(parts[1]).unwrap()).unwrap();
    assert_eq!(claims, serde_json::json!({ "aud": "https://push.example.net", "exp": 1453523768, "sub": SUBJECT }));
    assert_eq!(B64.decode(parts[2]).unwrap().len(), 64, "ES256 is raw r||s, not DER");
    assert!(verifies(&jwt, TEST_PUB));
    assert!(!verifies(&jwt, RFC_K), "and not under somebody else's key");
}

#[test]
fn the_header_names_the_endpoints_origin_and_expires_within_24_hours() {
    let s = Signer::new(TEST_D, TEST_PUB).unwrap();
    let now_ms = 1_759_500_000_000;
    let h = s.authorization("https://fcm.googleapis.com/fcm/send/abc:def?x=1", now_ms).unwrap();
    let (t, k) = h.strip_prefix("vapid t=").unwrap().split_once(", k=").unwrap();
    assert_eq!(k, TEST_PUB);
    let claims: serde_json::Value = serde_json::from_slice(&B64.decode(t.split('.').nth(1).unwrap()).unwrap()).unwrap();
    assert_eq!(claims["aud"], "https://fcm.googleapis.com");
    let exp = claims["exp"].as_i64().unwrap();
    assert!(exp > now_ms / 1000 && exp <= now_ms / 1000 + 24 * 3600);
    assert!(verifies(t, TEST_PUB));
}

#[test]
fn a_secret_that_is_not_the_pair_of_the_public_key_is_refused_and_its_pair_is_not() {
    match Signer::new(TEST_D, PUBLIC_KEY) {
        Err(VapidError::NotOurKey { derived }) => assert_eq!(derived, TEST_PUB),
        other => panic!("expected NotOurKey, got {:?}", other.err()),
    }
    assert!(Signer::new(TEST_D, TEST_PUB).is_ok());
    assert_eq!(Signer::new("not base64 !", TEST_PUB).err(), Some(VapidError::BadSecret));
    assert_eq!(Signer::new(&B64.encode([0u8; 32]), TEST_PUB).err(), Some(VapidError::BadSecret), "zero is no scalar");
}

#[test]
fn the_audience_is_an_https_origin_only() {
    assert_eq!(audience("https://Updates.Push.Services.Mozilla.com/wpush/v2/gAAA"), Some("https://updates.push.services.mozilla.com".into()));
    assert_eq!(audience("https://web.push.apple.com:443/Q"), Some("https://web.push.apple.com:443".into()));
    assert_eq!(audience("http://fcm.googleapis.com/x"), None);
    assert_eq!(audience("https://user@fcm.googleapis.com/x"), None);
    assert_eq!(audience("https:///x"), None);
}

#[test]
fn the_public_key_in_code_is_a_p256_point() {
    let raw = B64.decode(PUBLIC_KEY).unwrap();
    assert_eq!(raw.len(), 65);
    assert!(VerifyingKey::from_sec1_bytes(&raw).is_ok());
}
