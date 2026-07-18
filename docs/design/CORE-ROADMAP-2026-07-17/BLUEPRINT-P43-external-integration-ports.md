# BLUEPRINT P43 — External Integration Ports: transactional channels / tracking / export / media import (2026-07-18)

> **Planning document — writes no product code.** Written against the 20-point contract in
> `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map in §9 — every point
> addressed, none skipped). Deepens the roadmap-index DoD for **P43** in
> `docs/design/MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §10.5.5 (lines 1058-1076)
> to the standard's depth. Structure/depth template: `BLUEPRINT-P40-agent-loop-tool-wiring.md`
> / `BLUEPRINT-P41-three-mode-ai-operation.md` (same directory). Source arc:
> `integration-ports-reactive-arc-2026-07-13` (IP-11/12/13/14/19/20 absorbed here; the arc's
> two confirmed-false claims are carried as corrections, §0). Incorporates the 2026-07-18
> operator-directed research pass: **httpSMS** as the recommended own-infra SMS mechanism,
> a researched **WhatsApp Business Cloud API** transactional adapter design (with an honest
> cost model — it is NOT free like Telegram), a researched **SimpleX Chat adapter** as the
> architecturally-preferred ADDITIONAL privacy channel (§3.4b, second 2026-07-18 operator
> directive), and a **native Rust media-import port** for menu photos, pattern-referenced
> from Ghost-Downloader and deliberately NOT adopting it (§3.7).

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

Working tree `/root/dowiz`, branch `main` (`f9b2eb9bb`), 2026-07-18. Code rows read from live
files this session; web rows researched this pass (marked), not recalled.

| Claim | Fresh cite (this pass) | Status |
|---|---|---|
| The ONLY "messenger" code is non-sending link construction: "this is contact/link *construction* only — it never sends"; `telegram_link()` at `:33`, **`whatsapp_link()` (wa.me click-to-chat, optional prefilled message) at `:39`** | `kernel/src/messenger.rs:7,33,39` | verified — the roadmap's corrected claim confirmed; AND the wa.me builder is load-bearing for §3.4's free-window design |
| No send port exists: `kernel/src/ports/` = `{llm.rs, agent/, mod.rs}` — no `notify`/`social`/`media` module; no `notify-adapters`/`social-adapters` crate at repo root | live `ls`, this pass | verified — DoD-2's gap is real; P22's `social.rs` is also still unbuilt (its blueprint's own §1.4 says so) |
| QRNG bug (roadmap DoD-1): the kernel's feature-gated entropy provider hits the **deprecated legacy ANU endpoint** `qrng.anu.edu.au/API/jsonI.php` at `:48` and again hardcoded in the raw-TLS fallback at `:118-122` | `kernel/src/pq/entropy.rs:48,118,120,122` | verified live — two occurrences, not one; the fix must cover both (§3.1) |
| Outbox substrate exists in-kernel: `Spool` (`spool.rs:36`, 235 lines), `TokenBucket` (`token_bucket.rs:26`, 158 lines), `ChannelLedger` (`analytics.rs:40`, 312 lines) | live grep + `wc`, this pass | verified — P22 §1.2's claim independently re-confirmed; §3.2 reuses, never re-invents |
| Blob primitive exists: `backup.rs` = "native content-addressed backup organ… `chunker` (Buzhash CDC) splits a byte stream into content-defined blocks; this organ stores each unique block once, keyed by its `sha3_256` id… EXACT RESTORE" (702 lines); `chunker` registered at `lib.rs:44-46` | `kernel/src/backup.rs:1-15`, `kernel/src/lib.rs:44-46` | verified — §3.7's store target pattern; media import invents no storage |
| Media gap (MVP audit): "No blob/media-storage concept exists anywhere in the new stack… P48's surface implies them but no phase owns media ingest/storage/serving. Proposed home: P48 scope extension… sharing whatever capture/blob path P13's PoD photo lands" | `docs/design/DELIVERY-MVP-FEATURE-COMPLETENESS-AUDIT-2026-07-18.md:102,158,201` | verified — §3.7 reconciles this proposed-P48 home with this phase's import port EXPLICITLY (ingestion vs storage/serving split) |
| `?ch=` tracking is from-scratch: new `web/src` = `{app.mjs, lib}`, `grep "ch=" web/src` → 0 hits; `ChannelLedger.orders_by_channel` is the ready consumer | live `ls`/grep + `analytics.rs:40` | verified — roadmap correction confirmed; DoD-3 is a build, not a wire-up |
| P22 boundary (must not duplicate): P22's §11.5 channel-home table routes WhatsApp/Viber 1:1 **campaigns**, mailing lists, and **SMS campaigns** to the IP-15 `ChannelAdapter` campaign lane under P22's number; its row 6 reserves "Transactional sends (order-status, OTP) over ANY channel incl. SMS/email" to **P43 DoD-2 + P49**. Its §2.6 re-scoped WhatsApp *marketing* out of `SocialPoster` for the same reason | `BLUEPRINT-SOCIAL-AUTO-POSTING-2026-07-17.md` §11.5 (table + SMS-cost note), §2.6 (read this pass) | verified — P43's channel scope below is exactly the reserved transactional lane; zero overlap invented (§1.2) |
| P48/P49 boundaries: "INBOUND channel intake belongs to P48's hub; the OUTBOUND notification send path stays P43's (unchanged)"; P49 DoD-3 "rides P43 DoD-2's send path; stays RED until that path actually transmits"; P49 anti-scope: "Do NOT build a second notification transport — P43 owns the send path" | master roadmap `:1258`, `:1344-1345`, `:1350-1351` | verified — P43 is the transmitter, P49 the customer-side consumer, P48 the inbound hub |
| **httpSMS** (web-verified this pass, github.com/NdoleStudio/httpsms + docs.httpsms.com): open-source (AGPL-3.0) service turning an Android phone into an SMS gateway; send = `POST /v1/messages/send`, auth `x-api-key` header, payload `{content, from, to}` (+ optional `encrypted`, `request_id`, `send_at`), async 202 → phone sends via native SMS API → delivery status reported back; incoming SMS forwarded to user-provided webhook callback URLs; optional AES-256 E2E encryption with the key resident ONLY on the phone (server cannot read content); Docker-compose self-hosting supported; active (3.6k stars, latest release 2026-07) | web research 2026-07-18 | verified — §3.3's recommended default; API shape designed against, not assumed |
| **WhatsApp Business Cloud API** (web-verified this pass, Meta developer docs + multiple 2026 pricing guides): per-message pricing since 2025-07-01 (was per-conversation); categories marketing/utility/authentication are **billed from the first send** (no free allotment in 2026); **service conversations (customer-initiated, 24h window) are free — 1,000/month free tier**, and non-template messages inside an open customer-service window are not charged; utility templates delivered inside an open window are free; indicative US-market rates: marketing $0.025, utility $0.004, authentication $0.0135 per message (market-dependent). Onboarding: Meta Business Manager account + business verification + a dedicated phone number NOT registered on any personal/app WhatsApp; direct Cloud API access needs NO BSP in 2026 (Cloud API is the default onboarding; BSPs like 360dialog/€49-mo or Twilio/~$0.005-msg-markup are optional managed layers, not gatekeepers) | web research 2026-07-18 | verified — §3.4's cost model; the honest verdict: NOT free like Telegram, but a real free path exists for the common transactional flow |
| **Ghost-Downloader-3** (web-verified this pass): GPL-3.0, **Python + Qt/PySide6**, multi-protocol (HTTP/BitTorrent/FTP/HLS/DASH/eD2k/YouTube parsers), IDM-style intelligent chunking, pause/resume, browser-TLS-fingerprint emulation, aria2-compatible RPC; active (6.6k stars, v4.1.1 2026-07) | web research 2026-07-18 | verified — §3.7 pattern-references its concurrent/resumable/multi-source ingestion shape and REJECTS adopting it (foreign runtime; scope mismatch) |
| **SimpleX Chat bot mechanism** (web-verified this pass, github.com/simplex-chat/simplex-chat README + `bots/README.md`): a bot is a standalone process driving the **`simplex-chat` terminal CLI run as a local WebSocket server** (`simplex-chat -p 5225`) — "All communication between your bot process and CLI happens via JSON-encoded WebSocket text messages"; commands `{"corrId": "<unique>", "cmd": "<command string>"}`, responses/events `{"corrId", "resp": {"type": ...}}` (incoming messages arrive as `NewChatItems` events); capabilities include "create and manage long-term user address, accepting connection requests automatically or via code" and `APISendMessages`; the API "lacks authentication and binds only to localhost… must not be sent via public networks"; official TypeScript SDK (npm `simplex-chat`) + unofficial Rust SDK (crates.io `simploxide-client`); CLI docs: `/ad` = long-term contact address, `/c` = ONE-TIME invitation ("can only be used once and even if this is intercepted, the attacker would not be able to use it"), `-s smp://<cert-fingerprint>@host` = self-hosted server selection; **NO REST API, NO webhooks** — the integration shape is a local sidecar daemon. License AGPLv3 (+ trademark/asset restrictions); protocol audited by Trail of Bits; **no user identifiers of any kind** (no phone/username/central directory — disposable unidirectional SMP relay queues); PQ-augmented double ratchet (sntrup761 hybrid, default for direct chats since v5.7) | web research 2026-07-18 | verified — §3.4b's adapter is designed against this actual shape, not an assumed REST API |
| **SimpleX SMP relay self-hosting** (web-verified this pass, simplex.chat/docs/server.html + github.com/simplex-chat/simplexmq): single Haskell binary, AGPLv3; deploy via install script (systemd), Docker image, or Linode/DigitalOcean marketplace; "can be run on any Linux distribution, including low power/low memory devices" — reference deploy is a 1 GB shared-CPU Nanode; ports 5223/tcp (SMP) + 80/443 (cert/short-links, Caddy + Let's Encrypt automated in Docker setup); domain recommended ("in the near future client applications will start using server domain name in the invitation links"); server identity = offline cert fingerprint embedded in the address string | web research 2026-07-18 | verified — §3.4b's self-hosting DECART reasons from this footprint, and from SMP's recipient-creates-the-queue placement rule |
| `ureq` is the thrice-DECART'd sync HTTP client class for adapter crates; hand-rolled std multipart precedent exists | `BLUEPRINT-SOCIAL-AUTO-POSTING-2026-07-17.md` §6.3/§6.4 (citing HARNESS §5 Decision 2, `llm-adapters/Cargo.toml:12`) | cited — §2's adapter crate reuses the identical spec |

Ground truth is non-discussible; everything below builds on this table only.

---

## 1. Scope — what P43 owns vs what it must NOT touch

### 1.1 Build items

| Item | Content |
|---|---|
| E-a | QRNG endpoint fix (roadmap DoD-1) — small, independent, both occurrences (`:48` and the raw-TLS fallback `:118-122`) |
| E-b | `ChannelSend` port (`kernel/src/ports/notify.rs`) — the ONE transactional send seam — + `notify-adapters` crate with the **Telegram** adapter first (roadmap DoD-2: a send path that actually transmits) |
| E-c | **SMS adapter over httpSMS as the RECOMMENDED default** (owner's spare Android = the gateway; self-hosted or hosted base URL — same adapter), with a paid-provider adapter (Twilio/TurboSMS-class) as the OPTIONAL alternative for owners without a spare device — the reverse of the prior framing, with the cost-model correction recorded (§1.3) |
| E-d | **WhatsApp transactional adapter** — Cloud API direct, free-window-first design (§3.4): wa.me deep link opens a customer-initiated service window; status updates ride it free; outside-window sends need a paid utility template or a typed fallback to another channel |
| E-e | `?ch=` channel tracking in the new `web/src` + attribution into the order event so `ChannelLedger.orders_by_channel` reads it (roadmap DoD-3) |
| E-f | Data-export port: orders/menu as CSV + JSON, downloadable from a live deployment over P37's authenticated surface (roadmap DoD-4) |
| E-g | **Native Rust media-import port** (menu/content assets): owner pastes an image URL (or uploads bytes) → size/type-capped fetch → content-addressed blob via the `chunker`/`backup.rs` pattern behind a `BlobSink` seam — Ghost-Downloader's PATTERN at 1% of its scope (§3.7) |
| E-h | **SimpleX Chat adapter** (2026-07-18 operator directive) — the architecturally-preferred ADDITIONAL privacy channel: `SimpleXSender` driving the `simplex-chat` CLI sidecar over its localhost WebSocket JSON API, per-order one-time invitation onboarding, zero vendor cost, zero vendor account, no user identifiers by protocol. Additional alongside Telegram/WhatsApp/SMS, never a replacement. Built AFTER E-b proves the port (§3.4b sequencing) |

### 1.2 Boundary map (nothing here is invented — every neighbor's claim is honored)

| Neighbor | Their claim (cited) | What is left for P43 — stated honestly |
|---|---|---|
| **P22** (social + campaign lane) | Feed/channel posting (`SocialPoster`); WhatsApp/Viber/SMS/email **marketing campaigns** to recipient lists (IP-15 `ChannelAdapter`, consent-ledger-gated) — §11.5 | **Transactional sends only** — order-status + OTP, per-order, to the ONE customer who placed it. P22's own table reserved exactly this to P43 (row 6), so there is real, non-overlapping scope here: the trigger is an order event, never owner-authored content; the recipient is the order's customer, never a list. The two lanes may later share a low-level provider adapter (roadmap `:1061` anticipates it; httpSMS is the obvious candidate) — the *producer pipelines never merge* |
| **P48** (hub) | ALL inbound channel intake (orders arriving FROM messengers/social/web-forms) — roadmap `:1258`; media upload UI + storage decision + serving (MVP audit's proposed home) | P43 is outbound-only for messaging; for media, P43 owns the **import/ingestion mechanics** (URL fetch, caps, resumption, content-addressing) behind a `BlobSink` seam whose concrete store is P48/P13's shared storage decision — §3.7's split, stated there in full |
| **P49** (customer UX) | Consuming the send path customer-side (which channel a customer picks, identity/re-identification) | P43 builds the transmitter; P49 decides for whom/when. P49 DoD-3 stays RED until E-b transmits — that coupling is the roadmap's, honored |
| **P45** (ops) | Backup/monitoring floor; `backup.rs` end-to-end for tenant data | E-f export is an operator-facing *data-portability* file download, not backup; E-g reuses `backup.rs`'s content-addressing PATTERN for media, while P45 owns exercising backup itself |
| **P37/P38** (DELIVERY) | The live order flow + HTTP surface | Hard dependency: no adapter before there is a real order flow to notify about (roadmap anti-scope, kept). E-a is the ONLY item exempt (independent bug fix) |

### 1.3 The SMS cost-model correction (research finding, recorded)

The standing framing — roadmap §10.5.5 P22 item 3 and P22 blueprint §11.5: *"SMS is
per-message PAID via any provider (Twilio/TurboSMS-class)"* — was written against the
hosted-aggregator provider class only. **httpSMS falsifies the "via any provider" half**: a
self-hosted Android-gateway route has **zero per-message vendor fee** — the marginal cost
collapses to the owner's carrier SMS plan (commonly flat-rate/bundled at venue volumes), plus
the one-time cost of a spare Android device the owner very likely already has. What SURVIVES
from the old framing: SMS is never free-as-in-Telegram (a carrier plan is real money, and
carrier/OS throttles are real limits), and the campaign-lane preflight rule
(`recipients × unit_cost`) stays structurally correct — `unit_cost` just becomes
provider-configuration (0 vendor-fee under httpSMS; nonzero under Twilio-class) instead of an
axiom. This correction is P43's to record because P43 owns the provider adapter; **P22's
campaign lane inherits it via the shared low-level adapter** (§1.2 row 1) — no edit to P22's
blueprint is made here, and its preflight design needs none (the formula already parameterizes
`unit_cost`). Alignment note: an owner's phone as their venue's SMS gateway is the repo's
own-infra/no-vendor-lock-in doctrine (`integration-ports-reactive-arc`: core-immutable,
integrations as ports at the edge; sovereignty posture per `ops-reliability-arc`) applied to
telephony — the RECOMMENDED default, with the paid class as the opt-out, not the other way
around.

### 1.4 Anti-scope (each a review-rejectable smell)

1. **NOT social posting, NOT campaigns, NOT marketing.** P22 owns all of it (§1.2). The
   structural guard is a closed enum, not a review note: `NotifyKind` has variants
   `OrderStatus` and `Otp` only — a marketing payload through this port is
   **unrepresentable** (§2). Adding a `Marketing` variant here is the scope-violation smell.
2. **NOT recipient lists.** One notification addresses ONE recipient bound to ONE order.
   There is no bulk-send API in the port; fan-out machinery is P22's campaign lane behind its
   consent-ledger precondition.
3. **NOT built before DELIVERY P37/P38 live** (roadmap anti-scope verbatim) — a messenger
   port with nothing to send is dead code. Sole exemption: E-a (the QRNG fix), explicitly
   ungated by the roadmap.
4. **NOT touching `tools/telemetry`'s Telegram bridge** — OPS plumbing, not a product
   channel (roadmap anti-scope, kept).
5. **NOT a download manager.** E-g fetches ONE image at a time from ONE
   owner-supplied HTTP(S) URL under hard caps. No BitTorrent, no HLS, no YouTube, no
   browser-fingerprint spoofing, no general resumable-download framework (§3.7's DECART).
6. **NOT model/weights fetching.** AGENT-side model downloads are Ollama's own
   `ollama pull` (its registry protocol, its store, its resume logic) — a genuinely separate
   lifecycle from menu photos. Conflating them would couple product media to AI
   infrastructure across the P41 mode-1 boundary (menu photos must work at `AiMode::Off`).
   Named as an aside, owned by AGENT's ops notes, not this port.
7. **NOT image processing.** No transcode/resize dependency in the import port (P22 §6.4's
   flagged-dep discipline); renditions are the serving side's concern (P48 lane; P22's Viber
   1MB constraint stays P22's).
8. **NOT final vendor picks beyond the researched defaults.** httpSMS is the recommended
   default SMS mechanism and Cloud-API-direct the recommended WhatsApp route — both with the
   optional-adapter framing (§3.3/§3.4); no further vendor enumeration, no BSP selection.
9. **NOT email in v1.** `Channel::Email` is declared (closed enum, one future variant slot)
   but no SMTP adapter ships in this phase — transactional email has no researched design
   yet, and pretending otherwise would be decorative. Named trigger: first real venue that
   asks for email receipts.
10. **SimpleX is ADDITIONAL, never a replacement.** E-h joins the channel map alongside
    Telegram/WhatsApp/SMS for privacy-conscious customers; no existing channel is removed
    or demoted by it. Telegram stays the primary works-today channel (§3.4b's own wave
    verdict).
11. **NOT a native SMP/chat-protocol implementation.** Self-hosting means running the
    EXISTING open-source `smp-server` binary, and bot integration means driving the
    EXISTING `simplex-chat` CLI — reimplementing either protocol in Rust is a separate
    future DECART nobody has asked for. Same posture as anti-scope 5's "not a download
    manager": use the shipped artifact at the edge, don't rebuild it.
12. **NOT SimpleX's own funding/roadmap mechanics.** Community-voucher/funding-token ideas
    from SimpleX's project roadmap are SimpleX's business, irrelevant to a channel adapter —
    out of scope by name so search-result noise doesn't creep in.

**Dependency posture:** depends on DELIVERY P37/P38 (live flow) and PROTOCOL P34
(capability-gated egress) for E-b..E-h; E-a lands any time. Blocks nothing on the critical
path; P49 DoD-3 consumes E-b when both exist.

---

## 2. Predefined types & constants (standard item 4 — named BEFORE implementation)

```rust
// ── kernel/src/ports/notify.rs — NEW module. Registered in ports/mod.rs beside
// `llm`/`agent`/`tool` with a one-line doc. ZERO network/HTTP/JSON/serde —
// the llm.rs:1-7 compile-firewall header discipline, verbatim in spirit. ────────

/// Closed channel set. Adding a channel is a reviewed kernel-ports diff.
/// SimpleX added 2026-07-18 (E-h, §3.4b) — the additional privacy channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel { Telegram, Sms, WhatsApp, SimpleX, Email }

/// Closed transactional-kind set. `Marketing` is deliberately ABSENT —
/// a campaign payload through this port is unrepresentable (P22 boundary
/// as a type, §1.4-1). OTP rides the same port as status (one send seam).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotifyKind { OrderStatus, Otp }

/// One recipient address on one channel — per-order, never a list.
/// The inner string is channel-shaped (chat id / E.164 phone / wa id /
/// SimpleX bot-local contact id — NOT a global identity; none exists, §3.4b).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recipient { pub channel: Channel, pub address: String }

/// One transactional notification. Body text is composed by the CALLER
/// (P49's lane) from closed-vocabulary order state — this port transmits,
/// it does not author.
#[derive(Debug, Clone)]
pub struct Notification {
    pub order_id: String,
    pub kind: NotifyKind,
    pub recipient: Recipient,
    pub body: String,
}

/// Proof of transmission — what DoD-2's "actually transmits" means in data.
#[derive(Debug, Clone)]
pub struct SendReceipt {
    pub channel: Channel,
    pub provider_msg_id: String,  // Telegram message_id / httpSMS request-id / WA wamid
    pub unix_ms: u64,
}

/// Typed send failure. Every variant is an observable outcome, never a panic;
/// the order flow is unaffected by ALL of them (§4.3 isolation).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendError {
    NotConfigured,        // channel has no credentials/config for this venue
    ProviderDown,         // transport-level failure (daemon/gateway/API unreachable)
    WindowExpired,        // WhatsApp: no open service window and no template configured
    RateLimited,          // local TokenBucket or provider throttle refused
    InvalidRecipient(String),
    Timeout,
}

/// The send port. One impl per channel in notify-adapters; selected by
/// Recipient.channel at the composition layer.
pub trait ChannelSend {
    fn channel(&self) -> Channel;
    fn send(&self, n: &Notification) -> Result<SendReceipt, SendError>;
}

// ── kernel/src/ports/media_import.rs — NEW module (E-g). Same zero-I/O rule. ────

/// Closed asset-kind set: P43 ships menu photos ONLY. P13's PoD capture photo
/// joins as a second variant WHEN that phase lands (one storage decision,
/// two consumers — the MVP audit's own proposal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetKind { MenuPhoto }

#[derive(Debug, Clone)]
pub struct ImportRequest {
    pub venue_id: String,
    pub kind: AssetKind,
    pub source: ImportSource,
}
#[derive(Debug, Clone)]
pub enum ImportSource { Url(String), Bytes(Vec<u8>) }  // paste-a-URL or direct upload

/// Content-address of a stored asset (sha3-256 of the byte stream — the
/// backup.rs id convention, reused not re-invented).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlobId(pub [u8; 32]);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportError {
    TooLarge,             // exceeded MAX_MEDIA_BYTES mid-stream — aborted, partial discarded
    BadContentType(String), // not in the image allowlist
    ForbiddenTarget,      // SSRF guard: private/loopback/link-local address (§4.1)
    Unreachable,
    Timeout,
    Interrupted,          // transient mid-stream failure after retries exhausted
}

/// The storage seam — the ONE deliberately-open edge of this design: P43's
/// importer writes through it; the concrete store is P48/P13's shared
/// storage decision (§3.7). First impl: the chunker→content-addressed-store
/// pattern backup.rs already proves.
pub trait BlobSink {
    fn put(&mut self, bytes: &[u8]) -> Result<BlobId, String>;
}

pub trait MediaImport {
    fn import(&self, req: &ImportRequest, sink: &mut dyn BlobSink)
        -> Result<BlobId, ImportError>;
}

// ── Constants (notify-adapters + media-adapters) ────────────────────────────────
pub const MAX_MEDIA_BYTES: u64 = 10 * 1024 * 1024;   // menu photo ceiling, streaming-enforced
pub const IMPORT_TIMEOUT_MS: u64 = 30_000;
pub const IMPORT_RETRIES: u8 = 2;                     // transient-failure retries, Range-resumed
pub const IMAGE_TYPES: &[&str] = &["image/jpeg", "image/png", "image/webp"];

pub const SEND_TIMEOUT_MS: u64 = 10_000;
/// Per-venue transactional send budget (TokenBucket, reused verbatim):
/// generous for real order traffic, a wall for runaway loops.
pub const NOTIFY_BUCKET_CAPACITY: f64 = 60.0;
pub const NOTIFY_REFILL_PER_MIN: f64 = 10.0;
/// httpSMS/Android honesty: an Android gateway is throttled by OS/carrier
/// anti-spam heuristics; a conservative local ceiling keeps the phone's
/// number from being flagged. Config-overridable per venue, default low.
pub const SMS_MAX_PER_HOUR: u32 = 30;
/// WhatsApp customer-service window (Cloud API rule, web-verified §0).
pub const WA_WINDOW_HOURS: u64 = 24;
/// SimpleX CLI sidecar WebSocket endpoint. LOOPBACK ONLY — the CLI's own docs:
/// the WS API is unauthenticated and unencrypted, it must never cross a network
/// boundary (§3.4b adversarial v asserts this stays loopback).
pub const SIMPLEX_WS_URL: &str = "ws://127.0.0.1:5225";
/// The WS command vocabulary is generated from the CLI's core types and is
/// version-coupled, not a stable REST contract — the adapter pins a CLI version
/// in config and refuses to run against a mismatched daemon (§3.4b).
pub const SIMPLEX_CLI_VERSION_PIN: &str = "<set at build; asserted at startup>";
```

**Rejected alternatives (DECART-style, one line each):** *one mega-`IntegrationPort` trait for
send+export+import* — rejected: three unrelated I/O shapes (fire-and-receipt, pull-a-file,
stream-and-store) forced through one vocabulary is correspondence-breaking; three small ports
mirror `ports/{llm,tool}`'s one-concept-one-module convention. *Reusing P22's future
`ChannelAdapter` trait for transactional sends* — rejected: that trait's shape is
per-recipient **fan-out over a list** + consent ledger; this port is one-order-one-recipient —
sharing the low-level provider HTTP code (yes, later, §1.2) is not the same as sharing the
port. *`String` channel/kind fields* — rejected: closed enums are the boundary enforcement
(§1.4-1); strings would demote a type guarantee to a review promise.

---

## 3. Build items — spec → RED test → code, each with an adversarial case (items 3, 5)

### 3.1 E-a — QRNG endpoint fix (roadmap DoD-1; independent, land first)

`kernel/src/pq/entropy.rs` references the deprecated legacy ANU endpoint **twice** (`:48`
formatted URL; `:118-122` hardcoded host + path in the raw-TLS fallback) — one more occurrence
than the roadmap's summary implies (ground-truth delta, §0). Fix per the roadmap's stated
option set: align to the current ANU endpoint following the proto-cap implementation
(`bebop-repo/bebop2/proto-cap/src/entropy.rs`, `AnuQrng`/`SeedPool` — fail-closed tests
already exist there), or delete the dowiz copy in favor of the proto-cap one. **This blueprint
recommends alignment, not deletion** (the dowiz copy is feature-gated and dependency-light;
deletion couples kernel `pq` to a bebop-repo crate path — a workspace question this bug
doesn't need to answer). RED→GREEN: the existing fail-closed test pattern (transport/parse
failure ⇒ `Err`, caller falls back to OS entropy — `:46-47`'s own doc) re-pointed at the
current endpoint; RED = a test asserting the deprecated URL is absent
(`grep -c "jsonI.php" kernel/src/pq/entropy.rs` → 0) fails today. **Adversarial:** the
existing never-panic-on-entropy-loss contract re-asserted with a black-holed endpoint.

### 3.2 E-b — `ChannelSend` port + Telegram adapter + outbox spine (roadmap DoD-2)

The port (§2) lands in kernel-ports; `notify-adapters/` (repo root, standalone crate, `ureq`
per the thrice-cited spec) implements `TelegramSender` — Bot API `sendMessage` (JSON, chat_id
+ text), token from per-venue config, the SAME token-store discipline P22 §4.5 designs (shared
convention, separate file — the venue's bot token may legitimately be the same bot as P22's;
credentials unify at the store, pipelines stay separate).

**Outbox, reused not invented:** sends ride the existing `Spool` (enqueue → drain → receipt
or DEAD) with the existing `TokenBucket` as the per-venue budget (§2 consts) — P22 §5's
doctrine applied to a second producer, zero new machinery. A send failure NEVER propagates to
order state: the notification is an edge effect of an already-committed order event (§4.3).

RED→GREEN (the roadmap's own falsifier, made a test): RED today = `messenger.rs` is the only
messenger code (grep-asserted in the test preamble); GREEN = `telegram_actually_transmits` —
against a spy HTTP double asserting the exact wire shape, plus ONE live gated probe (env-gated
like the Ollama live tests: runs when a test bot token is configured, skips otherwise) that
delivers a real message and records the returned `message_id` as the `SendReceipt`.
**Adversarial:** (i) Telegram 429 → `RateLimited`, spool row survives for redrain, no
tight-loop retry (Spool's own backoff discipline); (ii) bad chat_id → `InvalidRecipient`,
row goes DEAD with the rendered reason — visible, never silently dropped; (iii) bucket
exhausted → `RateLimited` BEFORE any HTTP (spy asserts zero calls — fail-closed budget,
P40 §3.6's discipline).

### 3.3 E-c — SMS via httpSMS (RECOMMENDED default) + the optional paid-provider adapter

**DECART — SMS mechanism (decision made here, per the operator-directed research):**

| Criterion | **httpSMS (Android gateway) — CHOSEN default** | Twilio/TurboSMS-class (optional adapter) |
|---|---|---|
| Per-message vendor cost | **Zero** — carrier plan only (§1.3) | $0.0079+/msg-class, market-dependent |
| Infra sovereignty | Self-hostable (Docker), AGPL-3.0, key-on-phone AES-256 option — own-infra doctrine | Hosted vendor, per-message metering, account gatekeeping |
| Onboarding for a small venue | Spare Android + app + API key — minutes | Account, sender-ID/A2P registration, billing |
| Sender identity | The venue's own number (customers recognize it; replies work) | Rented number/alphanumeric ID |
| Throughput/reliability | Phone must be powered + online; OS/carrier throttles (~tens/hour safe) — FINE for transactional per-order volumes, WRONG for bulk | High-throughput, SLA'd — the reason the paid adapter exists |
| Wire shape | `POST {base}/v1/messages/send`, `x-api-key`, `{content, from, to}`, 202-async; delivery events + inbound via webhooks (§0 row) | Per-vendor REST |

**Verdict:** httpSMS default; paid-provider class = optional adapter for owners without a
spare device or with bulk needs (which are P22's campaign lane anyway — where the paid class
may earn its keep; shared low-level adapter per §1.2). The adapter takes `base_url` config so
hosted `api.httpsms.com` and self-hosted deployments are the SAME code path. AGPL honesty:
httpSMS runs as a separate network service; nothing links into our binary — no license
coupling. Delivery-status webhooks are a RECEIVE surface: the callback endpoint lands on P37's
HTTP surface (dependency named), updating the spool row's receipt state; v1 may ship
send-only (202-accepted = optimistic receipt) with the webhook as the immediate follow-up —
stated so the DoD row is honest about what "transmits" proves.

RED→GREEN: `httpsms_send_wire_shape` against a spy double (exact path/header/payload
asserted from the §0 row, not from memory); live probe env-gated like §3.2's.
**Adversarial:** (i) gateway phone offline (202 accepted, then a failed-delivery webhook) →
spool row moves to DEAD with the provider `failure_reason` — the async-failure path is
tested, not assumed happy; (ii) `SMS_MAX_PER_HOUR` bucket exhausted → `RateLimited`, zero
HTTP; (iii) non-E.164 recipient → `InvalidRecipient` before any HTTP.

### 3.4 E-d — WhatsApp transactional adapter (researched design; honest cost model)

**The honest verdict first (operator asked for it straight):** WhatsApp is **NOT free like
Telegram**. In 2026 every business-initiated template message (marketing/utility/
authentication) is billed per-message from the first send — there is no free business-
initiated tier. What IS free: **customer-initiated service conversations** — once the
customer messages the venue, a 24h window opens in which free-form (non-template) messages
cost nothing, with a 1,000-conversations/month free tier on that service category; utility
templates delivered inside an open window are also free (§0 row). Onboarding is also not
Telegram-cheap: Meta Business Manager + business verification + a dedicated number never
registered on personal WhatsApp. **No BSP is required** — direct Cloud API is the 2026
default onboarding route; BSPs (360dialog, Twilio, …) are optional managed layers with fees
or markups, honestly framed as such and not selected here (anti-scope 8).

**The design exploits the free path structurally — this is the load-bearing move:**
`kernel/src/messenger.rs:39` already builds `wa.me` click-to-chat links with a prefilled
message. The order-confirmation surface (P37/P49 lane) shows that link; a customer who taps
it and sends the prefilled "order {id}" message OPENS the service window — from then on,
order-status updates flow as **free non-template messages for 24h**, which covers virtually
every delivery lifecycle (order → delivered inside a day). The adapter:

- `WhatsAppSender`: Graph API `POST /{phone_number_id}/messages` (text body inside a window;
  named template outside one), bearer token per venue.
- **Window ledger:** inbound webhooks (customer messages, on P37's receive surface — same
  dependency note as §3.3) record `(recipient, opened_at)`; `send()` checks
  `now < opened_at + WA_WINDOW_HOURS`.
- **Fail-closed outside the window:** no open window AND no utility template configured ⇒
  `SendError::WindowExpired` — a typed outcome the composition layer answers by falling back
  to the customer's next channel (Telegram/SMS), never by silently paying for a template the
  venue didn't configure, never by dropping the notification silently.
- **Template path (opt-in, paid, per-venue config):** a venue that completes template review
  (utility category, ~$0.004-class US rates, market-dependent) gets outside-window sends;
  the config records the template name + an `acknowledged_paid: true` bit so the cost is a
  decision, not a surprise. OTP-over-WhatsApp (authentication templates, ~$0.0135-class)
  is representable (`NotifyKind::Otp`) but NOT the recommended OTP default — Telegram or
  httpSMS-SMS carry OTP at zero vendor cost.

**What P43 does NOT claim about WhatsApp (P22 boundary, re-stated):** marketing broadcasts,
recipient-list template blasts, and the tiered-limit quality-score game are P22's IP-15
campaign lane (its §2.6 finding, verbatim precedent). P43's WhatsApp surface is exactly:
per-order transactional messages to the order's customer. There IS real scope left for P43
here — P22's own table reserved it (§0 row) — so this is the reserved lane being built, not
overlap being invented.

RED→GREEN: `wa_window_send_is_free_form` + `wa_expired_window_is_typed` against a spy Graph
double (asserting text-vs-template payload selection by window state). Live probe deferred
until a venue completes Meta verification (external gate, named — same posture as P22's
Wave-2 Meta paperwork). **Adversarial:** (i) window expires BETWEEN enqueue and drain →
drain-time re-check ⇒ `WindowExpired` fallback, no stale-window template-free send attempt;
(ii) webhook replay/forged inbound (no valid signature) does NOT open a window — webhook
signature verification (Meta `X-Hub-Signature-256`) is fail-closed, tested with a bad-sig
fixture; (iii) template configured but Graph returns payment-failure → `ProviderDown`
rendering carries the provider error, row DEAD, visible.

### 3.4b E-h — SimpleX Chat adapter (operator directive 2026-07-18): the architecturally-preferred ADDITIONAL privacy channel

**Role, stated as architecture, not as a feature list:** every other channel in this map is
a platform — Telegram is free but account-centric (a BotFather token Telegram can revoke, a
bot Telegram can ban), WhatsApp is Meta-verified and per-template billed (§3.4), SMS rides a
carrier plan. SimpleX is a **protocol**: no phone number, no username, no central directory,
no account to suspend, no pricing lever to pull — recipient/sender anonymity via disposable
unidirectional SMP relay queues, with the relays themselves open-source and self-hostable
(§0 rows). That is not a coincidence to gloss over: it is structurally the SAME design this
repo locked as its own foundation in `docs/design/ARCHITECTURE.md` §0. The rhyme, made
explicit:

| SimpleX property (§0, verified) | dowiz mesh anchor (`ARCHITECTURE.md`) |
|---|---|
| No user identifiers of any kind; pairwise disposable queues are the only addressing | **M4** — every edge autonomous, self-certifying identity, no central CA |
| No central directory, no single point of failure; any relay can drop | **M7** — no SPOF, mesh heals; **F48** — per-hub replicated, no central authority |
| Relay operators are user-chosen per queue; anyone may run one (`-s smp://fingerprint@host`) | **M5** — every hub autonomous, opens its own bridges at its discretion |
| Server identity = offline cert fingerprint IN the address string — trust travels with the address, not a CA | **M6** — zero trust dependencies at the wire boundary; self-certifying frames |
| PQ-augmented double ratchet (sntrup761 hybrid, default since v5.7) | **M2** — PQ posture (different primitive — theirs sntrup761, ours ML-DSA/ML-KEM — same fail-toward-PQ direction; a rhyme, not an identity, stated honestly) |
| AGPLv3, Trail-of-Bits-audited | **E52** — the repo's own AGPLv3 license posture |

This is the httpSMS argument (§1.3's own-infra doctrine applied to telephony) extended to
messengers: among all channels here, SimpleX is the only one where **no third party can
deplatform, bill, or gatekeep the venue** — the strongest sovereignty profile in the map.
That earns it a place in the channel set on architectural grounds, NOT a claim that
customers will flock to it (§ wave verdict below is honest about reach).

**Integration mechanism (researched, not assumed — the shape matters):** there is **no REST
API and no webhook surface**. A SimpleX bot is a standalone process driving the
`simplex-chat` terminal CLI run as a **local sidecar daemon** with a WebSocket server
(`simplex-chat -p 5225`); all communication is JSON-encoded WS text messages —
`{"corrId", "cmd"}` out, `{"corrId", "resp"}` back, plus unsolicited events (incoming
messages arrive as `NewChatItems`) on the same socket (§0 row, quoted from the bot API
docs). The daemon owns the bot's profile, its SQLite state, and its SMP queue connections;
our adapter is a WS client on loopback. Consequences, named: (i) this is the **Ollama
precedent** (a local daemon the adapter talks to), not the ureq-HTTPS shape of §3.2-§3.4 —
`SendError::ProviderDown` means "daemon unreachable", and ops owns keeping the sidecar up
(systemd unit beside the app, P45's floor); (ii) the WS command vocabulary is
auto-generated from the CLI's core types — **version-coupled, not a stable contract** — so
the adapter pins the CLI version and fail-closes on a mismatched daemon at startup
(`SIMPLEX_CLI_VERSION_PIN`, §2); (iii) the WS API is unauthenticated/unencrypted by design
and must never leave loopback (§2 const; adversarial v).

**DECART — WS client implementation:**

| Option | Verdict |
|---|---|
| Official TypeScript SDK (npm `simplex-chat`) | REJECTED — foreign runtime the repo is actively eliminating (same row that rejected Ghost-Downloader's Python, §3.7) |
| Unofficial Rust SDK (crates.io `simploxide-client`) | NAMED FALLBACK — unofficial (maintenance risk), likely drags an async runtime into an edge crate that is otherwise sync; a new-dep DECART per `rust-native-bare-metal-decision` if chosen at build time |
| **Minimal hand-rolled RFC-6455 client — CHOSEN** | Loopback-only, no TLS, text frames + ping/pong/close, client-side masking: ~200 lines over `std::net::TcpStream`. The hand-rolled-multipart precedent (§0 `ureq` row) applied to one more small wire format; JSON via the adapter-layer serde_json convention (kernel stays serde-free) |

**Onboarding / first contact (RQ: how does a customer with NO identity connect?):** SimpleX
has nothing to "look up" — connection is by link, and the CLI mints two kinds (§0 row):
a long-term contact address (`/ad`) and a **one-time invitation** (`/c`), which "can only
be used once and even if this is intercepted, the attacker would not be able to use it".
The design uses the one-time form, because it solves the binding problem *structurally*:

1. Order-confirmation surface (P37/P49 lane) shows "Order updates via SimpleX" beside the
   §3.4 wa.me link and the Telegram link.
2. Tap → the backend asks the sidecar for a fresh one-time invitation → rendered as a
   `simplex:/invitation#/…` deep link + QR (the https://simplex.chat/… form lands
   no-app-yet customers on an install page — the adoption wall, named honestly below).
3. Customer's app connects; the bot auto-accepts (bot-API capability, §0 row); the
   connection-established event carries the invitation's correlation → **whoever completed
   THAT link IS that order's customer**. No code entry, no phone, no name — the binding is
   the link itself, which is exactly how SimpleX's own contact model works. The CLI's
   bot-local contact id becomes `Recipient.address` (§2) — a pairwise edge, not an identity.
4. Status updates flow over the connection as ordinary messages via the send port.

The venue's long-term `/ad` address (general "message us on SimpleX" presence) is an
INBOUND-intake lane — P48's hub per the §1.2 boundary — named here, not built here.
Re-identification across orders (the connection persists; the customer may reuse or delete
it) is P49's identity lane; P43 transmits to the order's bound contact, nothing more.

**Self-hosting DECART (RQ: co-host an SMP relay on the dowiz hub?):** the honest answer is
*optional, later, and less total than it sounds*. Facts (§0 row): `smp-server` is one
AGPLv3 Haskell binary, install-script/Docker deployable, runs on "low power/low memory
devices" (reference: 1 GB shared-CPU VPS), port 5223 + 80/443 for certs. So the FLOOR is
low and co-hosting on P45's ops floor is genuinely cheap. But two structural facts bound
the payoff: (i) the adapter is **config-indifferent** — `-s smp://<fingerprint>@host` on
the sidecar is the whole switch, hosted-preset and self-hosted are the SAME adapter code
path (the §3.3 httpSMS `base_url` argument, verbatim); (ii) SMP queues are created by the
RECEIVING side on servers that side chose — a venue relay hosts the **bot's inbound
queues only**; messages TO customers land on relays the customers' own apps picked. So
"self-host = full sovereignty" is false, and "venue hosts everything" would actually
CONCENTRATE the venue-side queue metadata on one operator — SimpleX's own threat model
prefers operator mix. **Verdict:** default = SimpleX preset relays (zero infra, day one);
self-hosted relay = a named OPTIONAL P45 ops item (one config line + one systemd/Docker
unit + cert-fingerprint key custody), pulled by a venue's sovereignty preference, and NOT
part of P43's DoD. Making it a DoD row would be scope creep bought with someone else's
ops budget — rejected in writing.

**Wave placement (RQ: is this Telegram-like or genuinely different?):** the push semantics
are in fact Telegram-LIKE — both channels are customer-initiated-once, then
business-push-forever: Telegram needs the customer to start the bot; SimpleX needs the
customer to accept a connection; after that, both allow unlimited business-initiated sends
at zero cost (no WhatsApp-style 24 h window — the connection IS the consent). What differs
is the **first-contact wall**: Telegram is one tap on an app this market already carries;
SimpleX requires installing a niche app before step 3 above can happen, and the sidecar
integration (daemon + version-pinned WS) is heavier than one ureq call. Friction-adjusted
reach, not cost, is the honest ranking axis — so: **E-h ships in-phase but sequenced after
E-b (Telegram) proves the port**; it can proceed in parallel with E-c/E-d once the
port/Spool spine exists, and it is deliberately NOT demand-gated out of the phase the way
email is (§1.4-9) — the architectural-fit case above is strategic, standing, and
operator-directed, not a per-venue feature request. OTP honesty: `NotifyKind::Otp` over
SimpleX reaches only already-connected customers — Telegram/httpSMS remain the OTP
defaults (§3.4's same note for WhatsApp).

RED→GREEN: the falsifier for "a real bot connection established + a message round-tripped"
is a **hermetic full-stack round-trip against the real binaries, not a simulation**:
`simplex_roundtrip_real_cli` boots a local `smp-server`, a bot `simplex-chat` CLI
(throwaway profile, `-s` pointed at the local relay) and a second "customer" CLI; the
adapter mints a one-time invitation over the WS API, the customer CLI connects via it, the
adapter sends an order-status message, the test asserts the customer CLI received it —
zero external network. Gated on the two binaries being present (env-gated exactly like the
Ollama live tests; skipped, never faked). A `simplex_ws_wire_shape` spy-WS test covers the
`{corrId, cmd}` framing hermetically on every CI run. **Adversarial:** (i) sidecar down →
`ProviderDown`, spool row survives for redrain, order flow untouched; (ii) customer
deleted the connection (their right — the model's whole point) → send to the dead contact
→ `InvalidRecipient`, row DEAD visible, P49's fallback lane picks the next channel;
(iii) one-time invitation reuse: a second connect attempt on a consumed invitation is
refused (protocol property, asserted against the real CLI in the round-trip test, and the
adapter never maps a second contact onto an already-bound order); (iv) version drift: WS
handshake against a daemon reporting a non-pinned version → adapter refuses to start
(fail-closed, no garbage commands against an unknown vocabulary); (v) misconfiguration
binding the WS beyond loopback → adapter refuses `SIMPLEX_WS_URL` hosts other than
127.0.0.1 by construction (the API is unauthenticated — §2's const comment is a tested
guard, not advice).

### 3.5 E-e — `?ch=` channel tracking (roadmap DoD-3; from scratch, honestly)

The old stack's `Storefront.svelte` is gone (roadmap correction); the new `web/src` has zero
tracking code (§0). Build: the storefront entry reads `?ch=` (closed vocabulary: the
`Channel` enum's lowercase names + `qr`/`direct`), carries it through order placement into
the order-event payload's existing attribution slot, and `ChannelLedger.orders_by_channel`
(`analytics.rs:40`) — already built, currently starved of data — becomes the read side with
zero changes. RED→GREEN: one E2E check (roadmap's own DoD wording): place an order via
`?ch=telegram` → the ledger's count for `telegram` increments. **Adversarial:** an unknown
`?ch=evil<script>` value maps to `direct` (closed vocabulary, no passthrough of raw strings
into analytics — injection surface closed by construction).

### 3.6 E-f — data-export port (roadmap DoD-4)

`ExportPort` (orders/menu → CSV and JSON) served from P37's authenticated surface
(owner-scoped capability cert — P37's auth model, inherited not re-invented). Std-only
serialization (CSV is 30 lines of escaping; JSON via the adapter layer's serde_json —
kernel stays serde-free). RED→GREEN: an operator downloads a file from a live deployment
and it round-trips (`export → parse → row-count == ledger count`). **Adversarial:** a
customer-scoped cert requesting the export → refused typed (authz negative test);
CSV-injection guard (`=SUM(...)` cell prefixed) — exports get opened in spreadsheets,
test named.

### 3.7 E-g — native media-import port (Ghost-Downloader pattern-referenced, NOT adopted)

**DECART — pattern vs dependency (the operator-flagged conflict, resolved):**

| Criterion | Adopt Ghost-Downloader | **Native Rust port scoped to need — CHOSEN** |
|---|---|---|
| Runtime | Python + Qt/PySide6 — a foreign runtime the repo is actively eliminating (CORE-ROADMAP-STANDARD §1: "kernel/Rust/WASM only… Node/TS/JS/Python are adapters at most, being actively eliminated") | Rust, `ureq`, std |
| Scope fit | Multi-protocol download MANAGER (BitTorrent/HLS/YouTube/TLS-fingerprint-spoofing) — ~99% unused surface for "fetch one menu photo" | Exactly the need: HTTP(S) GET, caps, resume, content-address |
| License/supply chain | GPL-3.0 app + a Python dep tree | Zero new deps beyond the thrice-cited `ureq` |
| Doctrine | Violates the CLONE/build-your-own-x line (`ecosystem-strategy-arc`): reverse-engineer the PATTERN, don't vendor the artifact | Is that line, applied |

**What the pattern actually teaches (credited, then right-sized):** Ghost-Downloader's value
is concurrent segmented fetching, pause/resume without progress loss, and multi-source
ingestion. For menu photos (~100 KB–5 MB, single origin, one owner action) the honest
reductions are: *segmentation* → *none* (one connection saturates any origin at this size —
multi-connection chunking would be cargo-cult); *resume* → **`Range` resume across transient
mid-stream failures within one import attempt** (`IMPORT_RETRIES`, resuming at the received
byte offset when the origin advertises `Accept-Ranges`; origins without it restart — both
paths tested); *pause across restarts* → *not needed* — an interrupted import is simply
re-run, and **content-addressing makes re-import idempotent for free** (same bytes ⇒ same
`BlobId`, the store dedups — Buzhash-CDC dedup is `backup.rs`'s literal property 1);
*multi-source* → the `ImportSource` enum (URL paste or direct upload bytes), not protocol
plurality; *concurrency* → a menu-batch of N URLs runs through the existing Spool with a
small bounded drain (reuse, no pool invention); *TLS-fingerprint spoofing* → **rejected
outright** (if an origin blocks non-browser fetches, the owner saves the image and uploads
it — `ImportSource::Bytes`; impersonating a browser to defeat a host's bot policy is not
this product's business).

**Pipeline:** validate URL (§4.1 SSRF guard) → streaming GET with `IMPORT_TIMEOUT_MS` +
running byte count (abort at `MAX_MEDIA_BYTES` — enforced mid-stream, never post-hoc) +
`Content-Type` allowlist (`IMAGE_TYPES`; plus a magic-bytes sniff on the first chunk, since
Content-Type headers lie) → `chunker` CDC → `BlobSink::put` → `BlobId` recorded on the menu
item (P37 data model's lane).

**Ownership reconciliation (the MVP-audit tension, resolved explicitly, not papered over):**
the audit proposed media's home as a P48 scope extension ("owner uploads → content-addressed
store → P37 serves", shared with P13's PoD photo). That proposal is HONORED, not overridden:
P48 keeps the upload UI, the storage *decision*, and serving; P13 keeps its capture path.
What P43 adds — and is the natural IP-19/20 (export/automation ports) home for — is the
**external-ingestion mechanics**: fetching from a foreign origin with caps, resume, SSRF
guarding, and content-addressing, delivered behind the `BlobSink` seam so the importer is
indifferent to whatever store P48/P13 pick. One storage decision, three consumers (owner
upload, URL import, PoD capture) — the audit's "one storage decision, two consumers"
extended by one, in writing.

RED→GREEN: `import_url_roundtrip` — a local test HTTP server serves a fixture JPEG; import
→ `BlobId` = sha3 of the fixture bytes; re-import → same id, sink saw one distinct blob
(idempotency proven). **Adversarial:** (i) 11 MB body with a lying 1 MB `Content-Length` →
`TooLarge` aborted mid-stream, partial bytes NOT in the sink; (ii) `http://169.254.169.254/`
and `http://127.0.0.1:8080/` → `ForbiddenTarget` with **zero** connection attempts (spy
resolver); (iii) a `text/html` body with `Content-Type: image/jpeg` → magic-sniff ⇒
`BadContentType`; (iv) mid-stream disconnect at byte 40% with `Accept-Ranges` → resumed
completion, one blob, correct hash; without `Accept-Ranges` → clean restart, same result.

---

## 4. Cross-cutting design obligations (items 6, 8, 9, 11–16)

### 4.1 Hazard-safety as math (item 6)

- **Marketing-through-the-port is unrepresentable:** `NotifyKind` has two variants and no
  free-text kind; the P22 boundary is a closed enum, not a review promise (§1.4-1). Bulk is
  unrepresentable the same way: `Notification` holds ONE `Recipient`; no list type exists in
  the port's vocabulary.
- **PII surface, named honestly:** `Recipient.address` (phone/chat id) IS personal data —
  the one place P43 touches PII. Structure bounds it: addresses live per-order (the order the
  customer created — transactional basis), are never aggregated into lists by any type here,
  and the campaign world's consent-ledger obligations stay P22's precondition. Retention
  follows the order's own lifecycle, not a CRM's.
- **SSRF (the import port's real hazard):** an owner-pasted URL fetched server-side is a
  classic SSRF vector. Guard is fail-closed and post-resolution: scheme allowlist
  (`http`/`https` only) + resolve THEN check every resolved address against deny ranges
  (loopback, RFC1918, link-local incl. 169.254.169.254, ULA) + connect only to the vetted
  address (no re-resolution TOCTOU) + no redirect-following across the guard (each hop
  re-vetted). Tested with the §3.7 adversaries.
- **Webhook surfaces (httpSMS delivery events, WhatsApp inbound):** signature-verified
  fail-closed (§3.4 adversarial ii); an unauthenticated webhook cannot open a WA window or
  mark an SMS delivered — forgery yields state changes worth exactly nothing (windows gate
  FREE sends, receipts gate ledger rows; neither touches order/money state, which no path
  from this phase reaches at all — the kernel Law has no import from any of these crates,
  same firewall discipline as every ports module).
- **Residual risks, named:** (i) the Android gateway phone is a physical single point for
  SMS — mitigated by typed `ProviderDown` + channel fallback, not hidden; (ii) WA window
  state is only as truthful as Meta's webhook delivery — a missed webhook fails CLOSED
  (window unknown ⇒ treated expired ⇒ fallback channel), never open.

### 4.2 Schemas & scaling axes (item 8)

Sends scale per order event (single digits per order; venue-day volume in the hundreds —
`TokenBucket` consts §2 govern; the axis to re-examine is multi-venue nodes, where buckets
are per-venue by key, already the ledger's shape). WA window ledger scales by active
recipients over 24h (entries expire at `WA_WINDOW_HOURS` — self-bounding, ~order-count-sized).
Media blobs scale by venue menu size × photo size, capped per-item by `MAX_MEDIA_BYTES`;
dedup (CDC) bounds re-import growth at zero. Export files scale by order history — streamed
row-wise, never fully buffered (the axis where a naive impl breaks first, named). `?ch=`
vocabulary is a closed set — no cardinality explosion by construction.

### 4.3 Isolation / bulkhead (item 11), mesh (item 12), rollback (item 13), living memory (item 15)

- **Isolation:** every channel adapter sits behind the Spool outbox — a channel outage
  degrades to spooled/DEAD rows, NEVER a blocked or lost order (notifications are edge
  effects of already-committed events; the dependency arrow points outward only). Per-channel
  buckets bulkhead one channel's throttling from another's. The importer runs in the owner
  surface's lane — a hung origin times out typed, affecting one import.
- **Mesh (item 12):** all P43 egress is node-local adapter I/O — not mesh-gossiped, no
  payload budget on the mesh. Egress rides PROTOCOL P34's capability-gated boundary
  (dependency, named). Webhook receive surfaces are P37's HTTP listener, not a new one.
- **Rollback (item 13, vocabulary used precisely):** Self-Termination leg — every external
  call has a deadline (`SEND_TIMEOUT_MS`/`IMPORT_TIMEOUT_MS`), retries are count-bounded
  (`IMPORT_RETRIES`, Spool's drain discipline), so worst-case wall time is closed-form per
  row. Snapshot-Re-entry leg — ONLY for media import: content-addressing makes any
  interrupted/repeated import converge to the identical `BlobId` (idempotent regeneration
  from source, §3.7's test). No Self-Healing claim (no redundancy math here). Mechanically
  reversible: delete the two ports modules + the adapter crates; kernel tests unaffected by
  construction.
- **Living memory (item 15):** receipts/DEAD rows live in the Spool's existing persistence;
  channel attribution lands in `ChannelLedger` — both are the established stores; no third
  telemetry/attribution channel is added (standard item 19).

### 4.4 Linux-discipline verdict framework (item 9)

**ALREADY-EQUIVALENT:** ports-at-the-edge with a serde-free kernel (fourth and fifth ports
modules following `llm`/`tool`'s convention). **REINFORCES:** typed-failure discipline —
every provider fault is a `SendError`/`ImportError` variant, no silent drops (the DEAD-row
visibility rule). **EXTENDS:** the fail-closed doctrine to *economic* state — WhatsApp's
paid/free boundary is encoded as fail-closed types (`WindowExpired` + `acknowledged_paid`),
so spending money is always an explicit configured decision, never an implicit fallback.
**GAP (named, deferred):** webhook ingress hardening beyond signatures (replay windows,
idempotency keys on delivery events) — follow-up when P37's receive surface lands, tracked
in §5's ledger row.

---

## 5. DoD — falsifiable, RED→GREEN, per item (item 2)

Extends §10.5.5's four P43 DoD lines (kept verbatim as rows 1–4) with this pass's additions.

| Item | RED (fails before) | GREEN (passes after) | Named test / check (permanent, item 17) |
|---|---|---|---|
| 1 QRNG fix | `grep -c "jsonI.php" kernel/src/pq/entropy.rs` → 2 (today, both occurrences) | → 0; fail-closed test green against current endpoint | `kernel` pq tests::`qrng_current_endpoint_fail_closed` + the grep check |
| 2 messenger transmits | `messenger.rs` is the only messenger code (grep preamble) | Telegram spy wire-shape + env-gated live delivery with real `message_id` receipt | `notify-adapters/tests/telegram.rs::{telegram_actually_transmits, tg_429_is_rate_limited, tg_bad_chat_dead_row, bucket_refuses_before_http}` |
| 3 `?ch=` tracking | 0 hits in `web/src`; ledger starved | E2E: order via `?ch=telegram` increments `orders_by_channel("telegram")` | `e2e` check::`channel_attribution_roundtrip` + `unknown_ch_maps_to_direct` |
| 4 export | no export surface | live-deployment download; parse round-trip row-count match | `notify-adapters`(export mod)`/tests/export.rs::{export_roundtrip_counts, customer_cert_refused, csv_injection_guarded}` |
| 5 SMS (httpSMS default) | no SMS code | spy wire-shape per §0's verified API + async-failure webhook → DEAD row | `notify-adapters/tests/httpsms.rs::{httpsms_send_wire_shape, delivery_failure_goes_dead, sms_hour_bucket_refuses, bad_recipient_refused}` |
| 6 WhatsApp windowed | no WA code | window-state selects free-form vs template vs typed `WindowExpired`+fallback; forged webhook opens nothing | `notify-adapters/tests/whatsapp.rs::{wa_window_send_is_free_form, wa_expired_window_is_typed, wa_drain_rechecks_window, wa_forged_webhook_rejected, wa_payment_failure_visible}` |
| 7 media import | no blob concept (MVP audit) | fixture round-trip + idempotent re-import + 4 adversaries (TooLarge mid-stream, SSRF zero-connect, magic-sniff, Range-resume) | `media-adapters/tests/import.rs::{import_url_roundtrip, reimport_is_idempotent, oversize_aborts_midstream, ssrf_targets_refused, lying_content_type_sniffed, midstream_resume_completes}` |
| 8 SimpleX (E-h) | no SimpleX code anywhere (grep preamble) | **real** bot connection + message round-trip against the actual `smp-server` + `simplex-chat` CLI binaries (hermetic local relay, binaries-gated, never simulated); WS wire-shape spy on every CI run; the 5 §3.4b adversaries typed | `notify-adapters/tests/simplex.rs::{simplex_roundtrip_real_cli, simplex_ws_wire_shape, sidecar_down_is_provider_down, deleted_contact_goes_dead, used_invitation_refused, version_drift_refuses_start, ws_loopback_only}` |

Ledger obligations (`docs/regressions/REGRESSION-LEDGER.md`, red→green per its ratchet rule):
(a) "SSRF guard on owner-pasted media URLs; guardrail: `ssrf_targets_refused`"; (b) "WA
paid-send requires explicit config; guardrail: `wa_expired_window_is_typed`"; (c) the §4.4
GAP row (webhook replay hardening) as a tracked-open item, not a silent omission; (d) "the
unauthenticated SimpleX WS API never leaves loopback; guardrail: `ws_loopback_only`".

---

## 6. Benchmark plan (item 10) — existing conventions, modest by design

Nothing here is hot-path next to network I/O; budgets exist to catch accidental stupidity,
not to tune: (1) `notify/spool_enqueue_overhead` — enqueue-to-drain-eligible latency,
**budget ≤ 1 ms** (in-process bookkeeping must be invisible next to a 100 ms+ provider
call); (2) `media/import_pipeline_overhead` — chunk+hash+store a 5 MB fixture from memory,
**budget ≤ 150 ms** (CDC + sha3 at memory speed; the measured number documents the
non-network cost so origin latency is never blamed on the pipeline). Numbers into the
established `BENCH_HISTORY.md` convention. Telemetry: send outcomes are Spool rows +
`ChannelLedger` counts — the existing read surfaces ARE the telemetry hook; no new channel.

---

## 7. Links to docs & memory (item 7)

Depends on / cites: `CORE-ROADMAP-STANDARD-2026-07-17.md` (the contract) ·
`MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §10.5.5 P43 (the index entry deepened;
its two false-premise corrections carried in §0/§3.5) + §11 P48/P49 boundary rulings
(`:1258`, `:1344-1351`) · `docs/design/ARCHITECTURE.md` §0 M-series (the mesh anchors
§3.4b's fit table cites) · `BLUEPRINT-SOCIAL-AUTO-POSTING-2026-07-17.md` §2.6/§11.5/§6.3/§6.4
(the P22 boundary honored; the shared-adapter anticipation; the ureq/multipart precedents) ·
`BLUEPRINT-P42-mcp-agent-skills.md` (sibling written this pass — any future P43 read-tools
join the agent catalog via its §3.1 growth rule, not via loop edits) ·
`DELIVERY-MVP-FEATURE-COMPLETENESS-AUDIT-2026-07-18.md` (the media gap + proposed home,
reconciled §3.7) · `docs/design/BLUEPRINT-P47-P50-gap-closing-phases.md` (P48 hub / P49
consumer lanes) · `kernel/src/{messenger.rs, backup.rs, spool.rs, token_bucket.rs,
analytics.rs, pq/entropy.rs}` (§0 rows) · web research 2026-07-18: httpSMS
(github.com/NdoleStudio/httpsms + docs.httpsms.com), Meta WhatsApp Business Platform pricing
+ Cloud API onboarding docs and 2026 BSP-market guides, SimpleX Chat
(github.com/simplex-chat/simplex-chat README + bots/README.md, simplex.chat/docs/cli.html,
simplex.chat/docs/server.html + github.com/simplex-chat/simplexmq, v5.6/v5.7 PQ-ratchet
release notes), Ghost-Downloader-3
(github.com/XiaoYouChR/Ghost-Downloader-3) — each summarized in §0 with the design-relevant
facts inline so this doc stands without re-fetching. Memory files:
`integration-ports-reactive-arc-2026-07-13` (source arc) · `ecosystem-strategy-arc-2026-07-13`
(CLONE/build-your-own-x doctrine, §3.7) · `ops-reliability-arc-2026-07-13` (sovereignty
posture, §1.3) · `rust-native-bare-metal-decision-2026-07-14` (adapter-not-purge, DECART on
new-dep/swap — both applied) · `test-integrity-rules-2026-06-27` (PII red-line awareness,
§4.1) · `anu-ananke-strict-discipline-feedback-2026-07-17` (style discipline applied).
Supersedes: nothing — additive over §10.5.5's index entry; corrects (with citation) the
"SMS is per-message paid via any provider" framing per §1.3.

---

## 8. Hermetic principles honored (item 20 — explicit, per principle)

- **P2 CORRESPONDENCE:** one send seam (`ChannelSend`) for every transactional channel; one
  outbox doctrine (Spool/TokenBucket) shared with P22's lane rather than a second retry
  machine; one content-address convention (sha3, `backup.rs`'s) for every blob consumer.
- **P5 RHYTHM (return-swing honored):** the WhatsApp window is a rhythm the design rides
  rather than fights — free flow while the customer-initiated window is open, typed refusal
  and fallback when it closes; spending to force a send against the rhythm requires an
  explicit configured decision.
- **P7 GENDER (no self-certification):** transmission claims are certified by the far side —
  Telegram's `message_id`, httpSMS's delivery webhook, WA's `wamid` — never by our own code
  reporting success; the live probes exist precisely so the spy tests don't certify
  themselves. SimpleX's strongest form of this: the §5-row-8 round-trip is certified by a
  SECOND, independent client actually receiving the message through a real relay — not by
  the sender's daemon acknowledging it.

(Other principles are not load-bearing here and are not claimed decoratively.)

---

## 9. Standard-compliance map (all 20 points, checkable)

| §2 item | Where satisfied |
|---|---|
| 1 ground truth | §0 (live code rows incl. one found delta — the SECOND deprecated-URL occurrence; web rows marked as researched-this-pass) |
| 2 DoD | §5 (roadmap's 4 rows kept + 4 researched additions) |
| 3 spec/event-driven TDD | §2 types first; §3 per-item RED tests; send lifecycle asserted as spool-row event sequences |
| 4 predefined types/consts | §2 |
| 5 adversarial tests | §3.2-§3.7 incl. §3.4b (24 named adversaries incl. SSRF, forged webhooks, lying Content-Length/Type, mid-stream kills, sidecar version drift, consumed-invitation reuse) |
| 6 hazard-safety as math | §4.1 (closed-enum boundary enforcement, SSRF post-resolution math, fail-closed economic state) |
| 7 links docs/memory | §7 |
| 8 scaling axes | §4.2 (each axis + break point; streaming export named as the first-break) |
| 9 Linux discipline | §4.4 (incl. one honest GAP with a tracking row) |
| 10 benchmarks+telemetry | §6 (2 budgets; existing read surfaces as telemetry) |
| 11 isolation/bulkhead | §4.3 (outbox decoupling, per-channel buckets, one-way dependency arrow) |
| 12 mesh awareness | §4.3 (node-local egress via P34's gate; no mesh payloads) |
| 13 rollback/self-heal vocabulary | §4.3 (Self-Termination bounds + the ONE Snapshot-Re-entry claim, justified by idempotent content-addressing) |
| 14 error-propagation gates | §2 (closed enums), §5 (typed-outcome tests per path), §4.1 (unrepresentability arguments) |
| 15 living memory | §4.3 (Spool + ChannelLedger as the stores; no third channel) |
| 16 tensor/spectral + eqc reuse | N/A-honest: no closed-form math in this phase; no decorative claim |
| 17 regression ledger | §5 (3 rows incl. the open GAP) |
| 18 agent-executable instructions | §10 |
| 19 reuse-first | §2 rejected alternatives; §3.2 (Spool/TokenBucket verbatim); §3.7 (backup.rs pattern; pattern-not-dependency DECART) |
| 20 Hermetic citations | §8 |

---

## 10. Clear instructions for other agentic workers (item 18 — zero session context assumed)

Repo: `/root/dowiz`. **Gate check first:** T2–T8 require DELIVERY P37/P38 live (roadmap
anti-scope). T1 is ungated — land it any time, including today.

1. **T1 (E-a, independent).** Fix BOTH deprecated-endpoint occurrences in
   `kernel/src/pq/entropy.rs` (`:48` and the raw-TLS fallback `:118-122`), aligning to the
   current ANU endpoint per `bebop-repo/bebop2/proto-cap/src/entropy.rs`'s implementation.
   Acceptance: `grep -c "jsonI.php" kernel/src/pq/entropy.rs` → 0; `cd kernel && cargo test
   --features qrng` green (fail-closed test re-pointed).
2. **T2 (E-b port).** Create `kernel/src/ports/notify.rs` + `media_import.rs` (§2 verbatim);
   register both in `ports/mod.rs` (one line each, existing convention). NO new kernel deps
   (`git diff kernel/Cargo.toml` → empty). Acceptance: `cd kernel && cargo test --lib` green.
3. **T3 (E-b adapter).** Create `notify-adapters/` (repo root; deps: kernel path dep for
   ports? NO — mirror the facade discipline: if a facade crate exists by then for the agent
   lane, follow the established pattern; otherwise kernel path dep is acceptable HERE because
   notify-adapters is an edge crate like `llm-adapters`, which imports the kernel ports
   directly — cite `llm-adapters/Cargo.toml` and match it). `ureq` per the §6.3-cited spec.
   Implement `TelegramSender` + Spool/TokenBucket composition. Write the 4 §5-row-2 tests
   (spy double; live probe env-gated on `TG_TEST_BOT_TOKEN`). Acceptance: `cd notify-adapters
   && cargo test` green.
4. **T4 (E-c).** Add `HttpSmsSender` (`base_url` config; `POST /v1/messages/send`,
   `x-api-key`, `{content, from, to}`; §2's `SMS_MAX_PER_HOUR` bucket) + the 4 §5-row-5
   tests. Do NOT build the Twilio-class adapter — it is the named optional alternative,
   built when a real venue without an Android gateway asks. Acceptance: crate tests green.
5. **T5 (E-d).** Add `WhatsAppSender` + the window ledger + webhook signature verification
   (§3.4) + the 5 §5-row-6 tests. The live probe is gated on Meta business verification —
   record its absence as the named external gate, do not fake it. Acceptance: crate tests
   green; the paid path unreachable without `acknowledged_paid` config (grep + test).

   **T5b (E-h, after T3 — may run parallel to T4/T5).** Add `SimpleXSender` per §3.4b:
   minimal hand-rolled loopback RFC-6455 client (DECART's chosen row; `simploxide-client`
   only via a recorded new-dep DECART), startup version-pin assertion against the sidecar,
   one-time-invitation mint + connection→order binding, send via the WS `{corrId, cmd}`
   protocol with exact command strings pinned against the INSTALLED CLI's generated bot-API
   reference (do not trust this doc for command syntax — it is version-coupled by design).
   Write the 7 §5-row-8 tests; `simplex_roundtrip_real_cli` boots a LOCAL `smp-server` +
   two CLI instances (binaries-gated like the Ollama live tests — skip, never fake).
   Do NOT deploy a public SMP relay (optional P45 ops item, §3.4b verdict); do NOT touch
   Telegram/WhatsApp code paths — additional channel only. Acceptance: crate tests green;
   `ws_loopback_only` ledger row (d) added.
6. **T6 (E-e).** In `web/src`: read `?ch=` (closed vocabulary → `direct` fallback), thread
   through order placement into the event attribution slot; E2E
   `channel_attribution_roundtrip`. Acceptance: E2E green; `orders_by_channel` shows the
   count with zero `analytics.rs` changes.
7. **T7 (E-f).** Export endpoints on P37's authenticated surface (owner cert); CSV
   (std-only, injection-guarded) + JSON. The 3 §5-row-4 tests. Acceptance: live-deployment
   download parses and counts match.
8. **T8 (E-g).** Create `media-adapters/` (or a module in the same edge-crate family if
   review prefers one integration crate — either way ONE home): the §3.7 pipeline (SSRF
   guard → streaming capped GET → magic-sniff → chunker → `BlobSink`), `FsBlobStore` as the
   first sink impl following `backup.rs`'s content-addressed layout. The 6 §5-row-7 tests
   against a local fixture HTTP server. Acceptance: `cargo test` green incl. all four
   adversaries; NO image-decode dependency appears in `Cargo.toml` (§1.4-7).
9. **T9 (close-out).** Run all crate suites + kernel `--lib`; add the 3 §5 ledger rows;
   verify every §5 row RED→GREEN evidence exists. Do not mark P43 done if any adversarial
   test was weakened or `#[ignore]`d. Hand-offs: P49 DoD-3 can now go green against E-b's
   transmitter; P22's campaign lane may lift the shared httpSMS low-level adapter (§1.2)
   when its consent-ledger precondition clears; P48's storage decision replaces
   `FsBlobStore` behind `BlobSink` with zero importer changes.
