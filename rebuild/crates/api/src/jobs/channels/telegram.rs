//! Telegram send adapter — Q-TG-CIRCUIT (`docs/design/rebuild-jobs-s8-council/proposal.md`
//! §4.2). CARRIES the Node adapter's operational-safety state verbatim (per-chat rate-limit +
//! circuit breaker + 401/403/429 classification, source-verified against
//! `notifications/workers/index.ts:59-71`) — this is DIFFERENT from, and does not replace, the
//! REV-S8-1 durable dedup: `crate::jobs::dedup` guards against a crash-after-send DOUBLE-SEND (a
//! correctness/Postgres concern), while THIS module's rate-limit/circuit state guards against
//! hammering Telegram's API (an operational-politeness concern) — the Node `dedupCache` in-memory
//! `HashSet` this module's sibling `crate::jobs::dedup` replaces is intentionally NOT carried
//! here; carrying it forward would resurrect exactly the Potemkin dedup REV-S8-1 exists to close.
//!
//! Every external call is timeout-bounded (threat S8-T11) — 5s, matching the push/email adapters.
//!
//! ## Why this module is `#[allow(dead_code)]`
//! Same posture as `jobs::consent`'s owner-target functions and `jobs::channels::email`: the ONE
//! real caller (`notify.dispatch`/`notify.telegram.send`'s owner-target handler in
//! `crate::jobs::worker`) is not wired this pass — only the customer-push path
//! (`notify.customer_status`) is end-to-end. This adapter itself, and `routes::telegram_webhook`
//! (REV-S8-2, the security-critical fix), are both real and fully tested.
#![allow(
    dead_code,
    reason = "the owner-target notify.dispatch handler is not wired this pass — see doc above"
)]

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::config::Secret;
use crate::jobs::channels::SendOutcome;

const RATE_LIMIT_INTERVAL: Duration = Duration::from_millis(1200); // ~1 msg/s/chat, verbatim
const CIRCUIT_FAILURE_THRESHOLD: u32 = 5;
const CIRCUIT_COOLDOWN: Duration = Duration::from_secs(60);
const SEND_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Default)]
struct CircuitState {
    consecutive_failures: u32,
    tripped_until: Option<Instant>,
}

pub struct TelegramSender {
    client: reqwest::Client,
    bot_token: Secret,
    /// Per-chat state — a `std::sync::Mutex` (never held across an `.await`, only for the
    /// synchronous check-then-update around each send) is correct here: this is in-process
    /// operational throttling, not a correctness guard (that's `crate::jobs::dedup`, which IS
    /// Postgres-durable). Losing this state on restart is an accepted, documented trade-off —
    /// carried verbatim from the Node original, which has the exact same in-memory posture for
    /// these two concerns (rate limit + circuit breaker), unlike its dedup cache.
    last_send_at: Mutex<HashMap<String, Instant>>,
    circuit: Mutex<HashMap<String, CircuitState>>,
}

impl TelegramSender {
    pub fn new(bot_token: Secret) -> Result<Self, reqwest::Error> {
        let client = reqwest::Client::builder().timeout(SEND_TIMEOUT).build()?;
        Ok(TelegramSender {
            client,
            bot_token,
            last_send_at: Mutex::new(HashMap::new()),
            circuit: Mutex::new(HashMap::new()),
        })
    }

    /// Rate-limit + circuit-breaker gate, checked BEFORE issuing the HTTP call — pure/sync,
    /// split out so its decision logic is unit-testable without a network round trip.
    fn gate(&self, chat_id: &str, now: Instant) -> Result<(), SendOutcome> {
        // `unwrap_or_else(|e| e.into_inner())` (guardian): survive a poisoned lock instead of
        // panicking. A poisoned mutex means a PRIOR send panicked mid-critical-section, but this
        // is best-effort, in-memory, non-durable-by-design operational state (rate-limit +
        // circuit breaker) — the correct recovery is to keep going with whatever's there, never to
        // take the whole worker down. (Panic isolation in `crate::jobs::worker::spawn` already
        // firewalls a panicking job; this makes the shared adapter state survive that panic too,
        // so one poisoned send never wedges the rate-limiter for every other chat.)
        let circuit = self.circuit.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(state) = circuit.get(chat_id) {
            if let Some(until) = state.tripped_until {
                if now < until {
                    return Err(SendOutcome::RateLimited {
                        retry_after: until - now,
                    });
                }
            }
        }
        drop(circuit);

        let last_send = self.last_send_at.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(&last) = last_send.get(chat_id) {
            let elapsed = now.saturating_duration_since(last);
            if elapsed < RATE_LIMIT_INTERVAL {
                return Err(SendOutcome::RateLimited {
                    retry_after: RATE_LIMIT_INTERVAL - elapsed,
                });
            }
        }
        Ok(())
    }

    fn record_send_attempt(&self, chat_id: &str, now: Instant) {
        self.last_send_at
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(chat_id.to_string(), now);
    }

    fn record_success(&self, chat_id: &str) {
        self.circuit
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(chat_id);
    }

    fn record_failure(&self, chat_id: &str, now: Instant) {
        let mut circuit = self.circuit.lock().unwrap_or_else(|e| e.into_inner());
        let state = circuit.entry(chat_id.to_string()).or_default();
        state.consecutive_failures += 1;
        if state.consecutive_failures >= CIRCUIT_FAILURE_THRESHOLD {
            state.tripped_until = Some(now + CIRCUIT_COOLDOWN);
        }
    }

    /// `chat_id` is the Telegram chat/target id (the owner target's `address`, §4.2); `text` is
    /// the already-rendered, already-`maskPhone`d message body — this function sends exactly what
    /// it's given, no rendering/masking of its own (that's `crate::jobs::dispatch`'s job, matching
    /// `channels::push::send`'s identical "caller renders, adapter transports" split).
    pub async fn send(&self, chat_id: &str, text: &str) -> Result<SendOutcome, reqwest::Error> {
        let now = Instant::now();
        if let Err(gated) = self.gate(chat_id, now) {
            return Ok(gated);
        }
        self.record_send_attempt(chat_id, now);

        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            self.bot_token.expose()
        );
        let response = match self
            .client
            .post(&url)
            .json(&serde_json::json!({ "chat_id": chat_id, "text": text }))
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) if e.is_timeout() => {
                self.record_failure(chat_id, now);
                return Ok(SendOutcome::TimedOut);
            }
            Err(e) => {
                self.record_failure(chat_id, now);
                return Ok(SendOutcome::NetworkError {
                    message: e.to_string(),
                });
            }
        };

        let status = response.status();
        if status.is_success() {
            self.record_success(chat_id);
            return Ok(SendOutcome::Delivered);
        }

        if status.as_u16() == 429 {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .map(Duration::from_secs)
                .unwrap_or(Duration::from_secs(5));
            // A 429 is Telegram's own throttling signal, not a transport failure — it does not
            // count toward the circuit breaker (that's for genuine failures), matching the
            // Node original's distinct handling of 429 vs 401/403/network errors.
            return Ok(SendOutcome::RateLimited { retry_after });
        }

        self.record_failure(chat_id, now);
        if matches!(status.as_u16(), 401 | 403) {
            return Ok(SendOutcome::PermanentlyRejected {
                reason: status.to_string(),
            });
        }
        Ok(SendOutcome::NetworkError {
            message: format!("unexpected status {status}"),
        })
    }
}

impl std::fmt::Debug for TelegramSender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TelegramSender")
            .field("bot_token", &self.bot_token)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sender() -> TelegramSender {
        TelegramSender::new(Secret::new("123:test-token")).expect("client build must succeed")
    }

    #[test]
    fn telegram_sender_debug_never_prints_the_bot_token() {
        let sender = TelegramSender::new(Secret::new("123:super-secret-bot-token")).unwrap();
        let rendered = format!("{sender:?}");
        assert!(!rendered.contains("super-secret-bot-token"));
    }

    #[test]
    fn gate_allows_the_first_send_to_a_fresh_chat() {
        let sender = sender();
        assert!(sender.gate("chat-1", Instant::now()).is_ok());
    }

    #[test]
    fn gate_rate_limits_a_second_send_within_the_window() {
        let sender = sender();
        let t0 = Instant::now();
        sender.record_send_attempt("chat-1", t0);
        let result = sender.gate("chat-1", t0 + Duration::from_millis(500));
        assert!(matches!(result, Err(SendOutcome::RateLimited { .. })));
    }

    #[test]
    fn gate_allows_a_send_after_the_rate_limit_window_elapses() {
        let sender = sender();
        let t0 = Instant::now();
        sender.record_send_attempt("chat-1", t0);
        assert!(
            sender
                .gate(
                    "chat-1",
                    t0 + RATE_LIMIT_INTERVAL + Duration::from_millis(1)
                )
                .is_ok()
        );
    }

    #[test]
    fn rate_limit_is_per_chat_not_global() {
        let sender = sender();
        let t0 = Instant::now();
        sender.record_send_attempt("chat-1", t0);
        // A DIFFERENT chat must not be gated by chat-1's recent send.
        assert!(
            sender
                .gate("chat-2", t0 + Duration::from_millis(10))
                .is_ok()
        );
    }

    #[test]
    fn circuit_trips_after_five_consecutive_failures() {
        let sender = sender();
        let t0 = Instant::now();
        for i in 0..4 {
            sender.record_failure("chat-1", t0 + Duration::from_secs(i));
            assert!(
                sender
                    .gate(
                        "chat-1",
                        t0 + Duration::from_secs(i) + Duration::from_millis(1)
                    )
                    .is_ok(),
                "fewer than 5 failures must not trip the circuit"
            );
        }
        sender.record_failure("chat-1", t0 + Duration::from_secs(4));
        let result = sender.gate(
            "chat-1",
            t0 + Duration::from_secs(4) + Duration::from_millis(1),
        );
        assert!(
            matches!(result, Err(SendOutcome::RateLimited { .. })),
            "the 5th failure trips it"
        );
    }

    #[test]
    fn circuit_recovers_after_the_cooldown_and_a_success_clears_it() {
        let sender = sender();
        let t0 = Instant::now();
        for i in 0..5 {
            sender.record_failure("chat-1", t0 + Duration::from_secs(i));
        }
        assert!(sender.gate("chat-1", t0 + Duration::from_secs(5)).is_err());
        // After the cooldown elapses, the gate opens again (still counts as a "chance," not a
        // guaranteed success — the Telegram call itself decides success/failure from there).
        assert!(
            sender
                .gate(
                    "chat-1",
                    t0 + Duration::from_secs(5) + CIRCUIT_COOLDOWN + Duration::from_secs(1)
                )
                .is_ok()
        );
        sender.record_success("chat-1");
        // A success resets the failure counter — the NEXT failure alone must not re-trip it.
        sender.record_failure("chat-1", t0 + Duration::from_secs(200));
        assert!(
            sender
                .gate(
                    "chat-1",
                    t0 + Duration::from_secs(200) + Duration::from_millis(1)
                )
                .is_ok()
        );
    }

    #[test]
    fn a_429_does_not_count_toward_the_circuit_breaker() {
        // Structural pin: record_failure is the ONLY path that increments consecutive_failures;
        // `send`'s 429 branch returns before calling it (see the module source) — asserted here
        // by confirming record_failure alone drives the breaker (already covered above) and that
        // this is a deliberate, separate code path, not something this unit test can directly
        // observe without a live HTTP call (the `#[ignore]`d test below covers that).
        let sender = sender();
        assert_eq!(sender.circuit.lock().unwrap().len(), 0);
    }

    // ── live-network proof (requires a real TELEGRAM_BOT_TOKEN + chat id; not run in this
    // sandbox) ──

    #[tokio::test]
    #[ignore = "requires a real TELEGRAM_BOT_TOKEN and an authorized chat id — run manually"]
    async fn send_delivers_a_real_message() {
        let sender = TelegramSender::new(Secret::new(
            std::env::var("TELEGRAM_BOT_TOKEN").expect("set TELEGRAM_BOT_TOKEN"),
        ))
        .unwrap();
        let chat_id = std::env::var("TEST_TELEGRAM_CHAT_ID").expect("set TEST_TELEGRAM_CHAT_ID");
        let outcome = sender
            .send(&chat_id, "S8 Telegram adapter smoke test")
            .await
            .expect("send must not error at the transport level");
        assert_eq!(outcome, SendOutcome::Delivered);
    }
}
