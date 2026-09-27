//! THE COMMAND'S OWN TURN RECORDS THAT IT RAN (W-FIX O2, 2026-09-27).
//!
//! THE DEFECT (W-AUDIT O2). The key is claimed by the Worker, the command runs
//! in the object, and the answer is recorded by the Worker after the object
//! replied. A stub fetch that failed AFTER the object committed -- the reply
//! lost on the way back, the isolate cut off -- came back as 503, `answered`
//! RELEASED the key (a 5xx is not an answer), and the customer's retry ran the
//! command again: placement mints its order id per attempt, so that retry was
//! a second order, a second reservation and a second kitchen ticket.
//!
//! THE FIX. The Worker hands the object its claim (`Claim`) beside the command
//! (`Claimed`), and the object, in the same turn that wrote the log, marks the
//! claim `committed` with the command's own output. From then on:
//!   * `release` never gives a committed claim back (`verdict::release`);
//!   * a retry is told `Seen::Committed` and rebuilds its answer from that
//!     output instead of sending the command again;
//!   * the Worker's final `record` replaces the mark with the whole answer.
//!
//! PURE: `commit` is one table operation; the object's write is `hubdo/idem.rs`.

use dowiz_hub::table::Table;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use worker::*;

use super::KIND;

/// The key a Worker claimed, and the body it claimed it for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claim {
    pub key: String,
    pub print: String,
}

/// A command's input with the caller's claim beside it. `idem` is absent when
/// the client sent no key, and the command's own fields are untouched.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claimed<T> {
    #[serde(flatten)]
    pub input: T,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idem: Option<Claim>,
}

/// Mark `claim` committed with `output`. `Ok(false)` (nothing written) when the
/// claim is not this one's any more: absent, another body's, or already
/// answered -- a committed mark must never overwrite an answer.
pub fn commit(t: &mut Table, claim: &Claim, output: &str) -> Result<bool> {
    let Some(mut r) = t.get(KIND, &claim.key).and_then(|j| serde_json::from_str::<Value>(&j).ok()) else {
        return Ok(false);
    };
    let print = r.get("print").and_then(Value::as_str) == Some(claim.print.as_str());
    let done = r.get("done").and_then(Value::as_bool) == Some(true);
    if !print || done {
        return Ok(false);
    }
    r["committed"] = json!(output);
    t.put(KIND, &claim.key, &r.to_string(), &[], &[])
        .map_err(|e| Error::RustError(format!("idempotency: {e}")))?;
    Ok(true)
}

/// The order id inside a stored envelope: what a committed retry answers with.
pub fn stored_id(stored: &str) -> Option<String> {
    serde_json::from_str::<Value>(stored).ok()?.get("id")?.as_str().map(str::to_string)
}

#[cfg(test)]
mod tests;
