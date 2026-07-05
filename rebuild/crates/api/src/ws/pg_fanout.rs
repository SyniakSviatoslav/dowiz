//! `PgListener` — the Rust successor to `packages/platform/src/message-bus.ts`'s `PgMessageBus`
//! (proposal §6, Q4 🔴 — "the silent-blackout class"). LISTEN/NOTIFY does not work over the
//! Supavisor transaction pooler (6543, multiplexed) — a `LISTEN` issued there is orphaned with NO
//! error, and the Node bus already sidesteps this via `createSessionPool()`. This module is the
//! Rust half of that same discipline, plus the ACTIVE liveness probe the Node bus never had.
//!
//! ## REV-S6-1 (🔴 CRIT — the load-bearing fix) — active heartbeat, not disconnect-detection
//! `PgMessageBus.checkHealth()` returns `'ok'` as long as the socket hasn't errored/ended
//! (`message-bus.ts:237-239`) — but a `LISTEN` orphaned on the wrong pool mode succeeds and NEVER
//! disconnects; it just silently never delivers again. [`HeartbeatMonitor`] is a SELF-`NOTIFY`
//! probe: the listener periodically `NOTIFY`s its own heartbeat channel (via
//! [`HEARTBEAT_CHANNEL`], through its OWN connection, `send_heartbeat_probe` below) and must
//! echo-receive it within [`HEARTBEAT_TTL`], else `degraded`. This catches the exact
//! connected-but-mute case a disconnect-only check cannot — see
//! `heartbeat_monitor_detects_the_connected_but_mute_case` for the DoD proof.
//!
//! ## The session-DSN guardrail (config-time, red→green)
//! [`listener_dsn`] takes `&Config` and returns ONLY `config.database_url_session` — there is no
//! code path in this module that can construct a listener from `database_url_operational`
//! (structurally stronger than a runtime assertion: the operational URL is simply never in scope
//! at the call site). [`listener_dsn_is_never_the_operational_url`] pins this.
//!
//! ## REV-S6-6 — claim-check → explicit `Event::Resync` (🔴, ties to the heartbeat)
//! A payload the Node bus slimmed for the 8000-byte NOTIFY cap (`serializeForNotify`,
//! `message-bus.ts:140-154`, `{_truncated:true, type, data:{id,_truncated:true}}`) is detected by
//! [`interpret_notify_payload`] and turned into a first-class `ControlFrame::Resync{entity,id}`
//! (Q-WS-CLAIMCHECK) — an explicit "refetch via REST" contract, not the old accidental-refetch
//! heuristic. The SAME heartbeat that catches the silent blackout is also the recovery trigger: on
//! a degraded→healthy transition the caller fires a `Resync` too (a NOTIFY lost mid-outage would
//! otherwise leave a subscribed socket silently stale under a green status dot, resolution.md
//! REV-S6-6).

use serde_json::Value;
use serde_json::value::RawValue;
use sqlx::postgres::PgListener;
use tokio::time::{Duration, Instant};

use crate::config::Config;

pub const HEARTBEAT_CHANNEL: &str = "ws_heartbeat";
pub const HEARTBEAT_TTL: Duration = Duration::from_secs(10);
/// Node's cap (`message-bus.ts:105`) — retried forever, never abandoned (the old 5-attempt cap
/// left a machine "alive but realtime-dead").
const RECONNECT_BACKOFF_CAP: Duration = Duration::from_secs(30);

/// The session-mode DSN a `PgListener` MUST connect on — never the operational (tx-pooler) one.
/// See module doc for why this is a structural guardrail, not a runtime check.
pub fn listener_dsn(config: &Config) -> &str {
    &config.database_url_session
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Health {
    Healthy,
    /// Connected but not fanning out (REV-S6-1's exact CRIT case) OR genuinely disconnected —
    /// either way, the status-dot truth-signal binds to THIS, not raw socket liveness (REV-S6-6).
    Degraded,
}

/// The active-heartbeat state machine — pure, DB-free, so the connected-but-mute DoD case is a
/// deterministic unit test (see `crate::ws::pg_fanout::tests`), not a live-Postgres probe.
pub struct HeartbeatMonitor {
    ttl: Duration,
    last_probe_at: Option<Instant>,
    echoed_since_last_probe: bool,
    degraded: bool,
}

impl HeartbeatMonitor {
    pub fn new(ttl: Duration) -> Self {
        HeartbeatMonitor {
            ttl,
            last_probe_at: None,
            echoed_since_last_probe: true,
            degraded: false,
        }
    }

    /// Call right after issuing the self-`NOTIFY` probe.
    pub fn probe_sent(&mut self, now: Instant) {
        self.last_probe_at = Some(now);
        self.echoed_since_last_probe = false;
    }

    /// Call when a notification on [`HEARTBEAT_CHANNEL`] arrives. Returns `true` iff this is a
    /// degraded→healthy RECOVERY — the caller fires `ControlFrame::Resync` (REV-S6-6).
    pub fn echo_received(&mut self) -> bool {
        self.echoed_since_last_probe = true;
        let recovered = self.degraded;
        self.degraded = false;
        recovered
    }

    /// Call once per tick (after a probe's TTL window could plausibly have elapsed) to refresh the
    /// health signal. A probe sent with no echo within `ttl` ⇒ `Degraded` — the connected-but-mute
    /// case, since a merely-disconnected listener would ALSO fail every query, but a silently
    /// orphaned `LISTEN` on the wrong pool mode looks perfectly connected while this is the only
    /// thing that notices.
    pub fn check(&mut self, now: Instant) -> Health {
        if !self.echoed_since_last_probe {
            if let Some(sent) = self.last_probe_at {
                if now.checked_duration_since(sent).unwrap_or(Duration::ZERO) >= self.ttl {
                    self.degraded = true;
                }
            }
        }
        if self.degraded {
            Health::Degraded
        } else {
            Health::Healthy
        }
    }
}

/// Capped exponential backoff, parity with `message-bus.ts:105`
/// (`Math.min(1000 * 2**attempts, 30000)`), retried forever (no attempt ceiling — the caller loops
/// indefinitely, never gives up).
pub fn reconnect_backoff(attempt: u32) -> Duration {
    let ms = 1000_u64.saturating_mul(2_u64.saturating_pow(attempt));
    Duration::from_millis(ms).min(RECONNECT_BACKOFF_CAP)
}

/// What the fan-out dispatcher does with one parsed NOTIFY payload.
#[derive(Debug, Clone)]
pub enum NotifyFrame {
    /// The opaque passthrough (REV-S6-3) — the producer's ORIGINAL bytes, unparsed; relayed inside
    /// `{room, data}` verbatim (`RawValue`, not `Value` — see `protocol.rs`'s module doc for why a
    /// `Value` round-trip is already a re-encode, key order included).
    Room(Box<RawValue>),
    /// Q-WS-CLAIMCHECK: the Node bus slimmed this payload for the 8000B NOTIFY cap. Rendered as an
    /// explicit typed signal instead of forwarding the slim marker object as if it were real data.
    Resync { entity: String, id: String },
}

// `RawValue` has no `PartialEq` (raw bytes vs. semantic equality is deliberately not defined by
// serde_json) — this compares the RAW TEXT for `Room` (exactly the property the golden-frame tests
// care about: same bytes, not just "parses to the same value") and structural equality for `Resync`.
impl PartialEq for NotifyFrame {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (NotifyFrame::Room(a), NotifyFrame::Room(b)) => a.get() == b.get(),
            (
                NotifyFrame::Resync { entity: e1, id: i1 },
                NotifyFrame::Resync { entity: e2, id: i2 },
            ) => e1 == e2 && i1 == i2,
            _ => false,
        }
    }
}

/// Detects the Node bus's claim-check marker (`{_truncated:true, type, data:{id,_truncated:true}}`,
/// `message-bus.ts:144-149`) and converts it to [`NotifyFrame::Resync`]; everything else passes
/// through opaquely as the ORIGINAL bytes (REV-S6-3 — this is the ONE deliberate exception: a
/// control-plane marker the Rust side must recognize, not application data it re-encodes). The
/// truncation check itself parses into a throwaway `Value` (fine — only 2 leaf strings are ever
/// read out of it); the common (non-truncated) path never reconstructs the payload from that
/// parse, it re-wraps the ORIGINAL `raw` text via `RawValue`.
pub fn interpret_notify_payload(raw: &str) -> NotifyFrame {
    if let Ok(value) = serde_json::from_str::<Value>(raw) {
        if value.get("_truncated").and_then(Value::as_bool) == Some(true) {
            let entity = value
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            let id = value
                .get("data")
                .and_then(|d| d.get("id"))
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| "unknown".to_string());
            return NotifyFrame::Resync { entity, id };
        }
    }
    // Not truncated. NOTIFY payloads are always valid JSON in practice (the Node bus
    // `JSON.stringify`s them before publishing), so this is expected to always take the `Ok` arm;
    // the one literal fallback ("null" — always valid JSON, cannot itself fail to parse) exists so
    // a malformed payload degrades to a harmless no-op frame instead of panicking or being dropped
    // silently.
    #[allow(
        clippy::unwrap_used,
        reason = "the fallback input is the literal string \"null\" — the simplest possible valid \
                  JSON document, so this specific call cannot fail; used instead of nesting a second \
                  fallible branch for an input that is itself a hardcoded constant"
    )]
    NotifyFrame::Room(
        RawValue::from_string(raw.to_string())
            .unwrap_or_else(|_| RawValue::from_string("null".to_string()).unwrap()),
    )
}

/// The live `PgListener` wrapper — a dedicated session-mode connection (see [`listener_dsn`]),
/// LISTENing on the heartbeat channel plus whatever room channels `ws::mod`'s `RoomRegistry` opens.
/// sqlx's own `PgListener` already transparently re-connects and re-`LISTEN`s all registered
/// channels inside `recv`/`try_recv` (`message-bus.ts:95-114` parity comes largely for free); this
/// wrapper adds the ACTIVE heartbeat probe on top, which sqlx has no equivalent of.
pub struct PgFanout {
    listener: PgListener,
}

impl PgFanout {
    /// Connects on `listener_dsn(config)` — structurally never the operational URL — and LISTENs
    /// the heartbeat channel. Requires a live Postgres; exercised only by the `#[ignore]` tests
    /// below (same posture as `crate::db`'s live-DB tests — no DB is reachable in this sandbox).
    pub async fn connect(config: &Config) -> Result<Self, sqlx::Error> {
        let mut listener = PgListener::connect(listener_dsn(config)).await?;
        listener.listen(HEARTBEAT_CHANNEL).await?;
        Ok(PgFanout { listener })
    }

    /// Issues the self-`NOTIFY` heartbeat probe on the LISTENER'S OWN connection (`&mut PgListener`
    /// implements `Executor`) — the same connection object currently registered for `LISTEN`, so a
    /// probe that silently never echoes back is exactly the connected-but-mute failure mode
    /// REV-S6-1 names, not a race against some OTHER connection's state.
    pub async fn send_heartbeat_probe(
        &mut self,
        monitor: &mut HeartbeatMonitor,
        now: Instant,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT pg_notify($1, $2)")
            .bind(HEARTBEAT_CHANNEL)
            .bind("hb")
            .execute(&mut self.listener)
            .await?;
        monitor.probe_sent(now);
        Ok(())
    }

    /// `LISTEN` a room channel (a room just got its first member — `ws::mod`'s subscribe handler).
    pub async fn listen_room(&mut self, channel: &str) -> Result<(), sqlx::Error> {
        self.listener.listen(channel).await
    }

    /// `UNLISTEN` a room channel (its last member just left — eager teardown, P1-WSDUP parity).
    pub async fn unlisten_room(&mut self, channel: &str) -> Result<(), sqlx::Error> {
        self.listener.unlisten(channel).await
    }

    /// Waits, unbounded, for the next notification — either the heartbeat echo (`channel ==
    /// HEARTBEAT_CHANNEL`) or a room event for `ws::mod` to fan out via
    /// [`interpret_notify_payload`]. Used inside `ws::mod::run_fanout`'s `tokio::select!` alongside
    /// the room-lifecycle channel and the heartbeat-probe timer, so none of the three starves the
    /// others (a caller wanting a BOUNDED wait — e.g. the live-DB test below — wraps this in its
    /// own `tokio::time::timeout`, same as `run_fanout` bounds it via the probe-interval tick).
    pub async fn recv(&mut self) -> Result<(String, String), sqlx::Error> {
        let notification = self.listener.recv().await?;
        Ok((
            notification.channel().to_string(),
            notification.payload().to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{MediaConfig, NotificationsConfig};

    fn config(operational: &str, session: &str) -> Config {
        Config {
            port: 8080,
            database_url_operational: operational.to_string(),
            database_url_session: session.to_string(),
            media: MediaConfig::default(),
            notifications: NotificationsConfig::default(),
        }
    }

    // ── the config guardrail (red→green) ──

    #[test]
    fn listener_dsn_is_never_the_operational_url() {
        let cfg = config(
            "postgres://op-host:6543/db",
            "postgres://session-host:5432/db",
        );
        assert_eq!(listener_dsn(&cfg), "postgres://session-host:5432/db");
        assert_ne!(listener_dsn(&cfg), cfg.database_url_operational);
    }

    // ── REV-S6-1 (🔴 CRIT) — the named DoD test: connected-but-mute, not disconnect ──

    #[test]
    fn heartbeat_monitor_detects_the_connected_but_mute_case() {
        // The listener never errors, never disconnects — it just never hears its own echo. A
        // disconnect-only check (the Node bus's `checkHealth`) would report 'ok' forever here.
        let mut hb = HeartbeatMonitor::new(Duration::from_secs(10));
        let t0 = Instant::now();
        assert_eq!(
            hb.check(t0),
            Health::Healthy,
            "no probe sent yet — nothing to be mute about"
        );

        hb.probe_sent(t0);
        assert_eq!(
            hb.check(t0 + Duration::from_secs(5)),
            Health::Healthy,
            "within the TTL window, absence of an echo yet is not itself degraded"
        );
        assert_eq!(
            hb.check(t0 + Duration::from_secs(10)),
            Health::Degraded,
            "TTL elapsed with ZERO echo — the silent-blackout case REV-S6-1 exists to catch"
        );
    }

    #[test]
    fn heartbeat_monitor_stays_healthy_when_the_echo_arrives_in_time() {
        let mut hb = HeartbeatMonitor::new(Duration::from_secs(10));
        let t0 = Instant::now();
        hb.probe_sent(t0);
        hb.echo_received();
        assert_eq!(hb.check(t0 + Duration::from_secs(10)), Health::Healthy);
    }

    #[test]
    fn heartbeat_monitor_fires_recovery_exactly_once_on_degraded_to_healthy() {
        let mut hb = HeartbeatMonitor::new(Duration::from_secs(10));
        let t0 = Instant::now();
        hb.probe_sent(t0);
        assert_eq!(hb.check(t0 + Duration::from_secs(10)), Health::Degraded);

        // REV-S6-6: the recovery echo fires Resync exactly once, not on every subsequent healthy echo.
        assert!(
            hb.echo_received(),
            "the FIRST echo after degraded is the recovery transition"
        );
        assert!(
            !hb.echo_received(),
            "a second echo is just routine — not a fresh recovery"
        );
        assert_eq!(hb.check(t0 + Duration::from_secs(20)), Health::Healthy);
    }

    #[test]
    fn heartbeat_monitor_re_degrades_after_a_fresh_missed_probe() {
        let mut hb = HeartbeatMonitor::new(Duration::from_secs(10));
        let t0 = Instant::now();
        hb.probe_sent(t0);
        hb.echo_received();
        let t1 = t0 + Duration::from_secs(30);
        hb.probe_sent(t1);
        assert_eq!(hb.check(t1 + Duration::from_secs(10)), Health::Degraded);
    }

    // ── reconnect backoff (parity with message-bus.ts's capped exponential) ──

    #[test]
    fn reconnect_backoff_matches_node_capped_exponential() {
        assert_eq!(reconnect_backoff(1), Duration::from_millis(2_000));
        assert_eq!(reconnect_backoff(2), Duration::from_millis(4_000));
        assert_eq!(reconnect_backoff(3), Duration::from_millis(8_000));
        assert_eq!(reconnect_backoff(4), Duration::from_millis(16_000));
        assert_eq!(
            reconnect_backoff(5),
            Duration::from_secs(30),
            "capped at 30s, not 32s"
        );
        assert_eq!(
            reconnect_backoff(50),
            Duration::from_secs(30),
            "stays capped, never overflows"
        );
    }

    // ── REV-S6-6 / Q-WS-CLAIMCHECK: claim-check → explicit Resync ──

    #[test]
    fn interpret_notify_payload_passes_a_normal_payload_through_opaquely() {
        let raw = r#"{"type":"order.status","data":{"status":"CONFIRMED","total":52}}"#;
        let expected = RawValue::from_string(raw.to_string()).unwrap();
        assert_eq!(interpret_notify_payload(raw), NotifyFrame::Room(expected));
    }

    #[test]
    fn interpret_notify_payload_converts_the_truncated_marker_to_resync() {
        // Exact `serializeForNotify` slim shape (message-bus.ts:144-149).
        let raw = r#"{"_truncated":true,"type":"order.status","data":{"id":"22222222-2222-2222-2222-222222222222","_truncated":true}}"#;
        assert_eq!(
            interpret_notify_payload(raw),
            NotifyFrame::Resync {
                entity: "order.status".to_string(),
                id: "22222222-2222-2222-2222-222222222222".to_string(),
            }
        );
    }

    // ── live-Postgres proof (requires DATABASE_URL_SESSION; not run in this sandbox — see
    // crate::db's identical posture) ──

    /// REV-S6-1's build/cutover DoD: "a reconnect test (drop the listener conn → re-LISTEN →
    /// resume fan-out)". `PgListener::recv` already transparently reconnects + re-`LISTEN`s every
    /// registered channel (sqlx internals) — this proves OUR wrapper's self-heartbeat round-trips
    /// on a real session-mode connection end to end: probe out via `send_heartbeat_probe`, echo
    /// back in via `recv` (wrapped in a bounded `tokio::time::timeout` here, same as
    /// `run_fanout`'s `tokio::select!` bounds it via the probe-interval tick), feeding
    /// `HeartbeatMonitor::echo_received`.
    #[tokio::test]
    #[ignore = "requires a live Postgres — set DATABASE_URL_SESSION and run with --ignored"]
    async fn pg_fanout_self_heartbeat_round_trips_on_a_real_session_connection() {
        let database_url_session = std::env::var("DATABASE_URL_SESSION")
            .expect("DATABASE_URL_SESSION must be set for this ignored test");
        let cfg = config("postgres://unused-in-this-test/db", &database_url_session);
        let mut fanout = PgFanout::connect(&cfg)
            .await
            .expect("PgFanout::connect must succeed");
        let mut monitor = HeartbeatMonitor::new(Duration::from_secs(10));

        fanout
            .send_heartbeat_probe(&mut monitor, Instant::now())
            .await
            .expect("the self-NOTIFY probe must succeed");

        let (channel, _payload) = tokio::time::timeout(Duration::from_secs(5), fanout.recv())
            .await
            .expect("the self-NOTIFY echo must arrive within 5s on a healthy session connection")
            .expect("recv must not error");
        assert_eq!(channel, HEARTBEAT_CHANNEL);
        monitor.echo_received();
        assert_eq!(monitor.check(Instant::now()), Health::Healthy);
    }
}
