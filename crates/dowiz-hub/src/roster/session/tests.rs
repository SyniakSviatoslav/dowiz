use super::*;

const NOW: i64 = 1_789_000_000_000;

/// THE FAILURE THIS PREVENTS, measured on a live stand: every login wrote a
/// session, nothing ever removed one, and the roster arena filled. Login
/// then answered `ArenaFull { need: 67700, capacity: 64512 }` -- which locks
/// every person out of the hub, including whoever would fix it.
#[test]
fn a_thousand_logins_do_not_fill_the_roster() {
    let mut r = Roster::create().unwrap();
    r.set_iterations(64);
    r.upsert_person("ana@dubin.al", Role::Owner, "Ana", "pw").unwrap();

    let day = 24 * 60 * 60 * 1000;
    for i in 0..1000i64 {
        // A login a day for nearly three years.
        r.open_session("ana@dubin.al", NOW + i * day).expect("session");
    }
    // Only the ones that can still mint anything are kept.
    let live = r.live_session_count();
    assert!(live <= 32, "{live} sessions kept; the sweep is not working");
    // And the image still commits, which is the thing that actually broke.
    assert!(r.to_bytes().is_ok(), "the roster arena filled");
}

/// A sweep must never log anybody out. A session is only dropped once its
/// refresh token could no longer mint anything.
#[test]
fn a_live_session_survives_the_sweep() {
    let mut r = Roster::create().unwrap();
    r.set_iterations(64);
    r.upsert_person("ana@dubin.al", Role::Owner, "Ana", "pw").unwrap();
    let s = r.open_session("ana@dubin.al", NOW).unwrap();

    // A second login one day later must not touch the first session.
    r.open_session("ana@dubin.al", NOW + 24 * 60 * 60 * 1000).unwrap();
    assert_eq!(r.session_owner(&s).as_deref(), Some("ana@dubin.al"));

    // Nor one a day before the refresh expires.
    r.open_session("ana@dubin.al", NOW + crate::token::REFRESH_TTL_MS - 1).unwrap();
    assert_eq!(r.session_owner(&s).as_deref(), Some("ana@dubin.al"),
               "a session was dropped while its refresh token still worked");

    // Past the keep window it goes.
    r.open_session("ana@dubin.al", NOW + Roster::SESSION_KEEP_MS + 1).unwrap();
    assert_eq!(r.session_owner(&s), None);
}

/// A revoked session is dead the moment it is revoked, so it is swept at
/// the next opportunity rather than kept as a tombstone for ever.
#[test]
fn a_revoked_session_is_swept() {
    let mut r = Roster::create().unwrap();
    r.set_iterations(64);
    r.upsert_person("ana@dubin.al", Role::Owner, "Ana", "pw").unwrap();
    let s = r.open_session("ana@dubin.al", NOW).unwrap();
    r.revoke_session(&s);
    assert_eq!(r.session_owner(&s), None);
    let swept = r.sweep_sessions(NOW);
    assert_eq!(swept, 1);
    assert_eq!(r.live_session_count(), 0);
}
