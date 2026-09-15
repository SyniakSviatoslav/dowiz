//! Who may act on this hub, in a bebop KV store.
//!
//! ONE VENUE, so this is small by construction: an owner, some staff, some
//! couriers. It is not a user directory and deliberately cannot become one —
//! there are no profiles, no preferences, no history, and no cross-hub
//! identity. A person exists here because they work at this restaurant.
//!
//! WHERE THIS SITS RELATIVE TO P67. The endgame is an anchor roster with signed
//! capability delegations, where a person proves who they are with a key rather
//! than a password. This is the bridge to it, not a replacement for it: the
//! front-ends already speak password login and bearer tokens, and the token
//! contract here is the one a delegation would later mint into — so P67 can
//! replace the LOGIN step without touching a single route.
//!
//! PASSWORDS ARE PBKDF2-HMAC-SHA256, per-person salted, with the iteration
//! count stored IN the record. Storing it is what lets the count be raised
//! later without invalidating every existing password: an old record verifies
//! at its own cost and is rewritten at the new one on next login.

use crate::crypto::{
    constant_time_eq, hex, pbkdf2_sha256, random_bytes, unhex, PBKDF2_ITERATIONS,
};
use crate::minijson::{esc, int_field, str_field};
use crate::token::Role;
use crate::HubError;
use bebop_store::kv::Kv;
use bebop_store::Store;

pub const DEFAULT_ROSTER_BYTES: usize = 512 * 1024;

const P_PERSON: &str = "person:";
const P_SESSION: &str = "session:";
const SALT_LEN: usize = 16;
const HASH_LEN: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Person {
    pub id: String,
    pub role: Role,
    pub name: String,
    /// A person who has left. Their record is kept — an order they touched
    /// still names them — but they can no longer log in.
    pub active: bool,
}

pub struct Roster {
    store: Store,
    kv: Kv,
    /// The cost new passwords are hashed at, AND the cost a failed lookup burns.
    ///
    /// One field for both, deliberately. The constant-work miss is only
    /// constant if it costs what a hit costs; if the two are configured
    /// separately they drift, and a miss that is *slower* than a hit is the same
    /// enumeration oracle running backwards. That is not hypothetical -- it is
    /// exactly what this crate did for one commit, where records hashed at a
    /// low test cost were probed against a dummy at the production one.
    iterations: u32,
}

impl Roster {
    pub fn create() -> Result<Self, HubError> {
        let mut store = Store::create_bytes(DEFAULT_ROSTER_BYTES);
        Kv::init_bytes(&mut store)?;
        let kv = Kv::load(&store).ok_or(HubError::NotAHub)?;
        Ok(Roster { store, kv, iterations: PBKDF2_ITERATIONS })
    }

    /// Lower the hashing cost. For TESTS ONLY -- a suite that spends half a
    /// second per password is a suite that stops being run.
    pub fn set_iterations(&mut self, n: u32) {
        self.iterations = n.max(1);
    }

    pub fn load(bytes: &[u8]) -> Result<Self, HubError> {
        let store = Store::from_bytes(bytes);
        if store.pick().is_none() {
            return Err(HubError::NotAHub);
        }
        let kv = Kv::load(&store).ok_or(HubError::NotAHub)?;
        Ok(Roster { store, kv, iterations: PBKDF2_ITERATIONS })
    }

    pub fn to_bytes(&mut self) -> Result<Vec<u8>, HubError> {
        self.kv.commit_into_bytes(&mut self.store)?;
        Ok(self.store.to_bytes())
    }

    // ── people ──────────────────────────────────────────────────────────────

    /// Add or replace a person, hashing their password at the roster's cost.
    pub fn upsert_person(
        &mut self,
        id: &str,
        role: Role,
        name: &str,
        password: &str,
    ) -> Result<(), HubError> {
        let iterations = self.iterations;
        let salt = random_bytes(SALT_LEN).map_err(|_| HubError::NotAHub)?;
        let mut hash = [0u8; HASH_LEN];
        pbkdf2_sha256(password.as_bytes(), &salt, iterations, &mut hash);
        let rec = format!(
            r#"{{"id":"{}","role":"{}","name":"{}","salt":"{}","hash":"{}","it":{},"active":1}}"#,
            esc(id),
            role.as_str(),
            esc(name),
            hex(&salt),
            hex(&hash),
            iterations
        );
        self.kv.put(&format!("{P_PERSON}{id}"), rec.as_bytes());
        Ok(())
    }

    fn record(&self, id: &str) -> Option<String> {
        self.kv
            .get(&format!("{P_PERSON}{id}"))
            .map(|v| String::from_utf8_lossy(&v).into_owned())
    }

    fn parse(rec: &str) -> Option<Person> {
        Some(Person {
            id: str_field(rec, "id")?,
            role: Role::from_str(&str_field(rec, "role")?)?,
            name: str_field(rec, "name").unwrap_or_default(),
            active: int_field(rec, "active").unwrap_or(0) == 1,
        })
    }

    pub fn person(&self, id: &str) -> Option<Person> {
        Self::parse(&self.record(id)?)
    }

    pub fn people(&self) -> Vec<Person> {
        self.kv
            .entries
            .iter()
            .filter(|(k, _)| k.starts_with(P_PERSON))
            .filter_map(|(_, v)| Self::parse(&String::from_utf8_lossy(v)))
            .collect()
    }

    pub fn couriers(&self) -> Vec<Person> {
        self.people().into_iter().filter(|p| p.role == Role::Courier).collect()
    }

    /// Mark a person active or not. Returns false if there is no such person.
    pub fn set_active(&mut self, id: &str, active: bool) -> bool {
        let Some(rec) = self.record(id) else { return false };
        let want = if active { 1 } else { 0 };
        let updated = match rec.find("\"active\":") {
            Some(at) => {
                let head = &rec[..at + "\"active\":".len()];
                let tail = &rec[at + "\"active\":".len() + 1..];
                format!("{head}{want}{tail}")
            }
            None => return false,
        };
        self.kv.put(&format!("{P_PERSON}{id}"), updated.as_bytes());
        true
    }

    /// Check a password.
    ///
    /// SPENDS THE SAME WORK ON A MISS as on a hit. Without that, "no such
    /// person" returns in microseconds and "wrong password" in half a second,
    /// which hands an attacker a free account-enumeration oracle — they learn
    /// which ids exist without ever guessing a password.
    ///
    /// An inactive person fails like a wrong password, for the same reason.
    pub fn authenticate(&self, id: &str, password: &str) -> Option<Person> {
        const DUMMY_SALT: &[u8] = b"a fixed dummy salt";
        let rec = self.record(id);

        let (salt, expected, iterations, person) = match rec.as_deref().and_then(|r| {
            Some((
                unhex(&str_field(r, "salt")?)?,
                unhex(&str_field(r, "hash")?)?,
                int_field(r, "it")? as u32,
                Self::parse(r)?,
            ))
        }) {
            Some(v) => v,
            None => {
                // Burn the same time, then fail. The iteration count used here
                // is the production one, so a missing person costs what a real
                // one does.
                let mut sink = [0u8; HASH_LEN];
                pbkdf2_sha256(password.as_bytes(), DUMMY_SALT, self.iterations, &mut sink);
                return None;
            }
        };

        let mut got = [0u8; HASH_LEN];
        pbkdf2_sha256(password.as_bytes(), &salt, iterations.max(1), &mut got);
        if constant_time_eq(&got, &expected) && person.active {
            Some(person)
        } else {
            None
        }
    }

    // ── sessions ────────────────────────────────────────────────────────────

    /// Open a session and return its id.
    ///
    /// The id comes from the OS CSPRNG. A session id derived from a counter or
    /// a clock would be guessable, and a guessed session id is a stolen
    /// session — the tokens carrying it are checked against this roster, not
    /// against the person.
    pub fn open_session(&mut self, person_id: &str, now_ms: i64) -> Result<String, HubError> {
        self.open_labelled_session(person_id, now_ms, "")
    }

    /// A session with a name on it.
    ///
    /// The label is what makes a long-lived API key manageable: an owner with
    /// three of them needs to know which is the laptop and which is the thing
    /// they set up in March, or they will never revoke any of them.
    pub fn open_labelled_session(
        &mut self,
        person_id: &str,
        now_ms: i64,
        label: &str,
    ) -> Result<String, HubError> {
        let id = hex(&random_bytes(16).map_err(|_| HubError::NotAHub)?);
        let rec = format!(
            r#"{{"person":"{}","issued":{},"revoked":0,"label":"{}"}}"#,
            esc(person_id),
            now_ms,
            esc(label)
        );
        self.kv.put(&format!("{P_SESSION}{id}"), rec.as_bytes());
        Ok(id)
    }

    /// Every live session a person holds: id, label, and when it was issued.
    ///
    /// The ID IS RETURNED, not the token. A session id names a credential well
    /// enough to revoke it and is useless for authenticating, which is exactly
    /// the split a "manage your keys" screen needs.
    pub fn sessions_of(&self, person_id: &str) -> Vec<(String, String, i64)> {
        self.kv
            .entries
            .iter()
            .filter(|(k, _)| k.starts_with(P_SESSION))
            .filter_map(|(k, v)| {
                let rec = String::from_utf8_lossy(v);
                if int_field(&rec, "revoked").unwrap_or(1) != 0 {
                    return None;
                }
                if str_field(&rec, "person").as_deref() != Some(person_id) {
                    return None;
                }
                Some((
                    k[P_SESSION.len()..].to_string(),
                    str_field(&rec, "label").unwrap_or_default(),
                    int_field(&rec, "issued").unwrap_or(0),
                ))
            })
            .collect()
    }

    /// Is this session still usable, and whose is it?
    ///
    /// Returns `None` for an unknown session as well as a revoked one. An
    /// unknown session id must not be treated as valid just because there is
    /// nothing recorded against it.
    pub fn session_owner(&self, session_id: &str) -> Option<String> {
        let raw = self.kv.get(&format!("{P_SESSION}{session_id}"))?;
        let rec = String::from_utf8_lossy(&raw);
        if int_field(&rec, "revoked").unwrap_or(1) != 0 {
            return None;
        }
        str_field(&rec, "person")
    }

    /// Revoke one session. A tombstone, like the subscription store's: the KV
    /// layout is rewritten whole and has no delete, and a revoked session must
    /// stay revoked rather than vanish and read as unknown-but-harmless.
    pub fn revoke_session(&mut self, session_id: &str) {
        let key = format!("{P_SESSION}{session_id}");
        let person = self
            .kv
            .get(&key)
            .and_then(|v| str_field(&String::from_utf8_lossy(&v), "person"))
            .unwrap_or_default();
        let rec = format!(r#"{{"person":"{}","issued":0,"revoked":1}}"#, esc(&person));
        self.kv.put(&key, rec.as_bytes());
    }

    /// Revoke every session a person holds — what "log out everywhere" means,
    /// and what must happen when someone leaves.
    pub fn revoke_all_for(&mut self, person_id: &str) -> usize {
        let ids: Vec<String> = self
            .kv
            .entries
            .iter()
            .filter(|(k, _)| k.starts_with(P_SESSION))
            .filter(|(_, v)| {
                str_field(&String::from_utf8_lossy(v), "person").as_deref() == Some(person_id)
            })
            .map(|(k, _)| k[P_SESSION.len()..].to_string())
            .collect();
        for id in &ids {
            self.revoke_session(id);
        }
        ids.len()
    }

    pub fn root(&self) -> String {
        self.kv.snapshot_root()
    }
}

#[cfg(test)]
mod tests {
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
}
