//! INVITES: the code a new courier or cook is handed, how it is drawn, and how
//! it is claimed. The code is stored hashed, like the password it stands in for.

use super::*;

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

/// Render 16 random bytes as an invite code.
///
/// THE RANDOMNESS IS THE CALLER'S, and that is the seam. `random_bytes` reads
/// `/dev/urandom`, which exists on the hub's own machine and does not exist in
/// a Worker -- where the same call fails and an owner is told "no randomness
/// available" while trying to hire somebody. The alphabet and the length are
/// the part that must not differ between platforms, so they live here and each
/// platform supplies its own bytes.
///
/// Fewer than 16 bytes is refused rather than padded: a shorter code is a
/// weaker code, and silently producing one would be the kind of downgrade
/// nobody notices.
pub fn invite_code_from(bytes: &[u8]) -> Option<String> {
    if bytes.len() < 16 {
        return None;
    }
    Some(bytes[..16].iter().map(|b| CODE_ALPHABET[(b & 31) as usize] as char).collect())
}

/// A 16-character invite code, from this machine's CSPRNG. 32^16 is 2^80, which
/// is not a number anybody guesses against a hub that answers one request at a
/// time.
pub fn new_invite_code() -> Result<String, HubError> {
    let bytes = random_bytes(16).map_err(|_| HubError::NotAHub)?;
    invite_code_from(&bytes).ok_or(HubError::NotAHub)
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

impl Roster {
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
}

#[cfg(test)]
mod tests;
