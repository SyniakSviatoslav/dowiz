//! G1 / D1: EVERY ANSWER AFTER A CLAIM IS RECORDED, OR THE CLAIM IS GIVEN BACK.
//!
//! THE DEFECT. `begin` claims the key (`done: false`) before the handler runs.
//! Only the success path called `Guard::done`; a handler that refused -- "the
//! payment sum exceeds the total", "that order moved on" -- returned without a
//! word to this layer, and every retry with the same key was answered
//! "409 still running" (with `Retry-After`) for the whole 24-hour keep window.
//! The room's and the courier's outbox (`public/lib/outbox.js`) read that 409
//! as "wait" and spend no try on it, so ONE refused tap wedged every tap queued
//! behind it.
//!
//! THE RULE, as three pure table operations `begin` and `Guard` are made of:
//!   * `claim`   -- a fresh key is claimed; a claimed-and-answered key replays
//!     its answer; a claim nobody finished inside `LEASE_MS` is abandoned and
//!     re-claimed (a Worker cut off mid-call cannot hold a key for a day).
//!   * `record`  -- any answer below 500 IS the answer, refusals included, and
//!     a retry is given the same refusal. `keeps(status)` is the line.
//!   * `release` -- a 5xx, or an internal `Err`, gives the claim back: a
//!     failure replayed would make a transient fault permanent.

use dowiz_hub::table::Table;
use serde_json::{json, Value};
use worker::*;

use super::{Guard, IDEMPOTENCY_BYTES, IMAGE_IDEMPOTENCY, KIND};
use crate::hubstore::Place;

/// How long an unfinished claim is honoured as "still running". A Worker
/// request that has not answered in a minute has been cut off; before this the
/// claim was honoured for `KEEP_MS`, a day.
pub const LEASE_MS: i64 = 60 * 1000;

/// What a key's record says about the call now arriving with it.
#[derive(Debug, PartialEq)]
pub enum Seen {
    /// Nobody had it (or the last claim was abandoned); this call now owns it.
    Claimed,
    /// The first call answered; this is its answer.
    Answered { status: u16, body: String, ctype: String },
    /// Same key, different body (rule 3).
    Mismatch,
    /// The first call is inside its lease and has not answered (rule 4).
    Running,
}

/// Does an answer with this status become the key's permanent answer?
/// Everything the server DECIDED (2xx, 3xx, 4xx) is; a 5xx is not.
pub fn keeps(status: u16) -> bool {
    status < 500
}

fn claim_rec(print: &str, now_ms: i64) -> String {
    json!({ "print": print, "at_ms": now_ms, "done": false, "status": 0, "body": "" }).to_string()
}

fn put(t: &mut Table, key: &str, rec: &str) -> Result<()> {
    t.put(KIND, key, rec, &[], &[]).map_err(|e| Error::RustError(format!("idempotency: {e}")))
}

/// ONE TURN of the object: read the key's record and, when this call may run,
/// claim it. Claiming and finding-claimed cannot interleave because the object
/// runs its turns one after another.
pub fn claim(t: &mut Table, key: &str, print: &str, now_ms: i64) -> Result<Seen> {
    let found = t.get(KIND, key).and_then(|j| serde_json::from_str::<Value>(&j).ok());
    let Some(r) = found else {
        put(t, key, &claim_rec(print, now_ms))?;
        return Ok(Seen::Claimed);
    };
    let s = |k: &str| r.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    if s("print") != print {
        return Ok(Seen::Mismatch);
    }
    if r.get("done").and_then(Value::as_bool) == Some(true) {
        let status = r.get("status").and_then(Value::as_i64).unwrap_or(200) as u16;
        let ctype = Some(s("type")).filter(|c| !c.is_empty()).unwrap_or_else(|| "application/json".into());
        return Ok(Seen::Answered { status, body: s("body"), ctype });
    }
    let at = r.get("at_ms").and_then(Value::as_i64).unwrap_or(0);
    if now_ms - at < LEASE_MS {
        return Ok(Seen::Running);
    }
    put(t, key, &claim_rec(print, now_ms))?;
    Ok(Seen::Claimed)
}

/// The key's permanent answer.
pub fn record(t: &mut Table, key: &str, print: &str, at_ms: i64, status: u16, body: &str, ctype: &str) -> Result<()> {
    let rec = json!({
        "print": print, "at_ms": at_ms, "done": true,
        "status": status, "body": body, "type": ctype,
    });
    put(t, key, &rec.to_string())
}

/// Give an unfinished claim back. An answered record is never removed here:
/// only the nightly sweep ends a recorded answer.
pub fn release(t: &mut Table, key: &str) {
    let open = t
        .get(KIND, key)
        .and_then(|j| serde_json::from_str::<Value>(&j).ok())
        .is_some_and(|r| r.get("done").and_then(Value::as_bool) != Some(true));
    if open {
        t.remove(KIND, key);
    }
}

/// A response's body as text, when it is held in memory (every JSON and
/// `Response::error` answer is). A stream cannot be recorded without being
/// consumed, so it gives the claim back instead.
fn body_text(res: &Response) -> Option<String> {
    match res.body() {
        ResponseBody::Body(b) => String::from_utf8(b.clone()).ok(),
        ResponseBody::Empty => Some(String::new()),
        ResponseBody::Stream(_) => None,
    }
}

impl Guard {
    async fn write(self, place: &Place, answer: Option<(u16, String, String)>) {
        let Some(key) = self.key else { return };
        let (print, at) = (self.print, self.at_ms);
        let _ = crate::hubstore::with_table(place, IMAGE_IDEMPOTENCY, IDEMPOTENCY_BYTES, move |t| {
            match &answer {
                Some((status, body, ctype)) if keeps(*status) => record(t, &key, &print, at, *status, body, ctype),
                _ => {
                    release(t, &key);
                    Ok(())
                }
            }
        })
        .await;
    }

    /// Give the claim back without an answer (the call failed inside).
    pub async fn release(self, place: &Place) {
        self.write(place, None).await;
    }

    /// Refuse, and make the refusal the key's answer: `Response::error(text, status)`
    /// now and on every retry. A 5xx releases the claim instead.
    pub async fn refused(self, place: &Place, status: u16, text: &str) -> Result<Response> {
        self.write(place, Some((status, text.to_string(), "text/plain;charset=UTF-8".into()))).await;
        Response::error(text, status)
    }

    /// Whatever the handler answered, recorded: the one exit a handler whose
    /// body is an `async { .. }` block needs. `Err` and 5xx release the claim.
    pub async fn answered(self, place: &Place, res: Result<Response>) -> Result<Response> {
        match res {
            Ok(r) => {
                let ctype = r.headers().get("content-type").ok().flatten().unwrap_or_default();
                let answer = body_text(&r).map(|b| (r.status_code(), b, ctype));
                self.write(place, answer).await;
                Ok(r)
            }
            Err(e) => {
                self.release(place).await;
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests;
