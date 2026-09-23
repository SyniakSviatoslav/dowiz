use super::*;

const KEY: &[u8] = b"a hub signing key, 32 bytes long";

fn claims(role: Role, now: i64) -> Claims {
    Claims {
        role,
        subject: "person_1".into(),
        session: "sess_abc".into(),
        scope: String::new(),
        caps: String::new(),
        issued_ms: now,
        expires_ms: now + ACCESS_TTL_MS,
    }
}

#[test]
fn a_minted_token_verifies_and_round_trips() {
    let now = 1_700_000_000_000;
    let c = claims(Role::Owner, now);
    let t = mint(KEY, &c);
    assert_eq!(verify(KEY, &t, now + 1000).expect("verify"), c);
}

/// The single most important test in this file: a token signed by someone
/// else must not be accepted.
#[test]
fn a_token_signed_with_another_key_is_refused() {
    let now = 1_700_000_000_000;
    let t = mint(b"a different hub's signing key!!!", &claims(Role::Owner, now));
    assert_eq!(verify(KEY, &t, now), Err(TokenError::BadSignature));
}

/// Flipping ANY byte of the payload must invalidate the token -- this is
/// what stops a courier token being edited into an owner token.
#[test]
fn every_payload_byte_is_covered_by_the_mac() {
    let now = 1_700_000_000_000;
    let t = mint(KEY, &claims(Role::Courier, now));
    let (payload, mac) = t.split_once('.').unwrap();
    for i in 0..payload.len() {
        let mut bytes = payload.as_bytes().to_vec();
        // Move to a different character in the alphabet.
        bytes[i] = if bytes[i] == b'A' { b'B' } else { b'A' };
        let altered = format!("{}.{}", String::from_utf8_lossy(&bytes), mac);
        assert!(
            verify(KEY, &altered, now).is_err(),
            "byte {i} changed and the token still verified"
        );
    }
}

#[test]
fn an_expired_token_is_refused() {
    let now = 1_700_000_000_000;
    let t = mint(KEY, &claims(Role::Owner, now));
    assert!(verify(KEY, &t, now + ACCESS_TTL_MS - 1).is_ok());
    // Exactly at expiry is already too late; a token valid "until" a moment
    // must not be valid AT it.
    assert_eq!(verify(KEY, &t, now + ACCESS_TTL_MS), Err(TokenError::Expired));
}

/// An expired token must fail on its SIGNATURE first if the signature is
/// also wrong -- the error must never reveal that a forged token would
/// otherwise have been in date.
#[test]
fn signature_is_checked_before_expiry() {
    let now = 1_700_000_000_000;
    let t = mint(b"wrong key wrong key wrong key!!!", &claims(Role::Owner, now));
    assert_eq!(verify(KEY, &t, now + ACCESS_TTL_MS * 10), Err(TokenError::BadSignature));
}

/// A quote in the scope must not be able to inject claim fields into the
/// payload BEFORE it is signed. The MAC would be perfectly valid over the
/// forged claims, which is what makes this the dangerous case.
#[test]
fn a_quote_in_the_scope_cannot_forge_claims() {
    let now = 1_700_000_000_000;
    let mut c = claims(Role::Customer, now);
    c.scope = r#"ord_1","r":"owner","x":"#.into();
    let t = mint(KEY, &c);
    let got = verify(KEY, &t, now).expect("verify");
    assert_eq!(got.role, Role::Customer, "the role must not have been overwritten");
    assert_eq!(got.scope, c.scope, "the scope must survive intact");
}

#[test]
fn junk_is_refused_rather_than_panicking() {
    let now = 1_700_000_000_000;
    for junk in ["", ".", "a", "a.b.c", "....", "!!!.???", "Zm9v."] {
        assert!(verify(KEY, junk, now).is_err(), "accepted {junk:?}");
    }
}

#[test]
fn roles_round_trip_through_their_strings() {
    for r in [Role::Owner, Role::Courier, Role::Customer, Role::Refresh, Role::Staff] {
        assert_eq!(Role::from_str(r.as_str()), Some(r));
    }
    assert_eq!(Role::from_str("admin"), None);
    assert_eq!(Role::from_str("waiter"), None, "the fourth staff word is the operator's");
}

/// THE SIGNER'S CAPABILITIES RIDE ON THE SIGNATURE, which is the whole
/// point: `DECISIONS.md` OD-8 says trust is a signed capability, and a
/// capability list the caller could edit would be neither.
#[test]
fn a_staff_token_carries_its_capabilities() {
    let now = 1_700_000_000_000;
    let mut c = claims(Role::Staff, now);
    c.caps = "advance,take_orders".into();
    let t = mint(KEY, &c);
    let got = verify(KEY, &t, now + 1000).expect("verify");
    assert_eq!(got.role, Role::Staff);
    assert_eq!(got.caps, "advance,take_orders");
}

/// And editing one byte of that list breaks the token, for the same reason
/// `every_payload_byte_is_covered_by_the_mac` exists: a capability that
/// could be typed in is not a signed capability.
#[test]
fn a_capability_cannot_be_added_to_a_minted_token() {
    let now = 1_700_000_000_000;
    let mut c = claims(Role::Staff, now);
    c.caps = "advance".into();
    let t = mint(KEY, &c);
    let (payload, mac) = t.split_once('.').unwrap();
    let forged = crate::crypto::b64url_encode(
        encode_payload(&Claims { caps: "advance,open_till".into(), ..c }).as_bytes(),
    );
    assert_ne!(forged, payload, "the forgery must actually differ");
    let altered = format!("{forged}.{mac}");
    assert_eq!(verify(KEY, &altered, now), Err(TokenError::BadSignature));
}

/// A quote in the capability list must not be able to inject claim fields
/// before the payload is signed -- the same hole `a_quote_in_the_scope`
/// covers, on the field this commit adds.
#[test]
fn a_quote_in_the_capability_list_cannot_forge_claims() {
    let now = 1_700_000_000_000;
    let mut c = claims(Role::Staff, now);
    c.caps = r#"advance","r":"owner","x":"#.into();
    let t = mint(KEY, &c);
    let got = verify(KEY, &t, now).expect("verify");
    assert_eq!(got.role, Role::Staff, "the role must not have been overwritten");
    assert_eq!(got.caps, c.caps, "the list must survive intact");
}

/// A TOKEN MINTED BEFORE THIS FIELD EXISTED STILL VERIFIES, and confers no
/// capability. Deploying this must not sign out every owner and courier
/// holding a live token, and the missing field must read as the empty set
/// rather than as anything else.
#[test]
fn a_token_minted_before_capabilities_existed_still_verifies_with_none() {
    let now = 1_700_000_000_000;
    let legacy = format!(
        r#"{{"r":"owner","s":"person_1","j":"sess_abc","p":"","i":{now},"e":{}}}"#,
        now + ACCESS_TTL_MS
    );
    let payload = crate::crypto::b64url_encode(legacy.as_bytes());
    let mac = crate::crypto::hmac_sha256(KEY, payload.as_bytes());
    let t = format!("{payload}.{}", crate::crypto::b64url_encode(&mac));
    let got = verify(KEY, &t, now + 1000).expect("a token in flight must not be invalidated");
    assert_eq!(got.role, Role::Owner);
    assert_eq!(got.caps, "", "an absent list is no capabilities, never all of them");
}
