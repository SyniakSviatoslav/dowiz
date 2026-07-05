//! The notification dispatch orchestrator — claim-check payload (Q5 🔴), seat-first tenant read
//! (REV-S8-5b), and structural error redaction (REV-S8-5d).
//! `docs/design/rebuild-jobs-s8-council/proposal.md` §5, §8; resolution.md REV-S8-5.

use uuid::Uuid;

use domain::TenantId;
use sqlx::PgPool;

use crate::db::{TenantTxnError, with_tenant, with_user};

/// The claim-check job payload (§5, Q-CLAIM-CHECK) — carries an entity reference, NOT contact
/// data. `notify.dispatch` / `notify.telegram.send` / `notify.customer_status` all use this exact
/// shape. Structurally PII-free: there is no field on this type a caller could even attempt to
/// populate with a phone/name/address — the worker re-fetches under tenant isolation at dispatch
/// time (`customer_status_context` below) and renders with `maskPhone` (carried, not yet ported —
/// `crate::jobs::channels::telegram`'s job once it lands). A job that dies into the DLQ persists
/// exactly this struct, so the DLQ inherits the same PII-free guarantee for free.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct JobPayload {
    pub entity_id: Uuid,
    pub location_id: Uuid,
    pub event: String,
}

/// The result of the seat-then-read customer-status context fetch (REV-S8-5b). Intentionally
/// carries only what a customer-status push needs to decide/render — not a full order row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomerStatusContext {
    pub order_exists: bool,
    pub customer_opted_in: bool,
}

/// REV-S8-5b (breaker MED + counsel #1/#2) — `handleCustomerStatus` read `orders` BEFORE seating
/// the tenant GUC (`notifications/workers/index.ts:108-122`, source-verified: the order SELECT at
/// lines 108-118 runs first, `set_config('app.user_id', ...)` at line 122 runs after, scoping
/// only the SUBSEQUENT `customer_devices` read — that second seat was ALREADY correctly ordered
/// in the old code; the ORDERS read is the one that wasn't). This was masked by BYPASSRLS; post-B3
/// (FORCE RLS everywhere) an un-seated read of ANY RLS-protected table returns ZERO rows silently
/// — no error, just an empty result the caller can't distinguish from "no such order." The FIX is
/// the S7 complete-census pattern: seat the GUC BEFORE **every** read this handler makes, not just
/// the one the old code happened to put after it.
///
/// Two DIFFERENT GUC families are involved, each seated before its own read, both reused verbatim
/// from `crate::db` rather than adding new near-identical helpers:
/// 1. The `notify.customer_status` job payload (`JobPayload`) carries `location_id`, NOT a
///    `customer_id` — the worker does not know which customer to act as until it reads the
///    order. So the ORDER read seats [`with_tenant`] (`app.current_tenant = location_id`, the
///    courier/service RLS root `db.rs` already reserves for this GUC family) BEFORE reading
///    `orders`, deriving `customer_id` from the row.
/// 2. Only THEN does the `customer_devices` read seat [`with_user`] (`app.user_id = customer_id`)
///    — the owner-write GUC helper, reused here for a customer id rather than an owner's, per
///    that function's own doc: the GUC NAME is generic ("the acting principal"), interpreted
///    differently by each table's own RLS policy.
///
/// Both seats happen STRUCTURALLY before their respective read — there is no code path in this
/// function that could read either table before its seat. Takes the bare operational `PgPool`
/// (not the whole `crate::db::Pools`) — neither seat needs the session pool, and a bare `PgPool`
/// is cheap to `Clone` into a spawned worker task (`crate::jobs::worker`), unlike `Pools` itself.
pub async fn customer_status_context(
    pool: &PgPool,
    order_id: Uuid,
    location_id: Uuid,
) -> Result<CustomerStatusContext, TenantTxnError> {
    let customer_id: Option<Uuid> = with_tenant(pool, TenantId::from(location_id), move |txn| {
        Box::pin(async move {
            sqlx::query_scalar("SELECT customer_id FROM orders WHERE id = $1 AND location_id = $2")
                .bind(order_id)
                .bind(location_id)
                .fetch_optional(&mut **txn)
                .await
        })
    })
    .await?;

    let Some(customer_id) = customer_id else {
        return Ok(CustomerStatusContext {
            order_exists: false,
            customer_opted_in: false,
        });
    };

    let customer_opted_in: bool = with_user(pool, customer_id, move |txn| {
        Box::pin(async move {
            sqlx::query_scalar(
                "SELECT COALESCE(bool_or(opted_in), false) FROM customer_devices WHERE customer_id = $1",
            )
            .bind(customer_id)
            .fetch_one(&mut **txn)
            .await
        })
    })
    .await?;

    Ok(CustomerStatusContext {
        order_exists: true,
        customer_opted_in,
    })
}

/// REV-S8-5d (PII, structural) — `last_error`/`error_message` currently write the RAW
/// `err.message` (`workers/index.ts`, `mig 007:8`'s `error_message` column) — verified
/// incidental-PII-free (nothing in today's error paths happens to embed contact data), which is
/// NOT the same guarantee as structural (nothing CAN). This function converts incidental into
/// structural: a bounded length cap (parity with the `ops:reconciliation_drift` alert's own
/// `substring(...,1000)`, §5) plus masking of the two shapes contact data actually takes in an
/// error message — an email-like token (masked wholesale) and a run of 4+ digits (a phone
/// fragment; deliberately NOT 3+, which would also eat order/id numbers and non-PII counts).
/// `crate::jobs::runner::fail` requires its caller to redact BEFORE calling it — this is that
/// caller-side step.
pub fn redact_error(raw: &str) -> String {
    const MAX_LEN: usize = 1000;
    let truncated: String = raw.chars().take(MAX_LEN).collect();

    let mut out = String::with_capacity(truncated.len());
    let mut word: Vec<char> = Vec::new();

    // Operates on one WHITESPACE-DELIMITED word at a time (not a running digit/token split across
    // the whole string) — a phone number can be written `+355691234567` (a leading `+` directly
    // against the digits, no separator), and a per-character state machine that starts a "token"
    // the moment it sees a non-digit char (the `+`) would then swallow every following digit INTO
    // that token instead of recognizing the digit run — exactly the bug an earlier version of this
    // function had (a `+`-prefixed phone number leaked through unmasked). Scoping the digit-run
    // scan to one word at a time sidesteps that: the `+` and the 12-digit run are both inside the
    // SAME word, so the run is found regardless of what precedes it.
    let flush_word = |word: &mut Vec<char>, out: &mut String| {
        if word.iter().collect::<String>().contains('@') {
            out.push_str("***");
        } else {
            let mut i = 0;
            while i < word.len() {
                if word[i].is_ascii_digit() {
                    let start = i;
                    while i < word.len() && word[i].is_ascii_digit() {
                        i += 1;
                    }
                    if i - start >= 4 {
                        out.push_str("***");
                    } else {
                        out.extend(&word[start..i]);
                    }
                } else {
                    out.push(word[i]);
                    i += 1;
                }
            }
        }
        word.clear();
    };

    for ch in truncated.chars() {
        if ch.is_whitespace() {
            flush_word(&mut word, &mut out);
            out.push(ch);
        } else {
            word.push(ch);
        }
    }
    flush_word(&mut word, &mut out);

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_payload_carries_only_the_claim_never_contact_data() {
        let payload = JobPayload {
            entity_id: Uuid::new_v4(),
            location_id: Uuid::new_v4(),
            event: "order.created".to_string(),
        };
        let json = serde_json::to_value(&payload).unwrap();
        let keys: std::collections::BTreeSet<_> =
            json.as_object().unwrap().keys().cloned().collect();
        assert_eq!(
            keys,
            ["entity_id", "location_id", "event"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            "the claim-check payload must never grow a phone/name/address field"
        );
    }

    // ── REV-S8-5d: error_message no-PII assert (named cutover-DoD test) ──

    #[test]
    fn redact_error_masks_phone_like_digit_runs() {
        let raw = "failed to notify +1 555 123 4567 about order 9";
        let redacted = redact_error(raw);
        assert!(!redacted.contains("1234567"));
        assert!(!redacted.contains("5551234567"));
        assert!(
            redacted.contains("order"),
            "non-PII words must survive redaction"
        );
    }

    #[test]
    fn redact_error_masks_email_like_tokens() {
        let raw = "smtp rejected recipient customer@example.com with code 550";
        let redacted = redact_error(raw);
        assert!(!redacted.contains("customer@example.com"));
        assert!(redacted.contains("smtp"));
        assert!(redacted.contains("rejected"));
    }

    #[test]
    fn redact_error_preserves_short_digit_runs_like_order_numbers_and_status_codes() {
        // 3-digit HTTP status codes and small counts must survive — only 4+ digit runs are
        // treated as phone-shaped.
        let raw = "telegram API returned 429 after 3 attempts";
        let redacted = redact_error(raw);
        assert_eq!(redacted, raw);
    }

    #[test]
    fn redact_error_truncates_to_1000_chars_parity_with_reconciliation_drift_alert() {
        let raw = "x".repeat(5000);
        let redacted = redact_error(&raw);
        assert_eq!(redacted.chars().count(), 1000);
    }

    #[test]
    fn redact_error_has_no_pii_pattern_after_redaction_property_check() {
        // A broader property sweep: several realistic error shapes, none may retain a
        // long digit run or an email token afterward.
        let samples = [
            "customer phone 0691234567 unreachable",
            "webhook to jane.doe@customer-mail.com timed out",
            "order 42 status 500: contact +355691234567 failed twice",
        ];
        for raw in samples {
            let redacted = redact_error(raw);
            assert!(
                !redacted
                    .chars()
                    .collect::<Vec<_>>()
                    .windows(4)
                    .any(|w| w.iter().all(|c| c.is_ascii_digit())),
                "redacted output must contain no run of 4+ consecutive digits: {redacted:?}"
            );
            assert!(
                !redacted.contains('@'),
                "redacted output must contain no '@': {redacted:?}"
            );
        }
    }
}
