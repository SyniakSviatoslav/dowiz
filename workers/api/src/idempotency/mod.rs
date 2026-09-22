//! `Idempotency-Key`: the retry that must not become a second order.
//!
//! THE DEFECT THIS CLOSES. `place()` mints the order id from the platform
//! CSPRNG, per attempt. A customer on a weak connection whose response is lost,
//! and whose app retries, gets a SECOND ORDER: a second stock reservation, a
//! second kitchen ticket, a second Stripe intent keyed on the new id. Stripe's
//! own idempotency key protects the CHARGE and cannot protect the ORDER,
//! because it is derived from the thing that is duplicated. The venue does not
//! lose money — it loses a portion of food and a courier's trip, which is worse
//! per incident than the charge would have been.
//!
//! FIVE RULES, and four of them are the half people leave out:
//!
//!   1. **The stored value is the full response**, status and body — not a
//!      "seen" marker. A no-op that returns 204 where the first call returned
//!      an order and a customer token has not made the client whole.
//!   2. **The key is scoped to `(venue, principal, route, key)`.** A key from
//!      one venue can never replay into another; anything else is a tenancy
//!      defect waiting to be written.
//!   3. **A replay with a DIFFERENT body under the same key is 409**, loudly.
//!      This is Stripe's own rule and it is the half that catches client bugs:
//!      a POS reusing a key across two baskets is broken, and silence hides it.
//!   4. **A retry that arrives while the first call is still running is 409
//!      with `Retry-After`**, never a second execution. Without this the
//!      middleware only narrows the window it was built to close.
//!   5. **It fails OPEN on its own errors.** If the record cannot be read or
//!      written, the call proceeds. An idempotency layer that can refuse an
//!      order has inverted its purpose.
//!
//! WHERE IT LIVES. One `Table` image per venue, swept by the nightly job. It is
//! the venue's object, which is rule 2 arriving as a property of where the
//! bytes are rather than as a check somebody has to write.

use worker::*;

use crate::hubstore::Place;

/// The image, and the record kinds inside it.
pub const IMAGE_IDEMPOTENCY: &str = "idem";
pub const IDEMPOTENCY_BYTES: usize = 4 * 1024 * 1024;
const KIND: &str = "k";

/// How long a key is honoured. Long enough for a phone to retry after a tunnel,
/// short enough that the image stays small; swept nightly.
pub const KEEP_MS: i64 = 24 * 60 * 60 * 1000;

/// How long a caller is asked to wait when the first call is still running.
const RETRY_AFTER_S: u32 = 2;

/// What the middleware decided.
pub enum Decision {
    /// No key was sent. The route runs and nothing is recorded.
    NoKey,
    /// This call owns the key. Run, then call `finish`.
    Proceed { key: String },
    /// The first call already answered. This IS that answer.
    Replay { status: u16, body: String },
    /// Same key, different body — or the first call is still running.
    Refuse(Response),
}

/// The key a caller sent, if any, and what it is scoped by.
///
/// PURE. It reads a header and hashes a body; no I/O, no clock, no store. That
/// is what makes the scoping rule testable rather than reviewable.
pub fn scope(key: &str, venue: &str, principal: &str, route: &str) -> String {
    format!("{venue}\u{1}{principal}\u{1}{route}\u{1}{key}")
}

/// A stable fingerprint of the request body, so "same key, different body" is a
/// question this can answer.
pub fn fingerprint(body: &str) -> String {
    crate::auth::sha256_hex(body)
}

/// Decide what to do with a call that may be a retry.
///
/// `route` is a constant chosen at the call site — never anything derived from
/// the request, which is what stops one route's key from replaying into another.
pub async fn begin(
    place: &Place,
    header: Option<String>,
    principal: &str,
    route: &'static str,
    body: &str,
    now_ms: i64,
) -> Decision {
    let Some(raw) = header.map(|h| h.trim().to_string()).filter(|h| !h.is_empty()) else {
        return Decision::NoKey;
    };
    if raw.len() > 200 {
        return Decision::Refuse(
            Response::error("Idempotency-Key is too long", 400).unwrap(),
        );
    }
    let key = scope(&raw, &place.venue, principal, route);
    let print = fingerprint(body);

    // ONE TURN decides between the three answers, because the object runs its
    // calls one after another: claiming the key and finding it claimed cannot
    // interleave, which is the whole reason rule 4 is enforceable at all.
    let (k, p) = (key.clone(), print.clone());
    let found = crate::hubstore::with_table(
        place,
        IMAGE_IDEMPOTENCY,
        IDEMPOTENCY_BYTES,
        move |t| {
            let existing = t
                .get(KIND, &k)
                .and_then(|j| serde_json::from_str::<serde_json::Value>(&j).ok());
            match existing {
                Some(r) => Ok(Some(r)),
                None => {
                    // Claimed, with no answer yet. A retry arriving now sees
                    // `done: false` and is told to wait rather than run.
                    let rec = serde_json::json!({
                        "print": p, "at_ms": now_ms, "done": false,
                        "status": 0, "body": "",
                    })
                    .to_string();
                    t.put(KIND, &k, &rec, &[], &[])
                        .map_err(|e| Error::RustError(format!("idempotency: {e}")))?;
                    Ok(None)
                }
            }
        },
    )
    .await;

    let existing = match found {
        Ok(v) => v,
        // RULE 5: fail open. The call proceeds unrecorded rather than being
        // refused by the layer that exists to protect it.
        Err(e) => {
            console_error!("idempotency unavailable for {route}: {e}");
            return Decision::NoKey;
        }
    };

    let Some(r) = existing else {
        return Decision::Proceed { key };
    };

    let s = |k: &str| r.get(k).and_then(serde_json::Value::as_str).unwrap_or("").to_string();
    if s("print") != print {
        return Decision::Refuse(
            Response::error(
                "this Idempotency-Key was used with a different request body",
                409,
            )
            .unwrap(),
        );
    }
    if r.get("done").and_then(serde_json::Value::as_bool) != Some(true) {
        // Still running. A stale claim is treated as abandoned after the
        // keep window, so a Worker cut off mid-call does not wedge a key for
        // a day.
        let at = r.get("at_ms").and_then(serde_json::Value::as_i64).unwrap_or(0);
        if now_ms - at < KEEP_MS {
            let mut res = Response::error("a request with this key is still running", 409)
                .unwrap();
            let _ = res.headers_mut().set("retry-after", &RETRY_AFTER_S.to_string());
            return Decision::Refuse(res);
        }
        return Decision::Proceed { key };
    }
    Decision::Replay {
        status: r.get("status").and_then(serde_json::Value::as_i64).unwrap_or(200) as u16,
        body: s("body"),
    }
}

/// The whole middleware, as one call, for a handler that only wants the answer.
///
/// WHY A HELPER AND NOT A FOURTH COPY. `begin` returns four cases and every
/// caller has to spell all four: `NoKey`, `Proceed`, `Refuse`, and a `Replay`
/// that must rebuild the stored response with its `idempotent-replay` header.
/// That is twenty lines of exactly-the-same, and the shape of it — a replay
/// answering 204 instead of the first call's body, or forgetting the header —
/// is precisely rule 1's defect. `storefront::place` was the only caller for
/// long enough that the boilerplate was invisible; it is four callers now.
///
/// `Err(Response)` is EVERYTHING THE HANDLER MUST RETURN AT ONCE: a refusal, or
/// a replay carrying the first call's whole answer. `Ok(Guard)` means proceed.
pub struct Guard {
    /// `None` when the client sent no key. Nothing is recorded in that case,
    /// which is the same behaviour this layer had before it existed.
    key: Option<String>,
    print: String,
    at_ms: i64,
}

pub async fn guard(
    place: &Place,
    header: Option<String>,
    principal: &str,
    route: &'static str,
    body: &str,
    now_ms: i64,
) -> std::result::Result<Guard, Response> {
    let print = fingerprint(body);
    match begin(place, header, principal, route, body, now_ms).await {
        Decision::NoKey => Ok(Guard { key: None, print, at_ms: now_ms }),
        Decision::Proceed { key } => Ok(Guard { key: Some(key), print, at_ms: now_ms }),
        Decision::Refuse(r) => Err(r),
        Decision::Replay { status, body } => {
            // THE FIRST CALL'S WHOLE ANSWER, not a marker. On a courier's
            // `deliver` it carries the cash shortfall; a 204 here would lose it
            // on exactly the retry this exists to serve.
            let mut res = Response::ok(body)
                .unwrap_or_else(|_| Response::empty().unwrap())
                .with_status(status);
            let _ = res.headers_mut().set("content-type", "application/json");
            let _ = res.headers_mut().set("idempotent-replay", "true");
            Err(res)
        }
    }
}

impl Guard {
    /// Record what this call answered. Does nothing when there was no key.
    pub async fn done(self, place: &Place, status: u16, body: &str) {
        if let Some(key) = self.key {
            finish(place, &key, status, body, &self.print, self.at_ms).await;
        }
    }
}

/// Record what the first call answered, so a retry can be given the same thing.
///
/// NEVER FAILS THE CALLER. The answer is already correct; losing the record
/// means a retry runs again, which is the behaviour without this module at all.
pub async fn finish(place: &Place, key: &str, status: u16, body: &str, print: &str, now_ms: i64) {
    let (k, b, p) = (key.to_string(), body.to_string(), print.to_string());
    let _ = crate::hubstore::with_table(
        place,
        IMAGE_IDEMPOTENCY,
        IDEMPOTENCY_BYTES,
        move |t| {
            let rec = serde_json::json!({
                "print": p, "at_ms": now_ms, "done": true,
                "status": status, "body": b,
            })
            .to_string();
            t.put(KIND, &k, &rec, &[], &[])
                .map_err(|e| Error::RustError(format!("idempotency: {e}")))
        },
    )
    .await;
}

/// Drop keys past the window. Called by the nightly job, per venue.
pub async fn sweep(place: &Place, now_ms: i64) -> Result<usize> {
    crate::hubstore::with_table(place, IMAGE_IDEMPOTENCY, IDEMPOTENCY_BYTES, move |t| {
        let old: Vec<String> = t
            .all(KIND)
            .into_iter()
            .filter(|(_, j)| {
                serde_json::from_str::<serde_json::Value>(j)
                    .ok()
                    .and_then(|v| v.get("at_ms").and_then(serde_json::Value::as_i64))
                    .map_or(true, |at| now_ms - at > KEEP_MS)
            })
            .map(|(id, _)| id)
            .collect();
        let n = old.len();
        for id in old {
            t.remove(KIND, &id);
        }
        Ok(n)
    })
    .await
}

#[cfg(test)]
mod tests;
