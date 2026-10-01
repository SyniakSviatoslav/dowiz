//! SESSIONS: opened at login, labelled when they are long-lived keys, swept when
//! they expire, revoked one at a time or all at once for a person who left.

use super::*;

impl Roster {
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

    /// How many session records the roster is holding, so the tests can
    /// measure the arena pressure this caused rather than infer it.
    #[cfg(test)]
    fn live_session_count(&self) -> usize {
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
}

#[cfg(test)]
mod tests;
