//! THE SIGN-IN MEMO (W-AUTHC, operator 2026-10-11 "оптимізуй та здешеви максимально").
//!
//! WHAT IT CUTS, MEASURED (`storefront/routes/costs_tests.rs`): after W-BN4B one delivery
//! order cost 40 object requests and 21 of them were the PLATFORM object's sign-in reads --
//! a courier's or a cook's every step read two whole images (`sessions` and `couriers`, or
//! `identity` and `sessions`) from `__platform`, to re-derive a verdict that had not changed
//! since the step before. A courier on shift polls its tasks every 12 s: ten platform
//! requests a minute, per courier, to be told "still you".
//!
//! WHAT IT IS. A per-isolate map from a token's sha256 to the principal the guard built for
//! it, for COURIER and STAFF tokens only, kept for at most `TTL_MS` and never past the session
//! row's own expiry. The token is still verified (HMAC + `exp`) on every request before the
//! memo is asked; the memo only replaces the two LIVE-STATE reads.
//!
//! THE REVOCATION GUARANTEE, stated as a number and tested
//! (`a_revoked_courier_is_refused_within_the_window`):
//!   * A revocation written THROUGH THIS ISOLATE (`platform_store::with`, every writer of the
//!     platform's images) empties the memo at once: refused on the very next request.
//!   * A revocation written by ANOTHER isolate is honoured within `TTL_MS` (60 s) of the
//!     moment this isolate last read the live rows -- the documented window. A session's own
//!     expiry is never extended: an entry dies at `min(seen + TTL, session expires_at)`.
//!   * OWNERS ARE NEVER MEMOISED. "An owner removed a moment ago is refused" (W-CHEAP) stays
//!     exact, because no generation check exists that could give the same guarantee for free.
//!   * Customer tokens never read the platform, and API keys keep their own path.
//!
//! RACE: a guard that read the rows BEFORE a write and stores AFTER it would put a stale
//! verdict back. The epoch closes that: `begin()` is taken before the reads, every platform
//! write bumps it, and `store` drops an entry whose reads started in an older epoch.
//!
//! BOUNDED: at most `CAP` entries; a full memo is emptied rather than grown (an isolate's
//! working set is a few dozen tokens; emptying costs one read per token, never correctness).

use super::Principal;
use std::cell::RefCell;
use std::collections::HashMap;

/// The documented revocation window for a courier or staff session revoked elsewhere.
pub const TTL_MS: i64 = 60_000;
/// The most tokens one isolate remembers.
pub const CAP: usize = 512;

struct Entry {
    principal: Principal,
    seen_ms: i64,
    until_ms: i64,
}

/// The memo itself, PURE: the clock is an argument, so the window is tested at its edges.
#[derive(Default)]
pub struct Memo {
    map: HashMap<String, Entry>,
    epoch: u64,
}

impl Memo {
    /// The principal remembered for this token, if it is still inside its window.
    pub fn get(&mut self, key: &str, now_ms: i64) -> Option<Principal> {
        let e = self.map.get(key)?;
        // A clock that runs BACKWARDS (another Worker's instant) is a miss, never a hit:
        // "within the window" is only defined forwards from the read.
        let fresh = now_ms >= e.seen_ms && now_ms - e.seen_ms < TTL_MS && now_ms < e.until_ms;
        if fresh {
            return Some(e.principal.clone());
        }
        self.map.remove(key);
        None
    }

    /// The epoch a guard's live reads start in.
    pub fn begin(&self) -> u64 {
        self.epoch
    }

    /// Remember a verdict built from reads that started in `epoch`. Dropped if any platform
    /// write landed since (the reads may predate a revocation), or if it is not memoisable.
    pub fn store(&mut self, key: String, p: &Principal, epoch: u64, seen_ms: i64, until_ms: i64) {
        if epoch != self.epoch || !memoisable(p) || until_ms <= seen_ms {
            return;
        }
        if self.map.len() >= CAP {
            self.map.clear();
        }
        self.map.insert(key, Entry { principal: p.clone(), seen_ms, until_ms });
    }

    /// A platform image was written: everything remembered may be stale.
    pub fn forget_all(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
        self.map.clear();
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.map.len()
    }
}

/// Only a session-bound role is remembered. An owner's authority is re-derived every time.
pub fn memoisable(p: &Principal) -> bool {
    matches!(p, Principal::Courier { .. } | Principal::Staff { .. })
}

thread_local! {
    /// The isolate's memo. In wasm an isolate is one thread; in `cargo test` every test runs on
    /// its own thread, so one test's memo never answers another's.
    static MEMO: RefCell<Memo> = RefCell::new(Memo::default());
}

/// The memo key: a digest, so the isolate never holds a usable token in a map.
pub fn key(token: &str) -> String {
    super::sha256_hex(token)
}

pub fn get(key: &str, now_ms: i64) -> Option<Principal> {
    MEMO.with(|m| m.borrow_mut().get(key, now_ms))
}

pub fn begin() -> u64 {
    MEMO.with(|m| m.borrow().begin())
}

pub fn store(key: String, p: &Principal, epoch: u64, seen_ms: i64, until_ms: i64) {
    MEMO.with(|m| m.borrow_mut().store(key, p, epoch, seen_ms, until_ms));
}

/// Called by every writer of a platform image (`platform_store::with`).
pub fn forget_all() {
    MEMO.with(|m| m.borrow_mut().forget_all());
}

#[cfg(test)]
pub fn len() -> usize {
    MEMO.with(|m| m.borrow().len())
}

#[cfg(test)]
mod tests;
