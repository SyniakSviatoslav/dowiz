//! GET /api/version -- WHICH COMMIT IS THIS WORKER (W-DEPLOY, 2026-09-30).
//!
//! Before this, nothing live could say what code it was. A deploy from a dirty
//! tree, or one that failed after the build and was believed, looked exactly like
//! a good one, and the only check was a person diffing admin JS by hand.
//!
//! THE COMMIT IS BAKED INTO THE WASM AT COMPILE TIME (`option_env!`), not set as
//! a `wrangler --var`. A var is attached to the deployment whatever code was
//! built, so it would repeat what the deploy script believed; a compile-time
//! constant is only right when the bytes were compiled from a tree where the
//! script set it -- `tools/deploy/deploy.sh` sets it inside the clean HEAD copy
//! it builds from, and cargo re-runs the compile when the value changes.
//!
//! A value that is not what the script writes (40 lowercase hex for the commit,
//! `YYYY-MM-DDTHH:MM:SSZ` for the time) is reported as "unknown": a build made by
//! hand, by `cargo test`, or by the old `dowiz-deploy.sh` says so rather than
//! passing an arbitrary string through. No secrets, no auth, `no-store` (from
//! `harden`), and the handler reads no clock: `built_at` is the builder's time.
use serde_json::json;
use worker::{Request, Response, Result, RouteContext};

/// Set by `tools/deploy/deploy.sh` in the environment of the build.
pub const COMMIT: Option<&str> = option_env!("DOWIZ_COMMIT");
pub const BUILT_AT: Option<&str> = option_env!("DOWIZ_BUILT_AT");

const UNKNOWN: &str = "unknown";

/// The commit if it is a full lowercase SHA-1, else "unknown".
pub fn commit_or_unknown(v: Option<&str>) -> &str {
    match v {
        Some(s) if s.len() == 40 && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) => s,
        _ => UNKNOWN,
    }
}

/// The build time if it has the shape `2026-09-30T12:48:40Z`, else "unknown".
pub fn built_at_or_unknown(v: Option<&str>) -> &str {
    let ok = |s: &str| {
        let b = s.as_bytes();
        b.len() == 20
            && b.iter().enumerate().all(|(i, c)| match i {
                4 | 7 => *c == b'-',
                10 => *c == b'T',
                13 | 16 => *c == b':',
                19 => *c == b'Z',
                _ => c.is_ascii_digit(),
            })
    };
    match v {
        Some(s) if ok(s) => s,
        _ => UNKNOWN,
    }
}

/// The response body for a given pair of build values.
pub fn body(commit: Option<&str>, built_at: Option<&str>) -> serde_json::Value {
    json!({ "commit": commit_or_unknown(commit), "built_at": built_at_or_unknown(built_at) })
}

pub fn serve<D>(_: Request, _: RouteContext<D>) -> Result<Response> {
    Response::from_json(&body(COMMIT, BUILT_AT))
}

#[cfg(test)]
mod tests;
