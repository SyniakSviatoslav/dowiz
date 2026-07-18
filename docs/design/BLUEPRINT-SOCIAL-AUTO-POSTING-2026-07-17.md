# BLUEPRINT — Social Auto-Posting: `SocialPoster` port + per-platform adapters (2026-07-17)

> **Origin: a real client request, not a thought experiment.** Restaurant/venue owners on dowiz asked
> to write a post once (daily special: photo + caption) and have it publish to their own social
> surfaces simultaneously — Instagram, Facebook Page, Telegram channel, Viber channel, TikTok, etc.
> This document is the research + execution blueprint for that feature. **Planning only — no code,
> CI config, or canon file is edited by this pass.**
>
> Pattern mirrored: the `LlmBackend` port (`kernel/src/ports/llm.rs`) + `llm-adapters/` crate —
> the repo's established seam for anything external. Protocol followed: `AGENTS.md` Detailed
> Planning Protocol (ground-truth-first, DECART inline, falsifiable done-checks, 2-question doubt
> audit, wave-sequenced dependencies).
>
> Canon alignment: **M5** (every hub may open ports/bridges at its discretion — platform choice is
> per-venue config, never a kernel recompile) and **M6** (zero external deps at the trust boundary —
> the kernel gains a trait and plain structs only; all HTTP/serde lives in the adapter crate).
> Social posting is an **optional adapter ring**, degrade-closed: if every platform is down or
> unconfigured, dowiz itself loses nothing.

---

## 0. Executive summary

- **One new port**: `kernel/src/ports/social.rs` — `SocialPoster` trait + plain value types
  (`PostDraft`, `PostCaps`, `PostReceipt`, `SocialError`). Zero HTTP/JSON/serde in the kernel,
  exactly like `ports/llm.rs`'s compile firewall (`llm.rs:1-11`).
- **One new sibling crate**: `social-adapters/` (repo root, standalone — no workspace exists,
  verified; same shape as `llm-adapters/`). `ureq` + `serde`/`serde_json` only. One thin
  `HttpTransport`, per-adapter `SocialQuirks`, per-platform adapter files.
- **Fan-out pipeline reuses existing kernel primitives verbatim**: `Spool` (crash-safe outbox,
  claim/ack/reclaim) for delivery + retry, `TokenBucket` for per-platform rate budgets,
  `analytics::ChannelLedger` to close the measurement loop (post → `?ch=` link → order →
  attribution), telemetry ledger per the AGENTS.md native-telemetry mandate.
- **First wave = Telegram** (zero approval, per-venue bot token, 10 MB photos, stable-for-a-decade
  API). **Second wave = Viber Channels + Meta (Instagram + Facebook Page)** — Meta's business
  verification + App Review is a calendar-gated external process that starts at Wave 0 but lands
  when Meta says so. **Deferred behind named triggers**: TikTok (audit wall), X (per-post pricing),
  Google Business Profile (vendor approval form). **WhatsApp is explicitly re-scoped out**: it has
  no feed; "posting" there is opt-in customer messaging = the existing IP-15 `ChannelAdapter`
  design's territory, not this port's.
- **Extended 2026-07-18 (§11, operator directive):** content generation (native-template
  zero-AI path + `LlmBackend`-drafted path, one `MasterPost` output type), manual-approval
  DEFAULT with opt-in earned-autonomy agentic posting (P40 `ToolPort` seam, P42-gated), and the
  channel-breadth map — mailing lists + SMS ride the IP-15 `ChannelAdapter` campaign lane under
  this phase's number, never the `SocialPoster` trait; transactional sends stay P43/P49.
- **Extended 2026-07-18 (§12, operator directive): YouTube** joins as **Wave 2-Y** — a Shorts
  *relay* of owner-shot vertical clips via `videos.insert` (quota restructured per official
  2026-06-01 docs: dedicated bucket, 100 uploads/day per API project, fleet-shared). YouTube
  **Community posts have no write API** — never offered (IP-15 honesty); `compose()`-derived
  auto-Shorts **deferred behind TRIGGER-YT-AUTOGEN** (no video encoder exists in-repo and none
  is added). Instagram + Facebook remain fully covered by the existing Meta Wave 2 lane.
- **Biggest single risk found** (see §8, §9): the Meta lane concentrates every client behind
  dowiz's *one* reviewed app — one spam flag / failed re-review suspends posting for *all* venues
  at once. Telegram/Viber use per-venue tokens with no shared app identity and have no such choke
  point. This is why the resilient per-venue-sovereign lane ships first and Meta is additive.

---

## 1. Ground truth (live-verified 2026-07-17, file:line)

### 1.1 The pattern being mirrored — verified against source

- **Port trait**: `kernel/src/ports/llm.rs:154-169` — `trait LlmBackend { id(); caps(); chat();
  embed(); rerank(); health(); }`. Header doc (`llm.rs:1-11`): *"ZERO network / HTTP / JSON /
  serde… the concrete adapter crate owns all HTTP/JSON"* and *"Per M5: backend choice is
  configuration on the consumer side, never a kernel recompile."* Registered via one line in
  `kernel/src/ports/mod.rs:6` (`pub mod llm;`); `pub mod ports;` at `kernel/src/lib.rs:115`.
- **Fail-closed capability struct**: `llm.rs:17-23` `Caps` — "a capability the backend does not
  expose is `false`; the caller must NOT assume presence."
- **Typed error enum, never a mock**: `llm.rs:140-150` `LlmError { Unavailable, Unsupported,
  BadRequest(String), Timeout }`.
- **Adapter crate**: `llm-adapters/Cargo.toml` — standalone crate at repo root ("NOT a workspace
  member" per its own comment; `find -name Cargo.toml` confirms five standalone crates, zero
  workspaces), deps exactly `dowiz-kernel` (path), `ureq = { version = "2", default-features =
  false, features = ["tls", "json"] }`, `serde`, `serde_json`; criterion as dev-dep with
  `[[bench]] harness = false`.
- **Quirks pattern**: `llm-adapters/src/quirks.rs:11-28` — one plain struct of per-adapter wire
  deltas; constructors `Quirks::ollama()/vllm()/managed_api()`. Transport "holds no vendor
  knowledge itself" (`quirks.rs:10`).
- **Adapter shape**: `llm-adapters/src/ollama.rs:16-81` — thin struct wrapping the transport,
  `caps()` hard-codes only what is live-verified, unsupported capability returns
  `Err(Unsupported)` ("fail closed, caller falls back", `ollama.rs:73-76`).
- **ureq precedent is a settled DECART**: `HARNESS-LLM-BACKEND.md` §5 Decision 2 — *"the exact
  spec already vetted twice in this repo"* under the 2026-07-15 operator mandate (rustls+ring, no
  tokio). This blueprint cites that decision rather than re-litigating it (§6.3).

### 1.2 Reusable kernel primitives (verified present on this branch)

- **`kernel/src/spool.rs`** — pure crash-safe work-queue state machine: `append / claim_next /
  ack / reclaim / compact_drop`, backpressure watermark, "the kernel-native async channel reused by
  every subsystem (reporting, governance events, mesh sync), not just Telegram" (`spool.rs:17-20`).
  This IS the outbox for post fan-out; nothing new to invent.
- **`kernel/src/token_bucket.rs`** — `TokenBucket::new(capacity, refill_rate)`, `try_acquire(n)`,
  `available()`. Per-platform posting budgets map directly onto it. (Note: no `release()` method
  exists on this branch — memory's "B3 TokenBucket::release" landed on the mesh branch, not here;
  do not cite it.)
- **`kernel/src/messenger.rs`** — deep-link builders for TG/WA/Viber (`telegram_link`,
  `whatsapp_link`, `viber_link`) + `encode_query`. Explicitly "*contact/link construction only —
  it never sends*" (`messenger.rs:7-8`). The new port is the "sends" half that was deliberately
  left out; the link builders are reused for CTA links inside captions.
- **`kernel/src/analytics.rs`** — `ChannelLedger` ("closes the open attribution measurement
  loop", `analytics.rs:1-19`) already counts orders per acquisition channel with channel strings
  like `"tiktok"`/`"instagram"` in its own tests (`analytics.rs:178-262`). Posts published by this
  feature carry `?ch=<platform>` links → orders attribute → **the same daily special post becomes
  measurable in revenue**, not just impressions. This is the feature's honest ROI loop.
- **`kernel/src/telemetry.rs`** + the harvest-ledger pattern (`AGENTS.md` telemetry mandate):
  every dispatch appends a deterministic record; extend, don't fork.

### 1.3 Prior design art — what already covers what (so this blueprint doesn't duplicate it)

Verified in `docs/design/integration-ports/BLUEPRINTS-INTEGRATION-PORTS.md`:

- **IP-10 Notify crate** (event→outbox→fan-out, retry exp-backoff, DLQ "surfaced-in-UI-never-
  silently-dropped", `idem_key=H(event|channel|recipient)`) — designed for *order-event
  notifications*. This blueprint reuses its outbox/DLQ/idempotency doctrine but is a different
  producer (owner-authored marketing content, not kernel events).
- **IP-15 Port F Messaging** (`ChannelAdapter`, order-status push + OTP **to customers**) — the
  adapter-honesty rule stated there is adopted verbatim here: *"scope advertises per-channel
  capability → UI never offers 'push on Instagram/TikTok' (API-impossible)."* Same honesty, new
  direction: the UI must never offer "post to WhatsApp feed" (no such API exists — §2.6).
- **IP-14 Port B** (conversion upload to Meta CAPI/TikTok Events) and **IP-16 Port G**
  (menu→Meta catalog sync) — sibling *marketing* ports, distinct from content publishing; they
  share the future Meta app + tokens, which is an argument for one Meta token store (§4.5).
- **Roadmap adjacency checked**: `BLUEPRINT-P13` (delivery on protocol) — zero social/notify
  content (grep verified). `BLUEPRINT-P16` (product UI rebuild) — where the owner-facing composer
  UI must eventually live. `BLUEPRINT-P19` (growth engine) — positioning/grants/pricing only;
  **nothing anywhere in the 19 phases covers venue-owned social publishing.** This is genuinely
  new scope, slotting cleanly beside P16 (surface) and P19 (growth spirit) without colliding.

### 1.4 What does NOT exist today (honest gaps this plan must carry)

1. **No owner-facing web UI** — `apps/web`/`packages/ui` are fully deleted (master roadmap §1.2).
   OAuth redirect flows and a post-composer UI depend on Phase 16. **Interim path (Wave 0-1):
   token-paste config** (Telegram bot token, Viber channel token are copy-paste strings — no OAuth
   needed), so the feature is usable by pilot clients before P16 lands.
2. **No public media-hosting surface is decided.** Meta ingests images **by public URL only**
   (`image_url`), TikTok only via `PULL_FROM_URL` from a *domain-verified* URL prefix, Viber takes
   a public JPEG URL. Only Telegram accepts direct multipart upload. A public, stable image URL
   host is a hard precondition for Wave 1+ (§6.2, operator item O-SOC-1).
3. **No secrets-at-rest story for third-party tokens.** Known repo-wide issue (memory:
   secrets-on-disk). This plan stores per-venue tokens in one file with `0600` perms and names
   at-rest encryption (IP-11's XChaCha20 posture) as a follow-up — it does not pretend to solve it.
4. **No Meta developer app exists** for dowiz. Business verification + App Review is an external,
   unschedulable dependency (§2) — the single longest lead-time item in the whole plan, which is
   why §7 starts it at Wave 0 even though its code lands at Wave 2.

---

## 2. Platform research — real constraints (web-verified 2026-07-17, two independent passes)

Full per-claim citations live in the research passes' source lists (Meta developer docs, Telegram
Bot API docs, TikTok developer docs, Viber developer docs, X pricing docs — fetched live).
Claims that could not be verified against a live source are marked **UNVERIFIED** and carried into
§9's doubt audit rather than rounded away.

### 2.1 Telegram Bot API — LOW friction (near zero) ✅ Wave 0

- `POST https://api.telegram.org/bot<token>/sendPhoto|sendMessage|sendMediaGroup`, JSON or
  multipart; `chat_id` accepts `@channelusername`; bot must be channel admin with post rights.
- Photos ≤ 10 MB (multipart) · captions 1-4096 chars · albums 2-10 items · plain HTTPS, **no
  approval process exists** — a static BotFather token per venue.
- Rate limits (official FAQ): ~30 msg/s global, ≤1 msg/s per chat — orders of magnitude above a
  restaurant's 1-3 posts/day.
- Stability: additive-only API since 2015. ToS risk for owner-content-to-own-channel: none.
- **Per-venue sovereignty: total.** Each venue's bot token is its own; no shared dowiz app
  identity exists to be banned.

### 2.2 Viber — split verdict; **Channels Post API yes, Bot API no** ✅ Wave 1 (UA market)

- **Channels Post API** (`POST https://chatapi.viber.com/pa/post`, `auth_token` from the
  channel's in-app Developer Tools screen): no documented fees, JSON, posts appear as the
  channel. Constraints: text ≤ 7000 chars; **picture ≤ 1 MB, JPEG only** (real transcode work —
  §4.4); video ≤ 50 MB MP4/H264; a webhook with CA-signed SSL is mandatory before posting; token
  is dug out of the app UI by the channel super-admin (awkward but per-venue-sovereign, like
  Telegram). Rate limits: only error 12 `tooManyRequests` documented, numbers UNVERIFIED.
- **Bot API for broadcast: rejected.** Since 2024-02-05 new bots need a Viber contract +
  **EUR 100/month per bot** + per-message fees (exact current rate card requires contacting
  Viber). A non-starter for multi-tenant broadcast; the Channels API covers the actual use case.
- Risk: Viber has already rug-pulled bot pricing once; "Channels API stays free" is
  **UNVERIFIED forward-looking** — priced in by keeping the adapter optional/degrade-closed.

### 2.3 Meta — Instagram + Facebook Pages — MEDIUM-HIGH friction (gatekeeping, not tech) ⏳ Wave 2 code, Wave 0 paperwork

- **Plain HTTPS+JSON suffices everywhere** (Graph API v25.0). No SDK needed — the constraint the
  operator directive cares about is satisfied.
- **Instagram**: container flow `POST /<IG_ID>/media` (with public `image_url`; carousel =
  child containers) → `POST /<IG_ID>/media_publish`. Two login variants; the newer
  "Instagram API with Instagram Login" (`graph.instagram.com`) needs **no linked Facebook Page**.
  Tokens: 60-day long-lived, refreshable via `refresh_access_token` (≥24h old, unexpired) —
  **needs a refresh cron**; an expired token = owner must re-connect.
- **Facebook Pages**: `POST /{page_id}/photos` (`url` param) / `POST /{page_id}/feed`; native
  scheduling 10 min-30 days out. **Long-lived Page tokens have no time-based expiration** —
  operationally the best Meta surface.
- **Approval friction (the real cost)**: `instagram_content_publish` / `pages_manage_posts` (+
  `pages_read_engagement`, `pages_show_list`) all require **App Review for Advanced Access** with
  use-case screencasts, plus **Business Verification of dowiz itself** (mandatory since 2023).
  Meta publishes **no review SLA** (commonly days-to-weeks, UNVERIFIED). Until Advanced Access:
  works only for accounts holding a role on the app — fine for the operator's own pilot venue,
  useless multi-tenant. → Start the paperwork at Wave 0.
- **Rate limits**: IG API-publishing cap — docs currently state **100 posts/24h moving window**
  per IG account, but the same page elsewhere still says 50 (**internally inconsistent docs**);
  runtime check exists (`GET /<IG_USER_ID>/content_publishing_limit`) and the adapter must use it
  instead of trusting either number. Pages: BUC formula (4800 × engaged users)/24h. Both dwarf
  1-3 posts/day.
- **ToS/churn (explicit, load-bearing)**: Meta's spam policy flags posting *"either manually or
  automatically, at very high frequencies"* and *repetitive content* even at low frequency —
  a template-driven "same daily special across N venues" engine is precisely that fingerprint
  (§8 mitigation: per-venue distinct content, owner-authored, low cadence). Version treadmill: a
  new Graph version every ~3-4 months, each supported ≥2 years, **expired-version calls are
  silently forwarded** (breaks behavior without an error — pin + re-verify quarterly). Recent
  hard kills: Basic Display API (2024-12), legacy IG scopes (2025-01), WhatsApp On-Prem (2025-10).

### 2.4 TikTok Content Posting API — HIGH friction 🔒 deferred behind a named trigger

- Photo posts exist (`POST /v2/post/publish/content/init/`, ≤35 images, direct or draft), but:
  **unaudited clients can post SELF_ONLY (private), max 5 users/24h** — i.e. useless in
  production until a **2-4 week multi-round audit** passes, which also mandates specific UX
  (preview, privacy dropdown with no default, commercial-content toggles). Photos ingest only via
  `PULL_FROM_URL` from a **domain-verified** URL prefix. 6 req/min per user token.
- Ukraine availability for creators/developers: **UNVERIFIED** (no restriction stated in docs; no
  positive confirmation either).
- **TRIGGER-TIKTOK** (grep-able): begin the TikTok adapter + audit submission only when ≥3 paying
  venues explicitly request TikTok posting AND Wave 1 is live (the audit demands a working
  reviewable product anyway). Until then: `caps()` for TikTok simply doesn't exist — the UI never
  offers it (IP-15 honesty rule).

### 2.5 X (Twitter) API v2 — economically marginal 🔒 deferred behind a named trigger

- Feb 2026 model: pay-per-use, **$0.015/post, $0.200/post containing a URL** (13× link
  surcharge); 100 posts/15min/user. No approval wall, worst-in-class stability record (four
  breaking business-model changes in three years).
- A daily special *with a menu link* ≈ $6/venue/month paid to X — viable only link-less, and X
  has minimal UA-restaurant reach. **TRIGGER-X**: first paying venue that explicitly asks and
  accepts the per-post cost pass-through.

### 2.6 WhatsApp — **re-scoped out of this port** (honesty finding)

- **WhatsApp has no feed and no public Channels/Status API** (verified against the WhatsApp
  Business Platform docs index — Cloud API messaging/templates only; Channels remain app-only).
  The realistic equivalent — marketing template broadcasts to opted-in customers inside tiered
  limits (250→2k→10k→∞ unique recipients/24h), per-message billing since 2025-07, template
  review, quality-score throttling — is **customer messaging (CRM), not publishing**, and belongs
  to the existing IP-15 `ChannelAdapter` design, driven by opt-in lists and the 24h service
  window. Wiring it into a "post everywhere" button would train clients to spam-blast and decay
  their WhatsApp quality tier. The composer UI may *offer a handoff* ("also send as WhatsApp
  campaign") later, but via IP-15, never via `SocialPoster`.

### 2.7 Google Business Profile (localPosts) — MEDIUM, genuinely useful, second-tier

- `POST .../v4/accounts/{a}/locations/{l}/localPosts` (OFFER/EVENT/CTA types with photo). Gated
  by a one-time vendor application (0 QPM until approved, 300 QPM after; profile 60+ days old).
  Real value for restaurants (shows in Search/Maps). Long-term survival of the v4 API:
  UNVERIFIED (years-long slow deprecation, localPosts never migrated). Slot: Wave 2 alongside
  Meta paperwork — same "fill in a form, wait" lane, cheap adapter.

---

## 3. The `SocialPoster` port — `kernel/src/ports/social.rs`

Mirrors `ports/llm.rs` exactly: plain structs, zero HTTP/JSON/serde, compile firewall preserved
(`cargo tree -p dowiz-kernel` must show no HTTP client after implementation — same done-check as
the LLM port's WAVE-0). Registered as `pub mod social;` in `ports/mod.rs` (one line).

```rust
/// Fail-closed capability discovery per platform (mirror of llm::Caps, llm.rs:17-23).
/// A capability the platform does not expose is `false`/`0`; the caller must NOT assume presence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PostCaps {
    pub text: bool,
    pub photo: bool,
    pub video: bool,
    pub album: bool,                 // multi-photo single post (TG sendMediaGroup, IG carousel)
    pub max_caption_chars: u32,      // TG 4096 (1024 w/ media), Viber 7000, IG 2200
    pub max_media_per_post: u8,      // TG 10, IG 10, Viber 1
    pub max_photo_bytes: u64,        // TG 10 MB, Viber 1 MB — drives transcode decisions
    pub jpeg_only: bool,             // Viber: true
    pub media_by_url: bool,          // platform ingests from a public URL (Meta, Viber, TikTok)
    pub media_by_upload: bool,       // platform accepts direct multipart bytes (Telegram)
    pub links_clickable: bool,       // IG captions: false (links don't render) — honest signal to UI
    pub native_schedule: bool,       // FB Pages: true; others: false (we schedule via the outbox)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind { Photo, Video }

/// One media item. Both locators are optional; preflight() checks the one the platform's
/// caps require is present (media_by_url ⇒ public_url, media_by_upload ⇒ local_path).
#[derive(Debug, Clone)]
pub struct MediaItem {
    pub kind: MediaKind,
    pub public_url: Option<String>,
    pub local_path: Option<String>,
    pub bytes_len: u64,
    pub mime: String,
    pub alt_text: String,
}

/// One post as authored by the venue owner, already adapted for ONE platform.
#[derive(Debug, Clone)]
pub struct PostDraft {
    pub venue_id: String,
    pub caption: String,
    pub media: Vec<MediaItem>,
    pub link: Option<String>,        // CTA link; carries ?ch=<platform> for ChannelLedger attribution
    pub idem_key: String,            // sha3(venue_id | platform | canonical content | date-bucket)
}

#[derive(Debug, Clone)]
pub struct PostReceipt {
    pub platform: String,            // adapter id, e.g. "telegram"
    pub post_id: String,             // platform-native id (message_id / IG media id / post token)
    pub permalink: Option<String>,
    pub posted_at_ms: i64,
}

/// Typed error — the retry taxonomy is STRUCTURAL (P4 polarity: named poles, safe-directed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SocialError {
    /// Endpoint unreachable / 5xx. RETRYABLE (backoff).
    Unavailable,
    /// Capability absent on this platform (caps() said so, or platform refused the type). TERMINAL.
    Unsupported,
    /// Token expired/revoked. TERMINAL until the owner re-connects — never auto-retried,
    /// surfaced in the owner UI (IP-10: DLQ never silently dropped).
    AuthExpired,
    /// Platform throttle. RETRYABLE after the given delay (parsed from Retry-After / error body).
    RateLimited { retry_after_s: u64 },
    /// Platform rejected the CONTENT (policy/moderation/format). TERMINAL — a poison message;
    /// retrying identical content is exactly the spam fingerprint §2.3 warns about.
    Rejected(String),
    /// Malformed draft (caption too long, media missing/oversized). TERMINAL, caught by preflight.
    BadRequest(String),
    /// Transport deadline. RETRYABLE — but publish MUST be idempotency-guarded (§5.3).
    Timeout,
}

/// The pluggable social-publishing port. Implemented by `TelegramAdapter`, `ViberChannelAdapter`,
/// `FacebookPageAdapter`, `InstagramAdapter`, … in the `social-adapters` crate.
pub trait SocialPoster {
    /// Stable adapter id, e.g. "telegram" — used in telemetry rows, idem keys, ?ch= tags.
    fn id(&self) -> &str;
    /// Fail-closed capability discovery (static per platform, from its Quirks).
    fn caps(&self) -> PostCaps;
    /// PURE validation of a draft against caps (length/media count/size/format/locator kind).
    /// No network. Deterministic. This is the half a table-driven kernel test can exercise
    /// for every adapter identically (the parity pin P2 requires for divergent wire code).
    fn preflight(&self, draft: &PostDraft) -> Result<(), SocialError>;
    /// Publish. `Err` on any failure — never a fabricated receipt.
    fn publish(&self, draft: &PostDraft) -> Result<PostReceipt, SocialError>;
    /// Token-validity / reachability probe (TG getMe, Meta /me debug_token, Viber get_info…).
    /// Never fabricates liveness (mirror of llm.rs:166-168).
    fn health(&self) -> Result<(), SocialError>;
}
```

Deliberate deltas from `LlmBackend`, named (P2: divergence must be stated, not silent):

1. **`preflight()` has no LLM analog.** Social platforms reject at content-shape level
   (caption length, 1 MB JPEG) where LLMs don't; catching this before the outbox prevents poison
   messages from ever being enqueued. It is pure, so it doubles as the cross-adapter parity test.
2. **`RateLimited`/`AuthExpired`/`Rejected` variants have no LLM analog** — they encode the
   retry taxonomy the outbox drainer dispatches on (§5.2). `LlmError` has no retry semantics
   because the harness Dispatcher owns those; here the taxonomy IS the contract.
3. **No `TaskClass` routing** — there is no within-adapter model choice to route.

## 4. The `social-adapters/` crate

```
social-adapters/                  # repo root, standalone (sibling of llm-adapters/ — §1.1)
  Cargo.toml                      # dowiz-kernel (path), ureq 2 (default-features=false, tls+json),
                                  # serde + derive, serde_json; criterion dev-dep, [[bench]] harness=false
  src/
    lib.rs                        # pub mod + re-exports, mirror of llm-adapters/src/lib.rs
    quirks.rs                     # SocialQuirks + per-platform constructors (§4.1)
    transport.rs                  # HttpTransport: thin ureq wrapper (§4.2)
    multipart.rs                  # hand-rolled multipart/form-data encoder, std-only (§4.3, DECART §6.4)
    telegram.rs                   # TelegramAdapter        — Wave 0
    viber.rs                      # ViberChannelAdapter    — Wave 1
    facebook.rs                   # FacebookPageAdapter    — Wave 2 (code parallel-safe earlier)
    instagram.rs                  # InstagramAdapter       — Wave 2 (container flow: 2 calls + poll)
    outbox.rs                     # fan-out drainer: kernel Spool + per-platform TokenBucket (§5)
    tokens.rs                     # per-venue token store + IG 60-day refresh schedule (§4.5)
    telemetry.rs                  # publish-outcome ledger rows (AGENTS.md mandate)
```

### 4.1 `SocialQuirks` — same role as `llm_adapters::Quirks`, different axes

```rust
pub struct SocialQuirks {
    pub base_url: String,               // https://api.telegram.org / graph.facebook.com/v25.0 / chatapi.viber.com
    pub caps: PostCaps,                 // the static capability truth for this platform (§3)
    pub auth_style: AuthStyle,          // TokenInPath (TG) | BearerHeader | HeaderXViberToken | QueryParam (Meta access_token)
    pub api_version_pin: Option<String>,// Meta: "v25.0" — pinned, bumped only by a deliberate edit (§2.3 silent-forwarding hazard)
    pub rate_capacity: u64,             // TokenBucket capacity for this platform
    pub rate_refill_per_s: f64,         //   … and refill (TG: 1/s per chat; Meta: generous; Viber: conservative-unknown)
    pub publish_is_two_phase: bool,     // Instagram container flow: create → publish (+ status poll)
}
```

Constructors: `SocialQuirks::telegram()`, `::viber_channel()`, `::facebook_page()`,
`::instagram()` — each documenting its numbers with the §2 citation, exactly as
`Quirks::ollama()` documents its live-probe provenance (`quirks.rs:44-54`).

### 4.2 Transport — honest divergence from `OpenAiCompatTransport`, stated

The LLM adapters share one transport because every backend speaks the *same* OpenAI wire shape
(`transport.rs:1-4`). Social platforms share **no** wire shape — so `HttpTransport` here is
thinner: it owns `ureq` calls (JSON POST, multipart POST, GET), auth-header/query injection from
`SocialQuirks.auth_style`, HTTP-status→`SocialError` mapping (429 + `Retry-After` →
`RateLimited`; 401/403 token-class errors → `AuthExpired`; 5xx → `Unavailable`), and timeouts.
**Endpoint paths and body shaping live in each adapter** — that is vendor knowledge, and each
platform's is unique. The P2 parity pin for this permitted divergence is the shared table-driven
`preflight` conformance test in the kernel (one test, all adapters) plus one shared
`error_mapping` test in the transport (all adapters inherit it).

### 4.3 Telegram media path (Wave 0): multipart, no public host needed

`sendPhoto` accepts multipart bytes directly (≤10 MB) — Wave 0 therefore has **zero dependency on
the unresolved public-media-host question** (§1.4.2). `multipart.rs` is a ~40-line std-only
encoder (boundary, `Content-Disposition` parts, byte body) — DECART'd in §6.4.

### 4.4 Viber image constraint (Wave 1): 1 MB JPEG transcode

Viber posts ingest a public URL, ≤1 MB, JPEG only (§2.2). Food photos are routinely 3-8 MB.
Options: (a) require the composer/UI to produce a ≤1 MB JPEG rendition at upload time (P16's
media pipeline — where image renditions belong anyway); (b) a Rust image-transcode dep in
`social-adapters`. **This blueprint chooses (a)** — renditions are a media-pipeline concern, made
once, used by every ByUrl platform; adding an image codec dep to the adapter crate would need a
DECART it would likely lose against "the pipeline already resizes for the menu UI." Named
consequence: the Viber adapter's `preflight` rejects drafts whose photo rendition exceeds caps —
it never silently transcodes.

### 4.5 Token store (`tokens.rs`) — per-venue, file-backed, honest about encryption

`{venue_id → {platform → TokenRecord{secret, obtained_at, expires_at?, kind}}}` in one JSON file
under the existing runtime-config directory, `0600`. IG records carry `expires_at`; a
`refresh_due()` scan feeds the drainer's housekeeping pass (refresh at age ≥30 d — well inside
the ≥24h-old/unexpired window §2.3 requires). **At-rest encryption is a named follow-up**
(IP-11's XChaCha20-Poly1305 posture), not claimed here — consistent with the repo's known
secrets-on-disk debt rather than silently worsening or silently "solving" it. Red-line note:
these are *venue marketing tokens* — no dowiz money/auth/RLS surface is touched by this feature.

## 5. Fan-out pipeline (reuses Spool + TokenBucket + ChannelLedger — nothing new invented)

### 5.1 Flow

```
owner authors ONE master post (caption, media, link)  [P16 composer; Wave 0-1: CLI/config file]
  └─ for each platform ENABLED in the venue's config (M5: per-venue choice, no recompile):
       adapt: build per-platform PostDraft (caption within caps, ?ch=<platform> on the link,
              idem_key = sha3(venue|platform|content|date))          [pure kernel fn, testable]
       preflight: adapter.preflight(&draft)  → reject now = composer feedback, nothing enqueued
       enqueue: Spool.append(serialized draft)                        [spool.rs — crash-safe]
  drainer (cron/systemd timer binary, same convention as existing repo cronjobs):
       Spool.claim_next → TokenBucket[platform].try_acquire(1)
         → adapter.publish(&draft)
             Ok(receipt)                → record receipt row (telemetry + sent-ledger) → Spool.ack
             Err(RateLimited{s})        → Spool.reclaim, next_attempt = now + s (backoff)
             Err(Unavailable|Timeout)   → Spool.reclaim, exp-backoff + jitter, max N attempts → DEAD
             Err(AuthExpired|Rejected|
                 Unsupported|BadRequest)→ Spool.ack + DEAD row surfaced in owner UI (never silent,
                                          never retried — IP-10 DLQ doctrine verbatim)
```

### 5.2 Retry discipline is typed, not conventional

The drainer dispatches **only** on the `SocialError` variant — there is no per-adapter retry
opinion (P4: one mechanism, named poles). Poison content (`Rejected`) is never retried: repeating
rejected content is the documented Meta spam fingerprint (§2.3).

### 5.3 Idempotency without server support

None of these platforms offers idempotency keys. Dedupe is client-side and structural: the
drainer writes the receipt to a **sent-ledger keyed by `idem_key` *before* `Spool.ack`**; on
crash-replay, `claim_next` → sent-ledger hit → ack without re-posting. Worst case (crash between
platform accept and ledger write) = one duplicate post — the failure window is one write, and the
done-check for Wave 0 exercises exactly this seam (§7).

### 5.4 The measurement loop closes

Every published link carries `?ch=<platform>`; orders arriving through it flow into
`ChannelLedger` (`analytics.rs`) — `orders_by_channel()` / `funnel(channel)` then answer *"did
Tuesday's Telegram special sell?"* — the attribution reader that the analytics module was built
for, now with a producer feeding it. Post-level telemetry rows (`telemetry.rs`:
`{platform, venue, outcome, latency_ms, attempts}`) satisfy the AGENTS.md native-telemetry
mandate; criterion benches cover the pure hot paths (`preflight`, caption adaptation,
`idem_key`); live posting probes are pass/fail CI probes, **not** committed baselines (same
host-noise ceiling the AGENTS.md `innovate:` note already sets for live LLM benches).

---

## 6. DECART decisions (filed before implementation, per the Integration Decart Rule)

### 6.1 Decision — First-wave platform set

| Criterion | **Telegram-first, then Viber+Meta (chosen)** | Meta-first | All-at-once (5+ adapters) |
|---|---|---|---|
| Bare-metal / Rust-native fit | Plain HTTPS+JSON, multipart from std — zero SDK anywhere | Same (Graph API is plain REST) | TikTok forces domain-verified CDN + mandated UX widgets |
| Falsifiable correctness | Live-probe-able TODAY with one BotFather token; done-check = real post in a real channel | Nothing testable multi-tenant until App Review passes (unschedulable) | Untestable: TikTok unaudited = SELF_ONLY private posts |
| Time-to-first-client-value | Days (no approval process exists) | Weeks-to-months (Business Verification + App Review, no SLA) | Months, gated on 3 external review processes |
| Supply-chain / gatekeeping | Per-venue tokens, no shared app identity — no central choke point | ALL venues behind dowiz's ONE reviewed app (§8 SPOF) | Maximal accumulated gatekeeping |
| Market fit (UA venues) | Telegram is the #1 UA channel medium; Viber #2 | IG strong for food discovery, but reach ≠ owned-channel reach | X/TikTok marginal for UA restaurant dailies |
| Reversibility-as-port | Each platform = one `SocialPoster` impl behind per-venue config; dropping one deletes one file | Same | Same |
| Evidence | §2.1-2.2 (verified live) | §2.3 (verified live; review SLA UNVERIFIED) | §2.4-2.5 |

**DECISION:** Wave 0 = Telegram. Wave 1 = Viber Channels (+ the pure fan-out spine). Wave 2 =
Instagram + Facebook (code is parallel-safe earlier; *activation* gates on Meta review, whose
paperwork starts at Wave 0). TikTok/X/GBP behind named grep-able triggers (§2.4, §2.5, §2.7).
Falsifiable reason: Telegram is the only platform where a real multi-tenant post can be published
and verified **this week** with zero external approval, and it carries no shared-app ban risk.

**Probe (honest case against):** the clients asked for "Instagram, Facebook, TikTok…" —
Telegram-first risks shipping the platform the *tool* finds easiest rather than the one clients
value most. Answer: Meta paperwork starts day 0 in parallel (the long pole is the review queue,
not our code — Telegram-first costs Meta *zero* calendar days), and IG/FB adapters can be
live-tested single-tenant (operator's own venue holds a role on the app) before Advanced Access.

### 6.2 Decision — media ingestion for ByUrl platforms (O-SOC-1: needs an operator ruling)

| Criterion | Existing public app host (Fly) serves renditions | R2/CDN bucket | Per-venue self-hosted URL |
|---|---|---|---|
| Works for Meta `image_url` | Yes | Yes | Yes but fragile (venue uptime) |
| Works for TikTok `PULL_FROM_URL` | Only if dowiz domain is TikTok-verified | Same requirement | Unrealistic (per-venue domain verification) |
| Repo reality | Fly deploy exists today | **Off-Hetzner R2 was previously blocked** (docker-swap arc) — unresolved | Contradicts "it just works" for owners |
| Sovereignty posture | Canonical-operator convenience, venue may self-host later (M5) | Same | Purest but impractical now |

**DECISION: deferred to the operator as O-SOC-1**, with a flagged-overridable recommendation:
serve media renditions from the existing public app host under a stable path. This blueprint's
Wave 0 is deliberately independent of the ruling (Telegram = multipart). **Wave 1's Viber adapter
is the first thing blocked by O-SOC-1.**

### 6.3 Decision — HTTP client: `ureq` (citation, not re-litigation)

`HARNESS-LLM-BACKEND.md` §5 Decision 2 already DECART'd `ureq = { version = "2",
default-features = false, features = ["tls", "json"] }` against `reqwest` for exactly this
workload class (synchronous, one blocking call per dispatch, no tokio) under the 2026-07-15
operator mandate; `llm-adapters/Cargo.toml:12` is the third in-repo use of the identical spec.
Same decision, same spec, adopted. **Probe:** IG's container flow needs a status poll
(async-ish); resolved by the drainer's claim/reclaim loop, not by an async client.

### 6.4 Decision — multipart encoding: hand-rolled std, not a crate

| Criterion | Hand-rolled (~40 lines, chosen) | `multipart`/`ureq_multipart` crate |
|---|---|---|
| Scope needed | One format, one part-kind (photo bytes + string fields), one consumer (TG) | Full RFC 7578 generality unused |
| Supply chain | Zero new deps | New dep for a format that is boundary+headers+bytes |
| Correctness | Falsifiable: round-trip test + live TG probe | Same, plus someone else's parser surface |

**DECISION: hand-rolled `multipart.rs`**, std-only, with a unit test asserting the exact byte
layout and the Wave-0 live probe as the end-to-end check. **Probe:** if a second multipart
consumer with exotic needs appears (resumable video upload for Reels via `rupload.facebook.com`),
re-open this decision — that endpoint is a different protocol anyway and is out of v1 scope.

### What does / doesn't need a DECART, at a glance

| Item | DECART? | Why |
|---|---|---|
| First-wave platform set | **Yes — §6.1** | New external services at a trust-relevant surface |
| Media hosting surface | **Yes — §6.2, deferred O-SOC-1** | New public surface; prior R2 blocker on record |
| `ureq` | Cited — §6.3 | Already decided twice; identical spec reused |
| Multipart encoder | **Yes-lite — §6.4** | Would otherwise be a silent new dep |
| Image transcode dep | Flagged so nobody adds it | §4.4 routes renditions to the media pipeline instead |
| `Spool`/`TokenBucket`/`ChannelLedger` reuse | No | Zero-dep internal primitives, reused verbatim |
| TikTok/X adapters | Deferred + named triggers | No integration exists until a trigger fires |

---

## 7. Build plan — waves with falsifiable done-checks

Dependencies re-derived at consolidation (protocol step 2), not accepted from draft order:
Wave 1's two lanes are mutually independent; Meta *paperwork* has no code dependency at all and
therefore belongs in Wave 0 despite its adapter landing in Wave 2.

### WAVE 0 — port + Telegram + outbox spine (no external approvals; independently shippable)

| # | Step | Done-check (falsifiable) |
|---|---|---|
| 0a | `kernel/src/ports/social.rs` + `pub mod social;` in `ports/mod.rs`; types §3 | `cargo test -p dowiz-kernel` green; `cargo tree -p dowiz-kernel` shows **zero** new deps |
| 0b | `social-adapters/` crate scaffold + `SocialQuirks::telegram()` + `HttpTransport` + `multipart.rs` | `cargo build` in `social-adapters/`; multipart byte-layout unit test RED→GREEN |
| 0c | `TelegramAdapter` (`sendPhoto`/`sendMessage`/`sendMediaGroup`, preflight vs caps) | Live probe: post photo+caption to an operator-owned test channel → real `PostReceipt.post_id`; `health()` with a garbage token → `Err(AuthExpired)`, never Ok |
| 0d | `outbox.rs` drainer: Spool + TokenBucket + sent-ledger + typed retry (§5) | Kill-network test: drop connectivity mid-drain → record stays claimed→reclaimed, re-posts ONCE on restore, sent-ledger blocks the duplicate (assert exactly 1 message in channel) |
| 0e | Telemetry rows + criterion benches for `preflight`/adapt/`idem_key` | `bench_track.py` run, baselines seeded; probe test fails if telemetry row missing |
| 0f | **Meta paperwork (no code)**: create the dev app, start Business Verification, prep App Review screencasts for `instagram_content_publish` + `pages_manage_posts` | Verification submitted — artifact: submission reference recorded in-tree |

### WAVE 1 — two independent lanes (parallel-safe)

| Lane | Steps | Done-check |
|---|---|---|
| 1-V (blocked on O-SOC-1) | `ViberChannelAdapter` (`/pa/post`, webhook set-up, `X-Viber-Auth-Token`); preflight enforces 1 MB/JPEG/1-photo | Live probe: post to an operator-created test Viber channel; oversized photo draft → `Err(BadRequest)` at preflight, nothing enqueued |
| 1-C | Owner-facing config: per-venue platform enable/token paste (file-based until P16); master-post → per-platform adapt fn (pure, kernel) | Table-driven kernel test: one master post × every adapter's caps → each `preflight` passes or fails exactly as caps predict (the P2 parity pin, §4.2) |

### WAVE 2 — Meta adapters + GBP application (code parallel-safe from Wave 0; activation externally gated)

| # | Step | Done-check |
|---|---|---|
| 2a | `FacebookPageAdapter` (`/photos`, `/feed`, long-lived Page token flow) | Single-tenant live probe on the operator's own Page (role-holder — works pre-Advanced-Access) |
| 2b | `InstagramAdapter` (container create → status poll → publish; runtime quota via `content_publishing_limit` — never the doc constant, §2.3) | Single-tenant live probe; quota check asserted against the live endpoint |
| 2c | `tokens.rs` refresh housekeeping (IG 60-day) | Clock-mocked test: token at day 30 → refresh queued; expired token → `AuthExpired` DEAD row surfaced, never a silent retry |
| 2d | GBP Basic API Access application (form; no code until approved) | Application artifact recorded; adapter only after 300 QPM granted |
| 2e | **Multi-tenant activation** — gated solely on Meta Advanced Access | First non-operator venue connects via OAuth (needs P16 or a minimal hosted redirect page — named dependency) and publishes |

### DEFERRED — named triggers only (no dates)

- **TRIGGER-TIKTOK** (§2.4): ≥3 paying venues request it AND Wave 1 live → build adapter + submit audit.
- **TRIGGER-X** (§2.5): a venue asks and accepts per-post cost pass-through.
- **WhatsApp**: permanently out of `SocialPoster` (§2.6); any WhatsApp work goes to IP-15.

Implementation that follows is bound by AGENTS.md protocol step 8: plan-is-spec, TDD (each
done-check written RED first), telemetry+bench per change, worktree isolation for any concurrent
code-writing subagents (AGENTS.md shared-index TOCTOU rule).

---

## 8. Risk register — the honest "auto-posters get banned" section

| # | Risk | Severity | Mitigation (structural, not hoped — Ananke) |
|---|---|---|---|
| R1 | **Shared-app SPOF (Meta)**: every venue rides dowiz's ONE app; one spam flag / failed re-review / missed data-use checkup suspends ALL venues' IG+FB posting simultaneously. The platform imposes this shape — per-venue Meta apps (each needing its own review) are not realistic. | **HIGH — the biggest single risk in this plan** | Telegram/Viber lane is per-venue-token, no shared identity — the resilient baseline that keeps working; Meta lane is additive. Degrade-closed: Meta outage → posts queue as DEAD rows surfaced to owners, nothing else breaks. Documented honestly to clients. |
| R2 | Meta spam policy: "repetitive content… at lower frequencies" is flaggable — templated dailies across N accounts is the exact fingerprint | HIGH | Content is **owner-authored per venue** (never one template blasted across venues); cadence capped by TokenBucket well under platform limits; `Rejected` = poison, never retried (§5.2) |
| R3 | Meta churn: version every ~3-4 months; expired versions **silently forwarded** (breaks without erroring) | MED | `api_version_pin` in Quirks (§4.1) — explicit, greppable; quarterly re-verify task; adapter probes fail loudly on shape drift |
| R4 | Viber monetization rug-pull (precedent: 2024 bot fees) hits the Channels Post API | MED | Adapter ring is optional per venue; loss of one platform never degrades the product core (M6 posture) |
| R5 | IG token refresh missed → silent posting stop | MED | `refresh_due()` housekeeping + `AuthExpired` DEAD rows surfaced in owner UI — a dead pendulum is the P5 failure shape this explicitly wires the return-swing for |
| R6 | Duplicate post on crash between platform-accept and sent-ledger write | LOW | One-write failure window by construction (§5.3); Wave-0 done-check 0d exercises it |
| R7 | TikTok/X economics or audit change again | LOW (deferred) | Nothing built until triggers fire; zero sunk cost |

---

## 9. 2-question doubt audit (AGENTS.md closing ritual — applied to this blueprint)

**Q1 — least confident about (not rounded down):**

1. **IG daily-cap docs are internally inconsistent (100 vs 50 posts/24h)** — real, verified
   inconsistency in Meta's own pages. *Resolved structurally*: the adapter reads the live
   `content_publishing_limit` endpoint and never hard-codes either number (done-check 2b).
2. **Meta App Review SLA and pass-likelihood for a small UA vendor — UNVERIFIED** (Meta publishes
   no SLA). This is the plan's only unbounded calendar item; that's exactly why it's Wave-0
   paperwork and why no client-facing promise may be tied to a date.
3. **"Viber Channels Post API stays free" is a forward-looking claim** with a contrary precedent
   (2024 bot fees). Priced in via R4; not assumed away.
4. **TikTok availability/auditability for a Ukraine-based vendor — UNVERIFIED** in either
   direction. Low exposure: nothing is built until TRIGGER-TIKTOK fires.
5. **Exact Viber rate limits are undocumented** — the `viber_channel()` TokenBucket numbers will
   be a conservative guess needing live tuning; flagged in Quirks doc-comment, not presented as fact.
6. **The token-paste interim UX** (Wave 0-1, pre-P16) is assumed acceptable for pilot venues —
   plausible for BotFather tokens, unvalidated with a real client. Cheap to test with the first
   pilot; the fallback (concierge onboarding by the operator) costs nothing to design.
7. **Caps numbers snapshot** (4096/7000/2200 chars etc.) were verified now but live in `Quirks`
   constants — they will drift. Mitigation is the quarterly re-verify task (R3), which must cover
   all platforms, not just Meta.

**Q2 — the biggest thing I might be missing (one honest answer, no hedge):**

The plan optimizes the *pipe* and under-weights the *content*. Clients asked to "post once,
publish everywhere" — but a caption that works on Instagram (hashtags, no clickable link) is
wrong for Telegram (link previews, no hashtag culture) and for Viber (7000-char space, 1 photo).
If v1 ships as a dumb replicator, the output will look bot-like on every platform at once — which
is both the Meta spam fingerprint (R2) *and* bad marketing that clients will turn off. The §5.1
`adapt` step and `links_clickable`-style caps are the acknowledgment, but true per-platform
composition (P16 composer showing a per-platform preview, owner tweaking each) is the difference
between "feature clients keep using" and "feature clients try twice." **Flagged as a P16
requirement, not resolved here.** (An LLM-assisted per-platform caption rewrite via the existing
`LlmBackend` port is an obvious, cheap follow-up — deliberately NOT scoped into v1: it would
couple this feature's viability to the harness arc. Recorded so it's a decision, not an accident.)

---

## 10. Boundary — what this blueprint does NOT do

- Implements nothing; edits no code, CI, or canon. The next unit of work is Wave 0 per §7.
- Does not touch money/auth/RLS/migrations (red-lines) — venue marketing tokens only.
- Does not decide O-SOC-1 (media host) — operator ruling, with a flagged recommendation (§6.2).
- Does not promise Meta/TikTok timelines — external review queues are named as unschedulable.
- Does not add WhatsApp broadcast — re-scoped to IP-15's messaging design (§2.6).
- Does not build UI — composer/preview/OAuth surfaces are Phase-16 dependencies, named in §7.

---

## 11. EXTENSION (2026-07-18) — content generation, manual/agentic modes, channel breadth

> **Origin: operator directive 2026-07-18** (verbatim intent): not only Telegram — auto-posting
> of AI-generated OR natively-authored content to social media, channels, messengers, mailing
> lists, SMS — supporting BOTH manual posting AND agentic-workflow-driven posting.
>
> This section EXTENDS §0–§10. It changes **no** Wave 0/1/2 platform structure, no §3 port type,
> no §6 decision, no §8 risk row. Cross-referenced designs, read live this pass:
> `CORE-ROADMAP-2026-07-17/BLUEPRINT-P40-agent-loop-tool-wiring.md` (§2 `ToolPort` shape, §1
> anti-scope, §4.1 reachability argument) · `BLUEPRINT-P41-three-mode-ai-operation.md` (§2
> `AiMode { Off, LocalOffline, Connected }`, default Off, no auto-escalation) ·
> `DEMO-MARKETING-PIPELINE-REFACTOR-2026-07-17.md` (P20 DM-1/DM-7 offer objects) · master
> roadmap §10.5.5 (P22/P43 absorption ledger).

### 11.1 Content generation — dual path, ONE output type

The §5.1 flow already begins "owner authors ONE master post" without naming a type. Named now:

```rust
/// The pre-adapt master post — what BOTH generation paths and the manual composer produce.
/// Downstream (approve → adapt → preflight → Spool) consumes MasterPost and cannot tell
/// which path created it; the parity test below pins that.
pub struct MasterPost {
    pub venue_id: String,
    pub caption: String,
    pub media: Vec<MediaItem>,        // §3 type, reused verbatim
    pub link: Option<String>,
    pub source: DraftSource,          // provenance — load-bearing (§11.3 badge + ratchet)
    pub status: DraftStatus,
}
pub enum DraftSource { Manual, Template(TemplateId), Llm { model_id: String } }
pub enum DraftStatus { PendingReview, Approved, Discarded }
```

- **Path A — native template (zero-AI; works in P41 mode 1 / `AiMode::Off`).** A closed set of
  deterministic templates (one per §11.2 post type), pure fn
  `render_template(TemplateId, &Facts) -> MasterPost` — no LLM, no network, table-tested.
  Facts are structured inputs (menu-item name/price, offer terms, new hours), never free text
  from a model. Example: new menu item added → "Новинка в меню: {name} — {price} грн. {link}".
- **Path B — AI draft (P41 modes 2/3).** One `LlmBackend.chat` call through the existing
  Harness/Dispatcher (budget + harvest apply — no second budget mechanism, P40 §3.6 discipline);
  prompt = the same structured facts + optional venue voice notes; output = `MasterPost` with
  `Llm` provenance. `AssistantUnavailable` ⇒ degrade to Path A or the plain manual composer —
  typed, never a blocked post. This also answers §9 Q2's per-platform-tone problem: Path B can
  draft per-platform caption variants where Path A emits one caption for all.
- **Reconciliation with §9 Q2's recorded deferral** (honesty, not silent contradiction): §9
  deliberately kept LLM caption drafting OUT of v1 because it "would couple this feature's
  viability to the harness arc." The 2026-07-18 operator directive supersedes the deferral, and
  the coupling concern is answered structurally rather than waved off: **Path A keeps the
  feature fully viable at `AiMode::Off`** — AI is an enhancement lane, never a dependency. This
  is the P41 three-mode invariant applied to marketing content.
- **Parity done-check (falsifiable):** one table-driven test feeds a `Template`-produced and an
  `Llm`-produced `MasterPost` with identical content through adapt + every adapter's
  `preflight` and asserts byte-identical treatment — downstream indifference proven, not stated.

### 11.2 Post types — grounded, closed set (a venue owner's five, not a marketing suite)

| # | Type | Facts source | Path A template |
|---|---|---|---|
| T1 | Daily special / new menu item | menu change (name, price, photo) | yes |
| T2 | Sold-out notice ("Деруни закінчились — приходьте завтра") | menu item state change | yes |
| T3 | Offer/promotion announcement | **P20 DM-1 offer objects / DM-7 operator offers — render-only.** The discount math, redemption ledger, and publish-gating stay P20's scope entirely; this post type consumes a computed offer and formats it | yes |
| T4 | Hours / delivery-area change | venue config change | yes |
| T5 | Aggregate social proof ("50 замовлень сьогодні") | order-event aggregate count | yes |

- **T5 privacy check (done before proposing, per the directive):** the count is an aggregate over
  the event log — the same aggregation level as `ChannelLedger.orders_by_channel()`
  (`analytics.rs`), zero customer fields read. Two guards anyway: (a) minimum threshold — the
  post type is only offered when count ≥ 10 (a count of 1-2 in a tiny venue is both embarrassing
  and weakly correlatable to an individual order); (b) the number is READ from the log, never
  authored — a fabricated count is unrepresentable because the template's fact slot is filled by
  the aggregate query, not by any text input (model or human).
- **Closed-set discipline:** a sixth post type is a reviewed enum addition, not configuration.
  This is the anti-CMS line (§11.6).

### 11.3 Manual vs agentic — approval is the DEFAULT, autonomy is earned

**Manual (DEFAULT, stated as such):** every `MasterPost` — regardless of `DraftSource` — lands
`PendingReview`. The owner reviews (with an AI-drafted badge from provenance), edits per
platform, approves. **Only `Approved` drafts are adapted + enqueued to the Spool.** Posting to a
business's public channels without human review is a real reputational risk; auto-post is
therefore an explicit opt-in, never a default — this is a design commitment, restated in §11.6
anti-scope so it cannot be "optimized away" later.

**Agentic (OPT-IN, ratcheted — concrete and falsifiable, not "with guardrails"):**

| # | Guardrail | Falsifier (each a named test at build time) |
|---|---|---|
| A1 | First-N: the first **10** posts per (venue, post-type) ALWAYS require approval, opt-in flag notwithstanding | test: post #10 with opt-in on still lands `PendingReview` |
| A2 | Earned: autonomy activates only after **10 consecutive approved-without-edit** drafts of that type; any edit or rejection resets the counter to 0 | counter-reset test: 9 clean + 1 edited ⇒ counter 0 |
| A3 | Rate: autonomous posts draw from a **dedicated `TokenBucket` (capacity 1, refill 1/day per platform)** — max one autonomous post per platform per day; manual posts do not consume it | exhaustion test: 2nd autonomous draft same day ⇒ `PendingReview`, not queued |
| A4 | Revoke: any `SocialError::Rejected` on an autonomous post revokes autonomy for that post-type back to manual (the platform said the content is bad — the P5 return-swing) | revoke test: `Rejected` receipt ⇒ next draft `PendingReview` |
| A5 | Kill switch: one per-venue config bit disables all autonomy instantly | config-flip test |
| A6 | **Publish authority is never the model's** (§11.4): even at full autonomy the model only DRAFTS; the `PendingReview → Approved` transition is executed by the deterministic policy layer evaluating A1–A5 | grep/namespace check: no publish/approve symbol reachable from the tool executor |

### 11.4 Agent-loop integration — P40 `ToolPort` pattern, exact seam, correctly gated

P40's live shape (read this pass): closed enums `ToolResource { OrderStatus }` /
`ToolAction { Read }`, `ToolSpec`/`ToolInvocation`/`ToolOutput`/`ToolError`, `trait ToolPort`
behind the `agent-facade` firewall — and a hard anti-scope: **exactly one tool in P40; P42
standardizes the extension pattern.** Therefore the tools below are a **named FUTURE ToolPort
extension, buildable only after P42's pattern lands** — P40 ships untouched, no enum variant is
added today.

Two tools, pre-declared (small set — deliberately not a framework):

1. **`draft_social_post`** — scope `{resource: SocialDraft (new closed-enum variant), action:
   Draft (new variant)}`. Args: post type + structured facts. Executor = §11.1 Path A or Path B
   selected by `AiMode`; appends a `MasterPost(PendingReview)` to the review queue — and can
   reach **nothing else** (the facade re-exports the queue-append fn only; same
   namespace-reachability argument as P40 §4.1). `Publish`/`Approve` are deliberately NOT
   `ToolAction` variants — unrepresentable, the same structural move P40 used for writes.
2. **`read_post_queue`** — scope `{resource: SocialDraft, action: Read}`. Lists draft statuses +
   DEAD rows (§5.1), so the assistant can report "Tuesday's Viber post failed: token expired —
   re-connect in settings" conversationally.

**Worst-case reachability (P40 §4.1 style):** a fully adversarial model can (i) append drafts to
a queue a human reads, (ii) read draft/DEAD statuses. It cannot publish, cannot approve, cannot
name `SocialPoster`, the token store, or the Spool. The blast radius of a hallucinated draft is
one row in a review queue.

### 11.5 Channel breadth — where each operator-named channel lives (mapped, not lumped)

| Operator's channel | Home | Reasoning |
|---|---|---|
| Social media (IG/FB, TikTok, X) | **P22 `SocialPoster`** — Waves 1-2 + §2.4/§2.5 triggers, unchanged | already scoped |
| Channels (Telegram, Viber channels) | **P22 `SocialPoster`** — Wave 0/1, unchanged | already scoped |
| Video (YouTube Shorts) *(row added 2026-07-18)* | **P22 `SocialPoster`** — §12 Wave 2-Y | Shorts relay only; Community posts have no write API (§12.1), auto-generation deferred (D-YT-3) |
| Messengers (WhatsApp/Viber 1:1 campaigns) | **IP-15 `ChannelAdapter` campaign lane** (under P22's number per the roadmap §10.5.5 absorption ledger; a SEPARATE lane from `SocialPoster`) | §2.6's precedent verbatim: recipient-list broadcast to opted-in customers = CRM messaging, not feed publishing |
| Mailing lists (email) | **same IP-15 campaign lane** | not `SocialPoster`-shaped: N recipients + bounce/unsubscribe/consent ledger vs one `post_id` per channel; legal unsubscribe obligations; needs an email-provider adapter + its own mini-blueprint (named, not written here) |
| SMS | **same IP-15 campaign lane** | same shape as email, plus the cost honesty below |
| Transactional sends (order-status, OTP) over ANY channel incl. SMS/email | **NOT this feature — P43 DoD-2 send path + P49 customer-side consumer** | notification ≠ marketing; P43/P49 own it; anti-scope here |

- **SMS cost honesty (directive-mandated, stated plainly):** SMS is per-message **PAID** through
  any provider (Twilio, TurboSMS, etc.), unlike Telegram/Viber-channel posting which is free.
  Exact rates are market/provider-dependent (UNVERIFIED here — priced at build time, not
  assumed); the structural consequence is certain either way: campaign-lane preflight MUST show
  `recipient_count × unit_cost` before send, and cost pass-through to the venue must be explicit
  in the UI. A 500-recipient blast is a real invoice, not a free post.
- **Why the campaign lane shares P22's number but not its trait:** it reuses the producer side
  verbatim — same `MasterPost`, same §11.3 approval gate + ratchet, same Spool/TokenBucket
  outbox doctrine, same `?ch=` attribution — and diverges only at egress (per-recipient fan-out
  + consent ledger). One content pipeline, two egress families. Splitting the producer side
  across two phases would duplicate the approval/ratchet machinery; forcing recipient-list
  egress into `SocialPoster` would corrupt a clean one-post-one-receipt trait. The roadmap's
  "IP-15 → absorbed into P22, do not duplicate under P43" ledger line already points this way.

### 11.6 Added anti-scope (extends §10)

- **NOT a CMS / content-calendar / marketing suite.** Closed 5-type post set (§11.2); no
  calendar UI; no analytics dashboards beyond the existing `ChannelLedger` readers.
- **Auto-post-without-review is NEVER the default** — explicit per-venue, per-post-type opt-in
  behind the §11.3 ratchet, full stop.
- **Publish is never a model-callable action** (A6/§11.4) — at any autonomy level.
- **No re-design of P20's discount/offer math** — T3 is render-only consumption of DM-1/DM-7
  objects; publication gating stays P20/P18's.
- **No P43 re-scoping** — export/backup/hosting ports and the transactional send path are
  untouched; only the P22↔P43 boundary is clarified (§11.5).
- **No SMS/email adapter before the campaign-lane mini-blueprint + consent-ledger design
  exists.** Recipient lists are personal data (phone numbers, emails) — the one place this
  extension touches PII, named honestly; the `SocialPoster` lane deliberately holds none.
  Consent/opt-in/unsubscribe design is a precondition, not a retrofit.
- **P40 untouched** — no `ToolResource`/`ToolAction` variants until P42's extension pattern
  lands.

### 11.7 Wave placement (extends §7 — no renumbering, no existing done-check changed)

- **Wave 1-C gains:** `MasterPost` + `DraftSource`/`DraftStatus` + the Path A template renderer
  + review-queue statuses — all pure, no external approvals, and the §11.1 parity done-check.
- **New Wave A (agent lane)** — strictly after P40 T1–T9 land AND P42's tool-extension pattern
  exists: the two §11.4 tools + the §11.3 policy layer. Done-checks: (a) tool-produced draft
  lands `PendingReview` with a spy asserting **zero** `SocialPoster` calls; (b) ratchet tests
  A1–A5 green; (c) the §11.1 parity test extended to tool-produced drafts.
- **Campaign lane (mailing lists / SMS):** blocked on its own mini-blueprint (provider DECART +
  consent ledger); claims no wave number until that exists.

---

## 12. EXTENSION (2026-07-18) — YouTube as a posting channel (Wave 2-Y)

> **Origin: operator directive 2026-07-18** — add YouTube explicitly as a content-posting
> channel. This section EXTENDS §0–§11: no §3 port type is changed, no §6 decision reopened, no
> existing wave renumbered. Research web-verified 2026-07-18 against Google's **official** docs
> (quota page last updated 2026-06-01) plus secondary passes; claims that could not be verified
> are marked UNVERIFIED per the §2 convention.
>
> **Instagram + Facebook are NOT re-designed here.** Both are already adequately covered by the
> existing Meta Wave 2 lane — §2.3 research, §7 steps 2a/2b/2e, risks R1–R3 — under Meta's
> single Graph API app umbrella (Meta owns both platforms; one app, one review, two adapters).
> YouTube is an *addition to* the wave sequence, not a revision of it.

### 12.1 Platform research — YouTube — MEDIUM-HIGH friction, Meta-class gatekeeping

- **Community posts (text/image posts to a channel's Community tab) — NO write API exists.**
  The YouTube Data API v3 has no community-post endpoint at all (verified against the official
  API reference index; a years-old, still-open gap every third-party poster project confirms).
  Creation is YouTube-Studio-only. Consequence under the IP-15 honesty rule (§1.3): the UI must
  never offer "post to YouTube Community" — the capability is unrepresentable, exactly like
  WhatsApp feed posting (§2.6).
- **Shorts — the one API-reachable posting surface.** No separate Shorts endpoint exists; a
  Short is an ordinary `videos.insert` upload that YouTube auto-classifies by shape: vertical
  (9:16), ≤60 s (the reliably-supported bound; the 2024 "3-minute Shorts" expansion as an API
  classification bound is UNVERIFIED — pin 60 s), plus `#Shorts` in title/description as a
  conventional hint. Upload is **direct bytes (resumable/multipart)** — like Telegram, zero
  dependency on the O-SOC-1 public-media-host ruling. Native scheduling exists
  (`status.publishAt` with `privacyStatus: private`) → `native_schedule: true`.
- **Quota — restructured; verified against the official quota page (updated 2026-06-01).**
  `videos.insert` now draws from its **own dedicated bucket: default 100 calls/day per API
  project** — no longer the legacy 1600-units-against-10,000 model (secondary sources still
  quoting 1600 are stale). `search.list` likewise has its own 100/day bucket; all other
  endpoints share the 10,000 units/day pool. **The bucket is per-project, not per-venue**:
  every venue's uploads ride dowiz's one Google Cloud project → 100 Shorts/day across the
  whole fleet (≈33 venues at 3/day). Ample for pilot; a quota-extension request form exists.
  Maps verbatim onto §5: `TokenBucket[platform]` is already per-platform-global — YouTube's is
  capacity 100, refill 100/day.
- **Gatekeeping (the real cost — same class as Meta §2.3), two independent Google gates:**
  (a) **API compliance audit** — uploads from unverified API projects are **locked private**
  (policy in force since 2020-07-28, still current); locked videos cannot be appealed, only
  re-uploaded after verification. Pre-audit uploads *succeed* (real video id in the receipt)
  but stay invisible to the public — an honest single-tenant test surface, a silent failure
  mode multi-tenant (R8). (b) **OAuth app verification** — `youtube.upload` is a sensitive
  scope; unverified apps cap at 100 test users, and "Testing"-status apps get 7-day
  refresh-token expiry. Both are unschedulable external review queues → the paperwork belongs
  in the Wave-0 lane beside Meta's 0f; multi-tenant activation gates on both passing.
- **Auth mechanics:** OAuth 2.0 authorization-code flow per venue channel (needs a redirect
  surface — the same P16-or-minimal-hosted-page dependency as Meta step 2e); refresh tokens are
  long-lived once the app reaches production status. Token store: `tokens.rs` (§4.5) unchanged,
  a `youtube_oauth` record kind with refresh housekeeping beside the IG 60-day lane.

### 12.2 Scope decision — Shorts relay YES · Community posts IMPOSSIBLE · auto-generation DEFERRED

- **D-YT-1 — Community posts: not offered.** No API ⇒ no adapter capability ⇒ never in the UI.
  Revisit only if Google ships a community-post write endpoint (grep-able:
  **TRIGGER-YT-COMMUNITY**).
- **D-YT-2 — YouTube lane v1 = Shorts *relay* of owner-shot vertical clips.** The venue owner
  films a 15-60 s vertical clip on a phone (the production step stays human, free, and
  per-venue — which also keeps the R2 anti-template posture intact); dowiz uploads it with
  title/description carrying the `?ch=youtube` link → `ChannelLedger` attribution closes
  exactly as §5.4. **§3 types are reused verbatim**: a `PostDraft` with one
  `MediaItem { kind: Video }`; `YoutubeAdapter` caps = `{ text: false, photo: false,
  video: true, album: false, max_caption_chars: 5000 (description bytes), max_media_per_post:
  1, media_by_upload: true, media_by_url: false, links_clickable: true, native_schedule:
  true }`. One named additive delta, decided at build time and never silently: `MediaItem`
  carries no duration/aspect, which Shorts-shape preflight needs — either the composer supplies
  them as facts or `MediaItem` gains two `Option` fields (additive; no parallel type). §11.1's
  dual generation paths apply to the *title/description text only* — the clip itself is never
  generated (D-YT-3).
- **D-YT-3 — `compose()`-derived auto-Shorts: honestly evaluated, DEFERRED.** The tempting
  move: `engine/src/field_frame.rs:218` `compose()` already emits bit-deterministic RGBA
  frames (P38's oracle), so a frame sequence "could become" an animated Short. The honest gap:
  **frames ≠ video file.** YouTube ingests encoded containers (MP4/WebM); this repo has no
  video encoder, no muxer, and not even a JPEG encoder — §4.4 already deliberately kept image
  codecs out of the adapter crate, and a video-codec dep (or an ffmpeg external binary) is a
  supply-chain DECART that loses today with zero demand signal. A plausible
  zero-new-Rust-dep route does exist — the P16 composer rendering via P38's surface and
  capturing client-side (`canvas.captureStream()` + `MediaRecorder` → WebM in the browser; the
  adapter only relays bytes) — but it depends on P16+P38 landing, and an abstract physics-field
  animation is not obviously *food marketing* content anyway; the owner's dish clip is.
  **TRIGGER-YT-AUTOGEN** (grep-able): ≥3 paying venues on the YouTube lane request generated
  Shorts AND the P16 composer + P38 render surface are live (unlocking the browser-capture
  path) → then DECART browser-capture vs in-repo encoder. Until the trigger fires: nothing is
  built, no codec enters the tree.

### 12.3 Wave placement — Wave 2-Y (parallel lane inside Wave 2; paperwork joins the Wave-0 lane)

Readiness class matches Meta, not Telegram: the REST + per-venue OAuth code is easy, but the
compliance-audit + scope-verification queues are the long pole ⇒ Wave 2, never Wave 0/1.

| # | Step | Done-check (falsifiable) |
|---|---|---|
| 0g (joins the 0f lane) | **Google paperwork (no code)**: Cloud project, OAuth consent screen, sensitive-scope verification submission, YouTube API compliance-audit submission | Submissions recorded in-tree — same artifact convention as 0f |
| 2f | `YoutubeAdapter` (`videos.insert` resumable upload; caps per D-YT-2; preflight incl. Shorts-shape) | Single-tenant live probe on the operator's own channel: real video id in `PostReceipt`; pre-audit the probe ASSERTS `privacyStatus` comes back locked-private (the honest expected state) — never pretends public |
| 2g | `youtube_oauth` token flow in `tokens.rs` (refresh; production-status precondition documented) | Clock-mocked refresh test, mirror of 2c |
| 2h | **Multi-tenant activation** — gated on BOTH Google gates (audit + scope verification) passing | First non-operator venue connects via OAuth and publishes a *public* Short |

### 12.4 Risk register additions (extends §8)

| # | Risk | Severity | Mitigation (structural) |
|---|---|---|---|
| R8 | **Google shared-project SPOF** — same shape as R1: all venues ride dowiz's ONE Cloud project; a failed/revoked audit or lapsed OAuth verification locks every venue's uploads private **silently** (uploads still "succeed") | HIGH | Same doctrine as R1: the per-venue-token lane (TG/Viber) stays the resilient baseline; the receipt records `privacyStatus` and the owner UI surfaces "uploaded but private-locked" as a warning row — never a silent success |
| R9 | Quota bucket is fleet-shared: 100 uploads/day per project, all venues combined | MED | Global YouTube `TokenBucket` (capacity 100/day) → the drainer degrades to queue-and-wait, never a 403 spray; quota-extension form filed when the fleet approaches ~50/day sustained |
