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

pub const DEFAULT_ROSTER_BYTES: usize = crate::CEILING_BYTES;

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

pub struct Roster {
    // `store: Store` WAS HERE and was never read: `Kv::load` COPIES the
    // entries out (`Kv { entries: Vec<(String, Vec<u8>)>}`), so the arena it
    // was read from is dead the moment the load returns, and the bytes written
    // back come from `kv.compacted_bytes_fit`. Holding it kept a whole
    // 512 KiB image alive for the lifetime of every one of these, inside
    // a Durable Object that holds one per venue.
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
        let mut store = Store::create_bytes(DEFAULT_ROSTER_BYTES)?;
        Kv::init_bytes(&mut store)?;
        let kv = Kv::load(&store).ok_or(HubError::NotAHub)?;
        Ok(Roster { kv, iterations: PBKDF2_ITERATIONS })
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
        Ok(Roster { kv, iterations: PBKDF2_ITERATIONS })
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
        Ok(self.kv.compacted_bytes_fit(DEFAULT_ROSTER_BYTES)?)
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

    pub fn root(&self) -> String {
        self.kv.snapshot_root()
    }
}

mod invite;
mod session;
pub use invite::{invite_code_from, new_invite_code, ClaimError, Invite};

#[cfg(test)]
mod tests;
