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
/// A pending invite. A separate namespace from `person:` so an invite can never
/// be mistaken for an account -- the difference is exactly "can this id log in".
const P_INVITE: &str = "invite:";
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

/// The alphabet an invite code is drawn from: exactly 32 characters, being the
/// uppercase letters without `I` and `O`, and the digits without `0` and `1`.
///
/// The code is READ OFF A SCREEN AND TYPED ON A PHONE, usually by somebody
/// standing in a kitchen doorway. Each confusable pair it loses -- zero for
/// oh, one for eye -- is a courier who cannot start their shift and an owner
/// who has to issue a second code. Dropping the DIGITS is what lets `L` stay:
/// `L` is only ambiguous against a `1` that is no longer in the set.
///
/// Thirty-two is not cosmetic. It divides 256 evenly, so `b & 31` draws each
/// character with equal probability; an alphabet of 31 or 33 would make the
/// low characters likelier and quietly cost the code some of its entropy.
const CODE_ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

/// A 16-character invite code. 32^16 is 2^80, which is not a number anybody
/// guesses against a hub that answers one request at a time.
pub fn new_invite_code() -> Result<String, HubError> {
    let bytes = random_bytes(16).map_err(|_| HubError::NotAHub)?;
    Ok(bytes.iter().map(|b| CODE_ALPHABET[(b & 31) as usize] as char).collect())
}

/// A pending invite, as the owner sees it. No hash, no code -- there is nothing
/// here that could be replayed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invite {
    pub id: String,
    pub role: Role,
    pub name: String,
    pub made_ms: i64,
    pub until_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimError {
    /// No invite, or the wrong code. ONE variant on purpose: telling the two
    /// apart would say which phone numbers have been invited.
    NoSuchInvite,
    Expired,
    AlreadyClaimed,
}

impl ClaimError {
    pub fn as_str(self) -> &'static str {
        match self {
            ClaimError::NoSuchInvite => "that code does not match",
            ClaimError::Expired => "that code has expired -- ask for a new one",
            ClaimError::AlreadyClaimed => "this person already has an account",
        }
    }
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

    /// THE IMAGE IS REWRITTEN WHOLE, not appended to.
    ///
    /// The store is append-only: every commit allocates a new generation and
    /// the old one is never reclaimed. For a KV that rewrites the same small
    /// map over and over, the arena is spent by the NUMBER OF WRITES rather
    /// than by the data -- measured at 313 empty commits before a fresh roster
    /// refused, while five hundred sessions in ONE commit fitted easily. A hub
    /// would therefore stop accepting logins after a few hundred of them.
    ///
    /// `compacted_bytes` commits the entries into a fresh image, so the file is
    /// as large as its content rather than as large as its history. See
    /// `Kv::compacted_bytes` for what that gives up (nothing anything here
    /// reads).
    pub fn to_bytes(&mut self) -> Result<Vec<u8>, HubError> {
        Ok(self.kv.compacted_bytes(DEFAULT_ROSTER_BYTES)?)
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

    // ── invites ─────────────────────────────────────────────────────────────

    /// Invite somebody who has no password yet.
    ///
    /// THE CODE IS STORED HASHED, exactly like a password, because that is what
    /// it is: for as long as it stands, whoever holds it can become this
    /// courier. Keeping it in the clear would mean anyone who could read the
    /// roster image -- a backup, a copied hub directory -- could claim the
    /// account. The owner sees it once, at the moment they create it, and the
    /// hub cannot show it again.
    ///
    /// The invite is keyed by the id the courier will log in with, so inviting
    /// the same phone twice REPLACES the pending invite rather than leaving two
    /// codes alive for one person.
    pub fn create_invite(
        &mut self,
        id: &str,
        role: Role,
        name: &str,
        code: &str,
        now_ms: i64,
        ttl_ms: i64,
    ) -> Result<(), HubError> {
        let salt = random_bytes(SALT_LEN).map_err(|_| HubError::NotAHub)?;
        let mut hash = [0u8; HASH_LEN];
        pbkdf2_sha256(code.as_bytes(), &salt, self.iterations, &mut hash);
        let rec = format!(
            r#"{{"id":"{}","role":"{}","name":"{}","salt":"{}","hash":"{}","it":{},"made":{},"until":{}}}"#,
            esc(id),
            role.as_str(),
            esc(name),
            hex(&salt),
            hex(&hash),
            self.iterations,
            now_ms,
            now_ms + ttl_ms
        );
        self.kv.put(&format!("{P_INVITE}{id}"), rec.as_bytes());
        Ok(())
    }

    /// Pending invites, WITHOUT their hashes. An expired one is still listed:
    /// the owner needs to see that the code they sent has run out, which is the
    /// answer to "they say it does not work".
    pub fn invites(&self) -> Vec<Invite> {
        self.kv
            .keys()
            .into_iter()
            .filter(|k| k.starts_with(P_INVITE))
            .filter_map(|k| {
                let rec = String::from_utf8(self.kv.get(&k)?).ok()?;
                Some(Invite {
                    id: str_field(&rec, "id")?,
                    role: Role::from_str(&str_field(&rec, "role")?)?,
                    name: str_field(&rec, "name").unwrap_or_default(),
                    made_ms: int_field(&rec, "made").unwrap_or(0),
                    until_ms: int_field(&rec, "until").unwrap_or(0),
                })
            })
            .collect()
    }

    pub fn revoke_invite(&mut self, id: &str) -> bool {
        self.kv.remove(&format!("{P_INVITE}{id}"))
    }

    /// Turn an invite into a person, with the password THEY choose.
    ///
    /// One shot: the invite is removed whether or not the hub crashes a
    /// millisecond later, because the person is written first and the invite
    /// deleted in the same commit. A code that survived its own use would be a
    /// second key to somebody else's account.
    ///
    /// Spends the same work on a miss as `authenticate`, and for the same
    /// reason: otherwise "no invite for this phone" answers instantly and
    /// "wrong code" answers slowly, and an attacker learns which phones have
    /// been invited without guessing a single code.
    pub fn claim_invite(
        &mut self,
        id: &str,
        code: &str,
        password: &str,
        now_ms: i64,
    ) -> Result<Person, ClaimError> {
        const DUMMY_SALT: &[u8] = b"a fixed dummy salt";
        let rec = self
            .kv
            .get(&format!("{P_INVITE}{id}"))
            .and_then(|v| String::from_utf8(v).ok());

        let Some((salt, expected, iterations, role, name, until)) = rec.as_deref().and_then(|r| {
            Some((
                unhex(&str_field(r, "salt")?)?,
                unhex(&str_field(r, "hash")?)?,
                int_field(r, "it")? as u32,
                Role::from_str(&str_field(r, "role")?)?,
                str_field(r, "name").unwrap_or_default(),
                int_field(r, "until").unwrap_or(0),
            ))
        }) else {
            let mut sink = [0u8; HASH_LEN];
            pbkdf2_sha256(code.as_bytes(), DUMMY_SALT, self.iterations, &mut sink);
            return Err(ClaimError::NoSuchInvite);
        };

        let mut got = [0u8; HASH_LEN];
        pbkdf2_sha256(code.as_bytes(), &salt, iterations.max(1), &mut got);
        if !constant_time_eq(&got, &expected) {
            return Err(ClaimError::NoSuchInvite);
        }
        // Checked AFTER the hash, so an expired invite and a wrong code take
        // the same time. Told apart in the ANSWER, because a courier whose code
        // expired needs a new one rather than another attempt.
        if now_ms >= until {
            return Err(ClaimError::Expired);
        }
        if self.person(id).is_some() {
            return Err(ClaimError::AlreadyClaimed);
        }
        self.upsert_person(id, role, &name, password).map_err(|_| ClaimError::NoSuchInvite)?;
        self.revoke_invite(id);
        self.person(id).ok_or(ClaimError::NoSuchInvite)
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
        // ── SESSIONS ARE SWEPT ON THE WAY IN ──
        //
        // Every login wrote a session and nothing ever removed one. On a live
        // stand the roster arena filled and LOGIN ITSELF started answering
        // `ArenaFull { need: 67700, capacity: 64512 }` -- a 503 that locks
        // every person out of the hub and says nothing anyone could act on.
        // At one restaurant that is years away and it still arrives, and it
        // arrives at the worst possible moment: nobody can get in to fix it.
        //
        // A session outlives its refresh token by definition -- once the
        // refresh has expired the session can never mint anything again -- so
        // sweeping past that point removes records that are already dead
        // rather than logging anybody out. Done HERE because a login is the
        // one moment we are already holding the write lock and rewriting the
        // roster anyway.
        self.sweep_sessions(now_ms);
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
    /// How long a session record is kept after it is issued.
    ///
    /// The refresh token's own lifetime plus a day. The day is not politeness:
    /// clocks disagree, and a record removed a minute before its token expires
    /// logs somebody out mid-shift for no reason anybody could explain.
    pub const SESSION_KEEP_MS: i64 = crate::token::REFRESH_TTL_MS + 24 * 60 * 60 * 1000;

    /// Drop sessions that can no longer mint anything. Returns how many went.
    pub fn sweep_sessions(&mut self, now_ms: i64) -> usize {
        let dead: Vec<String> = self
            .kv
            .keys()
            .into_iter()
            .filter(|k| k.starts_with(P_SESSION))
            .filter(|k| {
                let Some(v) = self.kv.get(k) else { return false };
                let Ok(rec) = String::from_utf8(v) else { return true };
                // A record we cannot read is swept too: it can never
                // authenticate anybody, so keeping it only costs arena.
                let issued = int_field(&rec, "issued").unwrap_or(0);
                let revoked = int_field(&rec, "revoked").unwrap_or(0) == 1;
                revoked || now_ms.saturating_sub(issued) > Self::SESSION_KEEP_MS
            })
            .collect();
        for k in &dead {
            self.kv.remove(k);
        }
        dead.len()
    }

    /// How many session records the roster is holding. Exposed so the arena
    /// pressure this caused is measurable rather than inferred.
    pub fn live_session_count(&self) -> usize {
        self.kv.keys().into_iter().filter(|k| k.starts_with(P_SESSION)).count()
    }

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

#[cfg(test)]
mod invite_tests {
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
}

#[cfg(test)]
mod sweep_tests {
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
}
