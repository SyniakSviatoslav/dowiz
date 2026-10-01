use super::*;

/// A cost low enough to keep the suite usable. Production uses
/// `PBKDF2_ITERATIONS`; this is why `set_iterations` exists.
const FAST: u32 = 64;

fn roster() -> Roster {
    let mut r = Roster::create().expect("create");
    r.set_iterations(FAST);
    r.upsert_person("owner_1", Role::Owner, "Arben", "correct horse").expect("owner");
    r.upsert_person("cour_1", Role::Courier, "Eni", "battery staple").expect("courier");
    r
}

#[test]
fn a_correct_password_authenticates_and_a_wrong_one_does_not() {
    let r = roster();
    let p = r.authenticate("owner_1", "correct horse").expect("should log in");
    assert_eq!(p.id, "owner_1");
    assert_eq!(p.role, Role::Owner);
    assert_eq!(p.name, "Arben");
    assert!(r.authenticate("owner_1", "correct horsE").is_none(), "one bit differs");
    assert!(r.authenticate("owner_1", "").is_none());
    assert!(r.authenticate("nobody", "correct horse").is_none());
}

/// Two people with the same password must not share a hash, or one leaked
/// record tells an attacker about every other account that reused it.
#[test]
fn identical_passwords_get_different_hashes() {
    let mut r = Roster::create().expect("create");
    r.set_iterations(FAST);
    r.upsert_person("a", Role::Courier, "A", "same").unwrap();
    r.upsert_person("b", Role::Courier, "B", "same").unwrap();
    let ha = str_field(&r.record("a").unwrap(), "hash").unwrap();
    let hb = str_field(&r.record("b").unwrap(), "hash").unwrap();
    assert_ne!(ha, hb, "the salt must make these differ");
    assert!(r.authenticate("a", "same").is_some());
    assert!(r.authenticate("b", "same").is_some());
}

/// The password must not be recoverable from the stored record.
#[test]
fn the_password_is_not_in_the_record() {
    let r = roster();
    let rec = r.record("owner_1").unwrap();
    assert!(!rec.contains("correct horse"), "the record holds the password: {rec}");
    assert!(rec.contains("\"salt\":"), "and it must hold a salt");
}

/// Someone who has left cannot log in, but their record survives so an
/// order that names them still resolves.
#[test]
fn an_inactive_person_cannot_log_in_but_is_not_erased() {
    let mut r = roster();
    assert!(r.set_active("cour_1", false));
    assert!(r.authenticate("cour_1", "battery staple").is_none());
    assert_eq!(r.person("cour_1").map(|p| p.name), Some("Eni".into()));
    assert!(r.set_active("cour_1", true));
    assert!(r.authenticate("cour_1", "battery staple").is_some(), "and they can come back");
    assert!(!r.set_active("ghost", true), "an unknown person cannot be activated");
}

#[test]
fn sessions_open_revoke_and_do_not_resurrect() {
    let mut r = roster();
    let s = r.open_session("owner_1", 1_700_000_000_000).expect("session");
    assert_eq!(r.session_owner(&s).as_deref(), Some("owner_1"));
    assert_eq!(r.session_owner("never issued"), None, "unknown is not valid");

    r.revoke_session(&s);
    assert_eq!(r.session_owner(&s), None, "a revoked session must stay dead");

    // And it stays dead across the byte image.
    let bytes = r.to_bytes().expect("bytes");
    assert_eq!(Roster::load(&bytes).expect("load").session_owner(&s), None);
}

/// A long-lived key must be nameable and listable, or it can never be
/// revoked with confidence.
#[test]
fn labelled_sessions_can_be_listed_and_revoked_individually() {
    let mut r = roster();
    let laptop = r.open_labelled_session("owner_1", 1_000, "laptop").unwrap();
    let mcp = r.open_labelled_session("owner_1", 2_000, "claude on my phone").unwrap();
    let other = r.open_labelled_session("cour_1", 3_000, "phone").unwrap();

    let mut mine = r.sessions_of("owner_1");
    mine.sort_by_key(|(_, _, at)| *at);
    assert_eq!(mine.len(), 2);
    assert_eq!(mine[0].1, "laptop");
    assert_eq!(mine[1].1, "claude on my phone");
    assert_eq!(mine[1].2, 2_000);
    // A session id is enough to revoke and useless to authenticate with.
    assert!(mine.iter().all(|(id, _, _)| id.len() == 32));

    r.revoke_session(&laptop);
    let left = r.sessions_of("owner_1");
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].0, mcp);
    // And another person's session is untouched and unlisted.
    assert_eq!(r.sessions_of("cour_1").len(), 1);
    assert_eq!(r.session_owner(&other).as_deref(), Some("cour_1"));
}

/// A label with a quote must not be able to rewrite the session's owner.
#[test]
fn a_hostile_label_cannot_move_a_session() {
    let mut r = roster();
    let s = r.open_labelled_session("cour_1", 1, r#"x","person":"owner_1"#).unwrap();
    assert_eq!(r.session_owner(&s).as_deref(), Some("cour_1"), "owner must not move");
}

#[test]
fn two_sessions_are_never_the_same_id() {
    let mut r = roster();
    let a = r.open_session("owner_1", 1).unwrap();
    let b = r.open_session("owner_1", 1).unwrap();
    assert_ne!(a, b, "session ids must not be guessable or repeated");
    assert_eq!(a.len(), 32, "16 random bytes as hex");
}

#[test]
fn logging_out_everywhere_revokes_every_session() {
    let mut r = roster();
    let a = r.open_session("owner_1", 1).unwrap();
    let b = r.open_session("owner_1", 2).unwrap();
    let other = r.open_session("cour_1", 3).unwrap();
    assert_eq!(r.revoke_all_for("owner_1"), 2);
    assert_eq!(r.session_owner(&a), None);
    assert_eq!(r.session_owner(&b), None);
    assert_eq!(r.session_owner(&other).as_deref(), Some("cour_1"), "and only theirs");
}

#[test]
fn the_roster_survives_the_byte_image() {
    let mut r = roster();
    let bytes = r.to_bytes().expect("bytes");
    let r = Roster::load(&bytes).expect("load");
    assert_eq!(r.people().len(), 2);
    assert_eq!(r.couriers().len(), 1);
    assert!(r.authenticate("cour_1", "battery staple").is_some());
}

/// The miss path must cost what the hit path costs. Measured, not asserted
/// in a comment: at a cost high enough to be timeable, looking up a person
/// who does not exist must take roughly as long as one who does. The bound
/// is loose because this runs on a shared machine -- it is here to catch a
/// miss that returns INSTANTLY (no work at all) or one that burns the
/// production count against a cheap record, which is the inverted oracle
/// this crate actually shipped for one commit.
#[test]
fn a_miss_costs_what_a_hit_costs() {
    use std::time::Instant;
    let mut r = Roster::create().expect("create");
    r.set_iterations(20_000);
    r.upsert_person("real", Role::Owner, "A", "pw").unwrap();

    let t0 = Instant::now();
    assert!(r.authenticate("real", "wrong").is_none());
    let hit = t0.elapsed();

    let t1 = Instant::now();
    assert!(r.authenticate("ghost", "wrong").is_none());
    let miss = t1.elapsed();

    let ratio = miss.as_secs_f64() / hit.as_secs_f64().max(1e-9);
    assert!(
        (0.2..5.0).contains(&ratio),
        "miss/hit timing ratio {ratio:.2} (hit {hit:?}, miss {miss:?}) -- \
         a miss must not be distinguishable by cost"
    );
}

/// A name with a quote in it must not be able to rewrite the role field.
#[test]
fn a_hostile_name_cannot_escalate_a_role() {
    let mut r = Roster::create().expect("create");
    r.set_iterations(FAST);
    r.upsert_person("x", Role::Courier, r#"Eni","role":"owner"#, "pw").unwrap();
    assert_eq!(r.person("x").map(|p| p.role), Some(Role::Courier), "role must not move");
}
