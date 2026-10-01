use super::*;

/// The uniformity argument above is only true if the alphabet is exactly
/// 32 long. A thirty-third character added later would silently bias every
/// code the hub ever issues.
#[test]
fn the_alphabet_is_exactly_thirty_two_and_has_no_confusable_pairs() {
    assert_eq!(CODE_ALPHABET.len(), 32);
    for c in [b'0', b'1', b'I', b'O'] {
        assert!(!CODE_ALPHABET.contains(&c), "{} is confusable", c as char);
    }
    let mut seen = CODE_ALPHABET.to_vec();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), 32, "a repeated character is a biased draw");
}

/// The platform supplies the bytes; this supplies the alphabet. A short
/// buffer is refused rather than padded, because a silently shorter code is
/// a silently weaker one.
#[test]
fn a_code_renders_from_bytes_the_caller_supplies() {
    let bytes: Vec<u8> = (0u8..16).collect();
    let a = new_invite_code_for_test(&bytes);
    assert_eq!(a.chars().count(), 16);
    assert!(a.bytes().all(|b| CODE_ALPHABET.contains(&b)), "{a}");
    // Deterministic in the bytes: the same buffer is the same code, which is
    // what lets a second implementation be checked against this one.
    assert_eq!(a, new_invite_code_for_test(&bytes));
    assert_eq!(invite_code_from(&[1, 2, 3]), None, "a short buffer must not pad");
    assert_eq!(invite_code_from(&bytes[..15]), None);
}

fn new_invite_code_for_test(b: &[u8]) -> String {
    invite_code_from(b).expect("16 bytes")
}

#[test]
fn a_code_is_sixteen_characters_from_that_alphabet() {
    let code = new_invite_code().unwrap();
    assert_eq!(code.chars().count(), 16);
    assert!(code.bytes().all(|b| CODE_ALPHABET.contains(&b)), "{code}");
    assert_ne!(code, new_invite_code().unwrap(), "two codes must not match");
}

const NOW: i64 = 1_789_000_000_000;
const WEEK: i64 = 7 * 24 * 60 * 60 * 1000;

fn roster() -> Roster {
    let mut r = Roster::create().unwrap();
    r.set_iterations(64);
    r
}

#[test]
fn an_invite_becomes_a_person_with_the_password_they_choose() {
    let mut r = roster();
    r.create_invite("+355690000001", Role::Courier, "Eni", "CODE1234CODE5678", NOW, WEEK)
        .unwrap();
    assert!(r.person("+355690000001").is_none(), "an invite is not yet an account");

    let p = r
        .claim_invite("+355690000001", "CODE1234CODE5678", "their-own-pw", NOW + 1000)
        .expect("claim");
    assert_eq!(p.role, Role::Courier);
    assert_eq!(p.name, "Eni");
    assert!(r.authenticate("+355690000001", "their-own-pw").is_some());
}

/// A code that survived its own use would be a second key to somebody
/// else's account.
#[test]
fn a_code_works_once() {
    let mut r = roster();
    r.create_invite("+355690000002", Role::Courier, "Blerim", "ONCEONCEONCEONCE", NOW, WEEK)
        .unwrap();
    r.claim_invite("+355690000002", "ONCEONCEONCEONCE", "pw", NOW).unwrap();
    assert_eq!(
        r.claim_invite("+355690000002", "ONCEONCEONCEONCE", "other-pw", NOW),
        Err(ClaimError::NoSuchInvite)
    );
    // And the first password still works: the second attempt changed nothing.
    assert!(r.authenticate("+355690000002", "pw").is_some());
    assert!(r.authenticate("+355690000002", "other-pw").is_none());
}

#[test]
fn the_wrong_code_and_no_invite_are_the_same_answer() {
    let mut r = roster();
    r.create_invite("+355690000003", Role::Courier, "C", "RIGHTRIGHTRIGHT1", NOW, WEEK)
        .unwrap();
    assert_eq!(
        r.claim_invite("+355690000003", "WRONGWRONGWRONG1", "pw", NOW),
        Err(ClaimError::NoSuchInvite)
    );
    assert_eq!(
        r.claim_invite("+355699999999", "RIGHTRIGHTRIGHT1", "pw", NOW),
        Err(ClaimError::NoSuchInvite),
        "a phone nobody invited must not answer differently"
    );
}

#[test]
fn an_expired_code_says_so_rather_than_failing_silently() {
    let mut r = roster();
    r.create_invite("+355690000004", Role::Courier, "C", "EXPIREDEXPIRED12", NOW, WEEK)
        .unwrap();
    assert_eq!(
        r.claim_invite("+355690000004", "EXPIREDEXPIRED12", "pw", NOW + WEEK),
        Err(ClaimError::Expired)
    );
    assert!(r.person("+355690000004").is_none());
    // Still listed, so the owner can see WHY the courier is stuck.
    assert_eq!(r.invites().len(), 1);
}

/// Inviting the same phone twice must not leave two live codes for one
/// person -- the first one would keep working after the owner believed they
/// had replaced it.
#[test]
fn a_second_invite_replaces_the_first() {
    let mut r = roster();
    r.create_invite("+355690000005", Role::Courier, "C", "FIRSTFIRSTFIRST1", NOW, WEEK)
        .unwrap();
    r.create_invite("+355690000005", Role::Courier, "C", "SECONDSECONDSEC1", NOW, WEEK)
        .unwrap();
    assert_eq!(r.invites().len(), 1);
    assert_eq!(
        r.claim_invite("+355690000005", "FIRSTFIRSTFIRST1", "pw", NOW),
        Err(ClaimError::NoSuchInvite),
        "the replaced code still worked"
    );
    assert!(r.claim_invite("+355690000005", "SECONDSECONDSEC1", "pw", NOW).is_ok());
}

#[test]
fn an_invite_cannot_overwrite_an_existing_account() {
    let mut r = roster();
    r.upsert_person("+355690000006", Role::Courier, "C", "real-password").unwrap();
    r.create_invite("+355690000006", Role::Courier, "C", "TAKEOVERTAKEOVER", NOW, WEEK)
        .unwrap();
    assert_eq!(
        r.claim_invite("+355690000006", "TAKEOVERTAKEOVER", "stolen", NOW),
        Err(ClaimError::AlreadyClaimed)
    );
    assert!(r.authenticate("+355690000006", "real-password").is_some());
}

/// The code is a credential and is stored the way credentials are stored.
/// A roster image that leaked would otherwise hand over every pending
/// account.
#[test]
fn the_code_is_not_in_the_image() {
    let mut r = roster();
    r.create_invite("+355690000007", Role::Courier, "C", "PLAINTEXTSECRET1", NOW, WEEK)
        .unwrap();
    let bytes = r.to_bytes().unwrap();
    let hay = String::from_utf8_lossy(&bytes);
    assert!(!hay.contains("PLAINTEXTSECRET1"), "the code is readable in the roster image");
}

#[test]
fn invites_survive_a_round_trip() {
    let mut r = roster();
    r.create_invite("+355690000008", Role::Courier, "Ana", "ROUNDTRIPROUND12", NOW, WEEK)
        .unwrap();
    let bytes = r.to_bytes().unwrap();
    let mut back = Roster::load(&bytes).unwrap();
    back.set_iterations(64);
    assert_eq!(back.invites().len(), 1);
    assert_eq!(back.invites()[0].name, "Ana");
    assert!(back.claim_invite("+355690000008", "ROUNDTRIPROUND12", "pw", NOW).is_ok());
}
