# BLUEPRINT P53 — Tor/onion integration: anonymous-access tier, Onion-Location + QR convenience, mesh-transport option honestly scoped (2026-07-18)

> **Planning document — writes no product code.** Written against the 20-point contract in
> `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map §10). Component:
> **DELIVERY** (client-facing anonymous access) with a **PROTOCOL cross-reference** (§5.3's
> mesh-transport option, deferred with a named trigger). Provenance: this ACTIVATES fold-in
> ledger item **L4** ("Anonymous `.onion`/Tor tier", `CORE-ROADMAP-INDEX.md` §6 /
> SOVEREIGN §9.2 — E53-form waiver, original trigger "vendor-node tier ships AND a venue
> requires anonymity"). Operator ask (2026-07-18, verbatim): "можливість tor, onion інтеграції
> і взаємодії — зручної" — Tor/onion integration and interaction capability, CONVENIENT. The
> direct operator request supersedes the waiver's trigger condition; the waiver's honesty
> discipline is preserved by keeping the big half (mesh transport over Tor) deferred-with-
> trigger rather than silently activated wholesale. Structural template:
> `BLUEPRINT-P51-open-map-routing.md` (same research-verdict-first shape, same honest-rejection
> discipline — P51 rejected satellite on resolution physics; this file rejects arti-as-host on
> the Tor Project's OWN maturity warnings, and scopes Tor to what its latency physics supports).
>
> **Scope framing, stated up front:** Tor here is standard privacy networking — the same
> technology the BBC, ProPublica, and SecureDrop deployments use so that readers/sources reach
> a service without exposing their network identity, and so a small venue can serve customers
> without publishing its hub's IP/location. It changes the TRANSPORT/network-privacy layer
> ONLY. The trust model is untouched: a Tor-connecting client authenticates with exactly the
> same capability certs (P37 `CAP_HEADER`), the same fail-closed verification, the same
> red-line denials. Anonymity at the network layer is never an excuse to skip the auth layer
> (§2 anti-scope, §4.5, §5.1).

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

Verified 2026-07-18 against dowiz `main` (working tree clean at session start) and bebop-repo
`main` @ `e56ba6a35` (= `openbebop/main`). Paths relative to `/root/dowiz/` unless noted.

| # | Claim | Fresh `file:line` (this pass) | Status |
|---|---|---|---|
| 1 | The L4 waiver text, verbatim | `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md:496`: "E53-form waiver — what: anonymity/Tor access tier; why-suspended: no vendor-node tier exists and no anonymity demand demonstrated; trigger to revisit: vendor-node tier ships AND a venue requires anonymity"; mirrored in `CORE-ROADMAP-INDEX.md:110` (§6 row L4) | **VERIFIED — activation provenance.** The operator's direct 2026-07-18 request is the demand signal the trigger was waiting for; recorded as trigger-superseded, not trigger-met |
| 2 | The ONLY in-repo HTTP server (P37's extension point) is `native-spa-server`: `build_router` = ServeDir + SPA fallback + two middleware layers; **every response already passes a headers middleware** | `tools/native-spa-server/src/lib.rs:93-106` (`build_router`), `:36` (`SECURITY_HEADERS` const), `:51` (`security_headers` middleware fn) | **VERIFIED — the Onion-Location layer is a third sibling of an existing pattern, not new machinery** |
| 3 | Server config is clap CLI + env only, incl. bind/port/TLS; NO onion/tor/socks concept anywhere in the crate | `tools/native-spa-server/src/main.rs:23-42` (`--root/--port/--bind/--tls-cert/--tls-key`, `SPA_*` envs); `grep -rn -i "tor\b\|socks\|onion" tools/native-spa-server/src/` → 0 hits | VERIFIED — O2's config additions are genuinely new, additive |
| 4 | Every dependency in the server crate is annotated "(cached)" — the crate's own convention says un-cached deps are a real constraint | `tools/native-spa-server/Cargo.toml:23-37` (axum/tower-http/tokio/rustls/clap, each with a "(cached)" comment) | **VERIFIED — no `arti` crate is cached; embedding arti would need a network/cargo unlock (O18a-class), a real cost the §1.1 DECART must price** |
| 5 | Deploy tier precedent: systemd units as operator-run host artifacts | `deploy/pgrust.service`, `deploy/native-spa-server.service`, `deploy/deep-clean.service` — each carrying "host action, run by the operator — NOT by the agent"; `deploy/README.md` | VERIFIED — O4's tor sidecar unit follows this exact idiom |
| 6 | NO QR-code generator exists in product code | `grep -rn -i "qrcode\|qr_" kernel/src/ engine/src/ tools/` → only `kernel/src/householder.rs:282,301` (`qr_step` — **matrix QR decomposition**, unrelated; a name-collision to avoid) and node_modules noise | VERIFIED — O3 is genuinely new; module named `qr_code.rs`, never `qr.rs` (householder collision) |
| 7 | QR-as-handoff already has planning precedent | SOVEREIGN §13 P52 K6: owner mints enrollment capability "→ QR/deep-link → the courier's un-enrolled device redeems it" (`MASTER-ROADMAP…:1541-1543`) | VERIFIED — O3's encoder is the shared substrate P52 K6 will also consume (one encoder, two consumers) |
| 8 | Mesh carriers, both repos: QUIC (UDP) and WSS (TCP) | `/root/bebop-repo/bebop2/proto-wire/src/iroh_transport.rs:1` ("QUIC transport — real node-to-node carrier (pure-Rust quinn/rustls)"); `wss_transport.rs` in the same dir (WebSocket/TLS/TCP carrier) | **VERIFIED — load-bearing for §5.3: Tor's SOCKS interface carries TCP streams only, so the quinn/QUIC carrier CANNOT ride Tor; the wss carrier COULD** |
| 9 | Transport trust posture is P36's, not touched here | `bebop2/proto-wire/Cargo.toml:50` `default = ["insecure-tls"]` (P36 R-2 owns the flip); hybrid signed frames remain the authentication layer regardless | VERIFIED — §2 anti-scope: P53 layers on top, substitutes nothing |
| 10 | Capability machinery in-repo; P37's planned HTTP mapping | `kernel/src/ports/agent/cap.rs:480` (`verify_chain`), `:404-433` (`RevocationSet`); `BLUEPRINT-P37-order-http-surface.md` §2 (`CAP_HEADER = "x-dowiz-cap"`, `HttpReject` taxonomy, 401/403 split) | VERIFIED — O5 re-binds P37's own tests to the onion listener; zero new auth code |
| 11 | Kernel purity law (where O3 lives and why) | `kernel/src/router.rs:221-228` doc ("takes clean triples so the kernel stays I/O-free") — the standing pure-function-only kernel discipline | VERIFIED — a QR encoder is pure bytes→matrix, kernel-shaped; all I/O stays in server/tool crates |
| 12 | P51's wire budget for the traffic that would ride an onion circuit | `BLUEPRINT-P51-open-map-routing.md` §5.3: order JSON ≪ 4 KiB at ≪ 1 event/sec; `CourierPositionUpdated` ≤ 32 B at ≤ 0.5 Hz; MapPack ≤ 8 MiB as a static asset | VERIFIED — §1.4's latency verdict is computed against these real numbers, not vibes |
| 13 | No vendor-node tier exists yet (the waiver's other trigger leg) | DELIVERY component status: "~0% deployable (no HTTP server, no rendered UI, no live deployment)" (`CORE-ROADMAP-INDEX.md:36`); P37/P45 both PLANNED | VERIFIED — hence §2's wave split: W0 code is buildable now; the live onion service itself lands WITH P37+P45, not before |
| 14 | Old-stack Tor posture: none | `grep -rn -i "onion" docs/design/` → planning docs only; no product code ever had a Tor path | VERIFIED — nothing to port, nothing to regress |

Ground truth is non-discussible; everything below builds on this table only.

---

## 1. Research verdicts (2026-07-18 web pass; every claim cited) — the four questions answered

### 1.1 How to add Tor to a Rust stack in 2026 — the DECART

Two DIFFERENT capabilities are being priced, and the honest answer differs per capability:
**(A) hosting an onion service** (the hub reachable as `xxxx….onion` without exposing its
IP/location — the Wave-0 need) and **(B) making outbound connections through Tor as a client**
(no Wave-0 consumer; see §5.3).

**Candidate (a): `arti` — the Tor Project's official Rust implementation.**

Live 2026 status, from primary sources:

- Client core: production-ready since Arti 1.0.0 (Sep 2022 — "ready for production use",
  [blog.torproject.org/arti_100_released](https://blog.torproject.org/arti_100_released/)).
  Arti 2.0.0 (Feb 2, 2026) is the major stable line with LTS
  ([blog.torproject.org/arti_2_0_0_released](https://blog.torproject.org/arti_2_0_0_released/));
  Arti 2.5.0 (Jun 30, 2026) stabilizes Counter Galois Onion relay encryption and turns on
  congestion control by default
  ([blog.torproject.org/arti_2_5_0_released](https://blog.torproject.org/arti_2_5_0_released/)).
- Embedding shape: `arti-client` crate, `TorClient::create_bootstrapped(config)` →
  `connect()` → `DataStream: AsyncRead + AsyncWrite`
  ([docs.rs/arti-client](https://docs.rs/arti-client/latest/arti_client/)). Onion-service
  client connections are behind the opt-in `onion-service-client` cargo feature; hosting
  behind `onion-service-service`; `vanguards` feature layered on either.
- Vanguards (guard-discovery defense) apply to BOTH onion clients and onion services, enabled
  by default since 1.2.2 (lite mode; full mode recommended for services with >1-month uptime)
  ([blog.torproject.org/announcing-vanguards-for-arti](https://blog.torproject.org/announcing-vanguards-for-arti/),
  Jul 2024).
- **Service-side hosting is EXPLICITLY experimental.** Arti's own `doc/OnionService.md`
  (fetched this pass via the GitHub mirror of the official repo): "suitable for testing and
  experimentation only and should not be used for anything you care about", may "compromise
  the privacy of your other uses of the same Arti instance"; named gaps: **no meaningful DoS
  protection** ("Rate limits, per-circuit connection limits, proof-of-work, and memory limits
  are not implemented" — memory-quota tracking landed partially in 1.3.0,
  [blog.torproject.org/arti_1_3_0_memquota](https://blog.torproject.org/arti_1_3_0_memquota/));
  **no circuit padding**; restricted-discovery/client-auth config incomplete. Source-conflict
  note, resolved honestly: that doc's "missing Vanguard support" line contradicts the dated
  July-2024 vanguards announcement (which explicitly covers "running an Arti onion service");
  the dated official blog wins on vanguards — but the PoW/DoS and padding gaps are
  corroborated independently (the PoW ecosystem page and the 1.4.0/1.8.0 release notes still
  describe service-side DoS resistance as "work towards",
  [blog.torproject.org/arti_1_4_0_released](https://blog.torproject.org/arti_1_4_0_released/),
  [blog.torproject.org/arti_1_8_0_released](https://blog.torproject.org/arti_1_8_0_released/)).
- Repo-local cost: no arti crate is cached (§0 row 4) — embedding needs a cargo/network
  unlock (O18a-class operator act) and pulls a large tree (tokio it shares; plus tor-* crates).

**Candidate (b): system C `tor` daemon as a deployment-tier sidecar.**

- Onion-service hosting is its most mature surface: v3 onion services, **proof-of-work DoS
  defense (Equi-X) since tor 0.4.8** (Aug 2023,
  [blog.torproject.org/introducing-proof-of-work-defense-for-onion-services](https://blog.torproject.org/introducing-proof-of-work-defense-for-onion-services/)),
  intro-point DoS defenses, client authorization, vanguards-lite built in — the configuration
  is two torrc lines (`HiddenServiceDir` + `HiddenServicePort`) forwarding to a loopback port
  ([community.torproject.org/onion-services](https://community.torproject.org/onion-services/advanced/dos/)).
- Packaged in every distro (`apt install tor`) — **zero cargo deps, zero dowiz code**; it is
  deployment configuration, exactly the `deploy/pgrust.service` tier (§0 row 5).
- Cost: reintroduces a non-Rust runtime process on the hub host. Honest scoping of that cost:
  it is an **optional per-hub deployment sidecar**, not a wire/trust-boundary dependency —
  M6's zero-dep law governs the protocol crates (untouched); M5's hub sovereignty explicitly
  covers what processes a hub runs. The canonical binary never links it; a hub without the
  sidecar loses nothing but the onion mirror.

**Candidate (c): others, one line each.** `libtor`/FFI-embedded C tor — REJECTED: worst of
both worlds (C in-process, build fragility, weaker maintenance than either candidate).
I2P/Lokinet — REJECTED: different networks with no Tor Browser reach; the convenience story
(§1.3) depends on Tor Browser existing on customers' phones. Vendor onion-CDN (Cloudflare-
style) — REJECTED: vendor lock-in, excluded by standing canon (P51's own hard constraint
class). Vanity onion prefixes (mkp224o mining) — REJECTED as scope, noted as harmless
operator cosmetics later: the QR/Onion-Location layer exists precisely so nobody reads the
address.

**DECART verdict table:**

| Capability | Choice | Grounds |
|---|---|---|
| (A) Onion-service HOSTING, Wave 0 | **System C `tor` sidecar** (deploy-tier config, O4) | Only candidate with production-grade service-side DoS protection (PoW, 0.4.8+); zero code, zero new cargo deps, zero unlock; the Tor Project's OWN doc forbids relying on arti hosting today ("should not be used for anything you care about") — believing them IS the native-Rust-first discipline applied honestly: the Rust-purity gain does not outweigh shipping a hidden service without DoS defense |
| (A) hosting, future | **arti `onion-service-service` migration — NAMED TRIGGER**: arti's OnionService.md drops its experimental warning AND service-side PoW lands. Re-check cadence: each arti minor release (LTS 2.x line). The torrc fragment and loopback-forward shape (§3) are chosen to be arti-config-translatable 1:1 (`proxy_ports` has the same shape), so the migration is a sidecar swap, not a redesign |
| (B) Outbound Tor client connections | **arti-client, embedded, WHEN a consumer exists** (§5.3's deferred mesh option, or a native-app privacy mode). No Wave-0 consumer ⇒ no dep, no unlock now. When triggered: `onion-service-client` + `vanguards` features, DECART report per the standing new-dep rule |
| (B) outbound via system tor SOCKS5 | Fallback only — if a consumer materializes before the arti unlock is granted; honest note that this couples outbound privacy to a host daemon |

### 1.2 `Onion-Location` — the standard discovery mechanism (real, current)

- The header is Tor Browser's built-in onion-discovery mechanism since Tor Browser 9.5: a
  clearnet HTTPS response carrying `Onion-Location: http://<addr>.onion/<path>` makes Tor
  Browser show a "**.onion available**" pill in the URL bar; one tap redirects to the onion
  mirror ([community.torproject.org/onion-services/advanced/onion-location](https://community.torproject.org/onion-services/advanced/onion-location/),
  [support.torproject.org/onionservices/onion-location](https://support.torproject.org/onionservices/onion-location/)).
- Validity requirements (spec'd, and they shape O2's design): the value must be a valid
  `http(s)://…\.onion` URL; the page carrying the header **must itself be served over HTTPS**;
  and the header must NOT be emitted on the onion site itself. An equivalent
  `<meta http-equiv="onion-location" content="…">` in the HTML head exists for static pages
  ([http.dev/onion-location](https://http.dev/onion-location)).
- Adoption is real but niche: a top-1M crawl found ~30 sites emitting it; support exists in
  Tor Browser, Brave (desktop), and Onion Browser on iOS
  ([ctrl.blog/entry/tor-onion-location-header](https://www.ctrl.blog/entry/tor-onion-location-header.html)).
  Honest read: niche adoption does not weaken it for us — it is the ONLY mechanism the
  browser itself surfaces, it costs one response header, and it degrades to nothing in
  non-Tor browsers (invisible header). Perfect risk/benefit shape.

### 1.3 "Convenient" — what the research actually supports (and one claim it does NOT)

- **Tor Browser is current and phone-first-viable in 2026:** the 15.0.x line is live
  (Firefox ESR 140 base); Android got Connection Assist in 14.5 (automatic bridge fallback
  when a direct connection fails) and biometric screen lock in 15.0
  ([blog.torproject.org/new-release-tor-browser-145](https://blog.torproject.org/new-release-tor-browser-145/),
  [blog.torproject.org/new-release-tor-browser-150](https://blog.torproject.org/new-release-tor-browser-150/)).
  Android is a first-class Tor Browser platform — consistent with this roadmap's phone-first
  bias.
- **QR honesty — the claim this pass could NOT verify:** no evidence of a built-in
  QR-scan-to-open feature in Tor Browser for Android was found. The design therefore MUST NOT
  depend on one. The flow that IS verified-standard Android behavior: any camera/QR app
  decodes the QR to a URL and offers it to the user's browser chooser (or default browser);
  Tor Browser registers as a browser and handles `.onion` URLs natively. So the QR layer
  works today via the OS, with one caveat designed for in O6: a `.onion` URL opened in a
  NON-Tor default browser fails to resolve. Hence the two-QR design (§4.6): the primary QR
  is the CLEARNET URL (works in every browser; Tor Browser users then get the
  Onion-Location pill automatically — one extra tap, zero typing), and a second,
  explicitly-labeled "анонімний доступ / Tor" QR carries the onion URL directly for users
  who already run Tor Browser. Nobody ever types 56 base32 characters — that is the whole
  convenience requirement, met without depending on any unverified browser feature.

### 1.4 Latency physics — where Tor honestly fits, and where it does not

Measured shape of onion-service traffic (cited): the path is **six relays** (client's 3-hop
circuit + service's 3-hop circuit meeting at a rendezvous point); the INTRODUCE1→RENDEZVOUS2
rendezvous handshake alone typically takes **0.5–1.5 s**; established-circuit round-trips run
high-hundreds-of-ms and are variance-heavy; first-visit page loads are seconds-class
([PoPETs 2025 onion-service performance study](https://petsymposium.org/popets/2025/popets-2025-0029.pdf);
[arxiv 2602.23560 §measurements](https://arxiv.org/html/2602.23560); historical baseline
[Loesing et al. 2008](https://www.freehaven.net/anonbib/cache/loesing2008performance.pdf)).
Throughput is single-circuit-bound (Mbps-class on a good day, far less under load).

Against P51's real budgets (§0 row 12):

| Traffic | Budget | Over onion circuits | Verdict |
|---|---|---|---|
| Order placement/advance (P37 JSON) | ≪ 4 KiB, ≪ 1/sec | +0.5–1.5 s per request; fits trivially in size | **WORKS** — ordering over Tor is a good citizen |
| Menu/static assets, MapPack | ≤ 8 MiB one-time | seconds-to-tens-of-seconds one-time fetch | **WORKS, degraded** — acceptable for a privacy-tier first load; named in §5.2 |
| Customer live-track view | 32 B @ ≤ 0.5 Hz | updates arrive with ~1 s+ jitter; ordering already handled (`is_out_of_order`, P51 M6) | **WORKS, degraded-honest** — the view lags ~1–2 s behind reality; label it, never fake it (§4.5) |
| Courier-side live navigation | continuous | 6-hop RTT + jitter against a moving-vehicle UI | **DOES NOT FIT — not offered over Tor.** The courier runs the operator's own stack; courier privacy is a different problem (cap-scoped position events, P51 M6) and Tor is the wrong tool for it |
| Hub-to-hub mesh frames | ~5–6 KiB @ ≤ 1/sec (P34 §4.2) | fits size-wise; latency tolerable for order events; **BUT the QUIC carrier physically cannot ride Tor** (SOCKS = TCP streams only, no UDP — §0 row 8) | **DEFERRED** with trigger — §5.3 |

**The honest one-sentence verdict:** Tor is a **client-facing privacy tier** for browsing,
ordering, and delayed-tolerant tracking — it is NOT a universal transport replacement, and
this blueprint does not sell it as one.

---

## 2. Scope — what P53 owns vs deliberately does NOT

**P53 owns (build items §4):**

| Item | Content | Wave |
|---|---|---|
| O1 | `OnionAddress` validated type + parse refusals (server crate) | W0 (code, buildable now) |
| O2 | `Onion-Location` response layer on `native-spa-server` + dual-listener discrimination + static meta-tag fallback | W0 |
| O3 | Pure QR encoder `kernel/src/qr_code.rs` (bytes → module matrix; shared with P52 K6) | W0 |
| O4 | `deploy/` tor sidecar artifacts: torrc fragment, systemd note, operator checklist (key custody, PoW on) | W0 (files) / W1 (operator runs them) |
| O5 | Auth-unchanged proof: P37's cap-gated routes re-bound to the onion listener, same 401/403 | W0 (lands with/after P37's r-tests) |
| O6 | Discovery surface: two-QR share panel spec (P48 hub surface consumer) + a11y/copy fallback | W0 spec, P48-consumed |
| — | Live onion service on a real hub host (address minted, header live, QR printed) | **W1 — with P37 (HTTP surface) + P45 (deployment); operator-run per §0 row 5 convention** |
| — | arti hosting migration; arti-client outbound; Tor mesh transport | **W2 — deferred, named triggers (§1.1, §5.3)** |

**P53 explicitly does NOT own (anti-scope, binding):**

- **NO auth/trust-model change of any kind.** A Tor client presents the same capability cert
  in the same `CAP_HEADER`; every red-line denial, revocation check, and 401/403 arm is
  identical on both listeners (O5 proves it). Anonymity is network-layer only. A diff that
  weakens, bypasses, or special-cases auth for onion clients is a scope violation regardless
  of test state.
- **NOT a substitute for PQ wire security.** Tor's own onion encryption is classical
  (x25519/ed25519; CGO is a relay-crypto upgrade, not PQ) — the hybrid ML-DSA-signed frames
  (MESH-10) and P36's TLS-default remediation remain the authentication/integrity authorities.
  Tor adds a network-privacy layer ON TOP; nothing here touches `proto-wire`/`proto-crypto`.
- **NOT moderation/abuse-evasion machinery.** The onion mirror serves the same content, same
  rules, same capability gates as the clearnet surface — one router, one policy (§4.2's
  single-`build_router` design makes a divergent onion-only surface structurally awkward on
  purpose).
- **NOT the mesh Tor transport** — designed-and-deferred in §5.3 (PROTOCOL cross-ref), owned
  by a future P34-family unit when its trigger fires. No `TransportImpl` code in P53.
- **NOT an arti dependency now** — §1.1's verdict; the cargo unlock is not requested.
- **NOT onion-balance / multi-instance scaling, NOT bridges/pluggable transports, NOT
  single-onion (non-anonymous) mode** — each is a named later unit if a real need appears;
  single-onion mode in particular is explicitly refused as default because it surrenders the
  hub-location privacy that is this phase's entire point.
- **NOT vanity onion addresses** (§1.1c).

---

## 3. Predefined types & constants (standard item 4 — named BEFORE implementation)

```rust
// ── tools/native-spa-server/src/onion.rs — NEW module (O1 + O2) ─────────────
/// The Onion-Location response header (Tor Browser ≥ 9.5 discovery pill).
/// Emitted ONLY on clearnet-listener responses, ONLY when an onion address is
/// configured, and (spec) only meaningful when the clearnet side is HTTPS.
pub const ONION_LOCATION_HEADER: &str = "Onion-Location";

/// v3 onion address: exactly 56 base32 chars (RFC 4648 lowercase, no padding —
/// alphabet a-z2-7) + ".onion". 62 bytes total. v2 (16-char) is dead since
/// 2021 and is REFUSED, not grandfathered.
pub const ONION_V3_HOST_LEN: usize = 62;

/// Validated v3 onion host. Constructed ONLY via parse — no pub field, no
/// Deref-to-str shortcut that would let an unvalidated string through.
pub struct OnionAddress(String);
pub enum OnionAddrError { BadLength(usize), BadCharset(char), BadSuffix, V2Rejected }
impl OnionAddress {
    /// Accepts "abc…xyz.onion" (56+6). Lowercases ASCII input; refuses v2
    /// length explicitly (distinct error — an operator pasting an old address
    /// deserves a precise message).
    pub fn parse(s: &str) -> Result<Self, OnionAddrError>;
    pub fn as_host(&self) -> &str;               // "…​.onion"
    pub fn location_for(&self, path_and_query: &str) -> String; // "http://….onion{p}"
}

/// Tower layer: adds Onion-Location to every response when `Some(addr)` AND
/// the serving listener is the clearnet one. Onion-listener responses NEVER
/// carry it (spec: the header must not appear on the onion site itself).
pub fn onion_location_layer(addr: Option<OnionAddress>) -> /* axum middleware */;

// ── tools/native-spa-server/src/main.rs — CLI additions (clap, same idiom) ──
//  --onion-address  / SPA_ONION_ADDRESS  : Option<String> → OnionAddress::parse
//        (parse failure = startup refusal with the typed error, never a warn-and-run)
//  --onion-ingress  / SPA_ONION_INGRESS  : Option<u16>    → second loopback
//        listener (127.0.0.1:<port>) that tor's HiddenServicePort forwards to;
//        responses on THIS listener omit Onion-Location. Absent ⇒ single-listener
//        behavior, bit-identical to today.

// ── kernel/src/qr_code.rs — NEW module (O3; pure, no I/O — kernel law §0 row 11)
/// QR model-2 encoder, byte mode, versions 1–10, EC level M — sized for URLs
/// ≤ ~200 chars (a 62-char onion host + path fits in v4-5; clearnet URLs in
/// v2-3). Output is a square module matrix; rendering is the consumer's job
/// (P48 surface, print export) — this module never touches pixels.
pub const QR_MAX_VERSION: u8 = 10;
pub struct QrMatrix { pub size: usize, pub modules: Vec<bool> } // row-major, size×size
pub enum QrError { TooLong { bytes: usize, max: usize }, Empty }
/// Deterministic: same bytes ⇒ same matrix (fixed mask selection by the
/// standard's penalty rules — ties broken by lowest mask id).
pub fn qr_encode(data: &[u8]) -> Result<QrMatrix, QrError>;

// ── deploy/tor-onion.torrc + deploy/README.md § Tor onion tier (O4) ─────────
# HiddenServiceDir /var/lib/tor/dowiz-hub/          # identity keys live HERE
# HiddenServicePort 80 127.0.0.1:<SPA_ONION_INGRESS>
# HiddenServicePoWDefensesEnabled 1                 # tor ≥ 0.4.8 Equi-X PoW
# (shape chosen to translate 1:1 to arti's proxy_ports on the §1.1 trigger)
```

Rejected alternatives (DECART one-liners): **`qrcode` crate** — rejected Wave-0: not cached
(§0 row 4) ⇒ needs the same unlock class as arti for a ~500-line pure algorithm the repo's
own style hand-rolls (precedent: in-repo FIPS-203, SHA3, Ed25519); revisit only if O3's
external-decoder verification (§4.3) fails repeatedly. **A separate onion vhost/router** —
rejected: one `build_router` serving both listeners is the anti-divergence guard (§2
anti-scope 3); the ONLY per-listener difference allowed is the Onion-Location header itself.
**Emitting the header unconditionally** — rejected: spec requires clearnet-HTTPS-only; an
onion response advertising itself is nonsense and a spec violation. **`arti` embedded now**
— rejected per §1.1 (experimental hosting + uncached dep); trigger named. **Single-onion
(non-anonymous) service mode** — rejected per §2 (surrenders the point).

---

## 4. Build items — spec → RED test → code, each with adversarial cases (items 3, 5)

### 4.1 O1 — `OnionAddress` (the validated type everything else hangs on)

Spec: §3. RED→GREEN: parse tests — a known-good 56-char v3 host round-trips; `as_host` and
`location_for("/menu")` produce exact expected strings. **Adversarial (designed to break):**
55/57-char inputs ⇒ `BadLength`; chars outside a-z2-7 (`0`, `1`, `8`, `9`, `-`, unicode
homoglyphs) ⇒ `BadCharset` with the offending char; missing/mis-cased `.ONION` suffix
handled (lowercase) vs `.onio` refused (`BadSuffix`); a 16-char v2 address ⇒ `V2Rejected`
specifically (not generic `BadLength` — the error message is part of the operator UX);
`location_for` with an empty path yields `http://…onion/` (never a bare host — Tor Browser
expects a URL). No checksum validation is claimed: v3's embedded checksum requires base32
decode + SHA3; a named follow-up if garbage-in becomes a real operator hazard —
length+charset already excludes accidental typos of every class except a valid-alphabet
transposition, and the W1 checklist (§4.4) includes a live reachability check that catches
those.

### 4.2 O2 — the Onion-Location layer + dual listener (the convenience mechanism)

Spec: §3's layer + CLI. Implementation shape: `main.rs` binds the existing public listener
AND (when `--onion-ingress` is set) a second `127.0.0.1` listener; **both serve the SAME
`build_router` output** — the onion path exists as a different way IN, never a different
site. The layer is inserted beside `security_headers` (`lib.rs:105` idiom) with a
per-listener flag. Static fallback: `web/index.html` head gains the
`<meta http-equiv="onion-location">` equivalent, populated at deploy time (documented in O4's
checklist), so even a cached static page advertises the mirror.

RED→GREEN (extends P37's `r1..r7` integration-test convention, §0 row 2 of P37): new
`r8_onion_location_present` — boot with `--onion-address <fixture>`, GET `/` on the public
listener ⇒ header present with exact value `http://<fixture>.onion/`; `r9_onion_listener_
clean` — same GET via the onion-ingress listener ⇒ header ABSENT, body byte-identical.
**Adversarial:** boot WITHOUT `--onion-address` ⇒ header absent everywhere (bit-identical to
today's responses — the zero-config regression guard); boot with an invalid address ⇒
process exits non-zero at startup with the typed error (refusal, never warn-and-serve — a
half-configured privacy tier is worse than none); path/query propagation
(`/menu?x=1` ⇒ `…onion/menu?x=1`); header-injection teeth — an address that somehow contained
CR/LF cannot exist post-parse (charset refusal is the guard; test asserts construction is
the only path in).

### 4.3 O3 — QR encoder (kernel-pure; one encoder, two consumers)

Spec: §3. Byte-mode-only, EC-M, versions 1–10, deterministic mask choice. RED→GREEN:
(i) `qr_matrix_fixture_exact` — two committed test vectors (a short string and a 62-char
onion host + `http://` prefix) assert the FULL expected module matrix bit-for-bit (fixtures
generated once from an independent reference encoder, provenance recorded in the test
comment); (ii) determinism — two calls, identical matrix; (iii) capacity — a payload one
byte over v10-M capacity ⇒ `QrError::TooLong` with both numbers, NEVER truncation.
**Adversarial:** empty input ⇒ `Empty`; every version boundary payload size (v1↔v2 … v9↔v10)
encodes without panic (loop test); quiet-zone is NOT baked into the matrix (consumer
concern — asserted by `size` matching the standard's 21+4(v−1) exactly). **External
verification (Hermetic P7, once, recorded):** render the two fixture matrices to PNG in a
scratch script, decode with an independent decoder (zbar/phone camera), paste the decoded
strings into the PR. A QR encoder that only its own tests can read is self-certification;
this step is the independent witness. Consumers: O6's share panel and P52 K6's enrollment
QR (cross-referenced there; ONE encoder — a second QR implementation appearing anywhere is
a P2-CORRESPONDENCE violation).

### 4.4 O4 — the tor sidecar deploy tier (files now, operator runs at W1)

Artifacts, all in `deploy/` per §0 row 5's idiom: `tor-onion.torrc` (§3's fragment, PoW
defenses ON — tor ≥ 0.4.8 asserted in the checklist), a README section covering: install
(`apt install tor`), enable, **key custody** — `HiddenServiceDir` contains
`hs_ed25519_secret_key`, which IS the onion identity: back it up offline with the same
discipline as hub secrets (S3 EnvFile doctrine; losing it = losing the published address;
leaking it = someone else can BE the address) — 🔴 this file is red-line-secrets-adjacent
and the checklist says so; the deploy-time steps to read the minted hostname
(`/var/lib/tor/dowiz-hub/hostname`), feed it to `--onion-address`/meta-tag, and verify
reachability via Tor Browser; and the explicit statement that every step is a HOST action
run by the operator, not the agent (`deploy/*.service` convention, verbatim). RED→GREEN at
W0: a repo test asserting the torrc fragment parses (line-shape lint: exactly the three
directives, PoW line present) — a config-drift tripwire, honest about being a lint, not a
network test. **Adversarial (checklist drills, W1, recorded once):** stop the tor sidecar ⇒
clearnet service unaffected (isolation proof §5.2); `curl --socks5-hostname 127.0.0.1:9050
http://<addr>.onion/healthz` from a second host succeeds and its timing is RECORDED as the
deployment's baseline latency number (§7's no-Tor-in-CI honesty).

### 4.5 O5 — auth unchanged over the onion path (the trust-model proof)

No new code — a test obligation: P37's cap-gated route tests (`r6`-family: request without
`CAP_HEADER` ⇒ 401; bad chain ⇒ 403) re-run bound to the onion-ingress listener. GREEN =
identical status codes and bodies on both listeners. **Adversarial:** the test that must
stay red-impossible — grep-lint asserting `onion` appears nowhere in `api.rs`/cap-check
code paths (the auth code cannot even SEE which listener served the request; discrimination
lives only in the header layer — unsafe state "onion clients get a different auth path" is
unrepresentable because no bit of listener identity reaches the checker). Tracking-view
staleness honesty (from §1.4): the customer view consuming `TrackFrame` over Tor inherits
P51's stale-label rule (P52 DoD-3 "stale track labeled, never presented live") — asserted
there, cited here; latency does not create a new lying-UI path.

### 4.6 O6 — the two-QR discovery surface (P48-consumed spec)

Spec for the P48 owner-hub "share access" panel (P48 builds it; P53 defines it so the
convenience layer has a concrete owner): (a) **primary QR** = clearnet URL (`qr_encode` of
`https://<hub-host>/`), captioned as the normal entry — works in every browser; Tor Browser
users who scan it get the Onion-Location pill with zero extra work; (b) **second QR**,
explicitly labeled "Анонімний доступ (Tor Browser)" = `http://<addr>.onion/` — for
customers already in the Tor world; scanning it in a non-Tor browser fails to resolve,
which is why it is labeled and secondary, never the default (the §1.3 honesty). (c) the
onion host rendered as selectable/copyable TEXT beside the QR (a11y + desktop users +
paste-into-Tor-Browser), via the a11y-mirror DOM path until MSDF text lands — the exact
P51 M7 attribution precedent. (d) print-export note: the same two QRs on a venue
counter-card is the physical-world convenience layer (scan at the table). RED→GREEN: panel
spec test lands WITH P48's surface (cross-referenced in its blueprint's consumption of
this section); W0 deliverable is this spec + the encoder it needs (O3). **Adversarial:**
the onion QR must encode `http://` (not `https://`) — onion transport is already
end-to-end encrypted and v3-authenticated; a https-onion URL would demand a cert the hub
does not have and produce a browser warning — the test pins the scheme.

---

## 5. Cross-cutting design obligations (items 6, 8, 9, 11-16)

### 5.1 Hazard-safety as math (item 6)

- **Divergent onion surface (content or policy fork):** unreachable by construction — one
  `build_router` serves both listeners (§4.2); the only per-listener bit lives in the
  header layer. A fork would require building a second router, which no config flag can do.
- **Auth bypass for anonymous clients:** unrepresentable — listener identity never reaches
  the cap checker (§4.5's grep-lint + the layer-locality argument). Same `verify_chain`,
  same `RevocationSet`, same red-line denials.
- **Onion identity-key loss/leak:** not structurally preventable (it is a file on a host) —
  mitigated by process: the O4 checklist names custody explicitly, marks the dir
  red-line-secrets-adjacent, and the key never enters the repo (gitleaks + S3 doctrine
  unchanged). Honest classification: this is the one genuinely operational (non-math) risk
  this phase adds.
- **De-anonymization of the hub via the clearnet mirror:** out of P53's threat model BY
  DESIGN and stated so the operator is never misled — a hub serving BOTH clearnet and onion
  is locatable via its clearnet side; the onion tier protects the CLIENT's network privacy
  always, and protects the HUB's location only in an onion-only deployment (a W1 checklist
  note names this; onion-only is a legitimate per-hub M5 choice requiring no code change —
  just don't bind the public listener).
- **Header injection via config:** post-parse `OnionAddress` cannot contain CR/LF/spaces
  (charset law, §4.1) — the only path to the header value is through the parse.
- **DoS via the onion ingress:** C tor's PoW + intro-DoS defenses (ON in the torrc, §4.4)
  + the existing server-side posture; the onion listener binds loopback only, so it adds
  zero new public attack surface on the host itself.

### 5.2 Schemas & scaling axes (item 8) + isolation (item 11)

No persistent schemas change. Scaling axis = onion-circuit throughput (single-circuit,
Mbps-class): order traffic ≪ budget by orders of magnitude (§1.4 table); the 8 MiB MapPack
worst case ⇒ seconds-to-tens-of-seconds one-time fetch on the privacy tier — accepted and
stated; break point = if a many-venue aggregation surface ever serves bulk assets over
onion, onionbalance/multi-instance is the named (deferred) mechanism. Isolation: the tor
sidecar is a separate OS process; its failure mode is "onion mirror down, clearnet
untouched" (drilled in §4.4); the reverse (server down) leaves tor forwarding to a dead
port — connection refused, no queue buildup. The header layer is pure per-response
computation — no shared state, no new bulkhead needed.

### 5.3 Mesh awareness (item 12) — the PROTOCOL cross-reference, designed-and-deferred

**Question 4's answer, committed here so it is never re-derived:** Tor as a hub-to-hub mesh
transport IS representable under M6's Trait-as-Port law — but not as a fourth impl of the
QUIC shape. The physical constraint (§0 row 8): Tor's SOCKS interface carries **TCP streams
only**; quinn/QUIC is UDP and cannot ride it. A `TorTransport` would therefore wrap an
arti-client `DataStream` (AsyncRead/AsyncWrite) under the same framing the **wss** carrier
uses — architecturally a sibling of `wss_transport.rs`, not of `iroh_transport.rs`. Budgets
fit (5–6 KiB frames @ ≤ 1/sec vs §1.4's latency: order events tolerate 1–2 s delivery);
onion services also give both-ends-dial-out NAT traversal for free — a genuine alternative
answer to the hole-punch unit P34B §3.3 deferred (cross-referenced there, absorbed by
neither). **Deferred because no requirement exists:** federating hubs have no demonstrated
need to hide their network location FROM EACH OTHER — the demonstrated need (operator ask)
is client-facing. Trigger to build: a hub operator requires an inter-hub link where at
least one endpoint's location must stay hidden from the other or from the path, OR direct
inter-hub connectivity is censored/blocked in a real deployment. Owner when triggered: a
P34-family unit (PROTOCOL), consuming §1.1's arti-client verdict; P53 never builds it.

### 5.4 Rollback / self-healing vocabulary (item 13) + living memory (item 15)

**Self-Termination leg claimed:** typed `OnionAddrError`/`QrError` refusals; startup
refusal on invalid config; unrepresentable auth-fork and header-injection states (§5.1).
**Self-Healing / Snapshot-Re-entry: NOT claimed** — nothing here is redundant-math or
epoch-recovering; the tor sidecar's restart is systemd's job (`Restart=on-failure` in the
unit note), named as ops, not dressed as math. Mechanical rollback: every item is additive
(one server module, one kernel module, deploy files, tests) — deleting them restores
today's tree; W1 rollback = disable the systemd unit. Living memory: reasoned exemption —
no stored data with temporal access patterns; the onion hostname is deploy-time config,
not corpus.

### 5.5 Linux discipline (item 9) + tensor/spectral/eqc (item 16)

**ALREADY-EQUIVALENT** — mechanism-not-policy: the server transports; whether a hub runs
the onion tier is per-hub policy (M5), exactly the pgrust-tier precedent; also
secure-by-default: no config ⇒ bit-identical behavior. **REINFORCES** — the
validated-config-type discipline (`OnionAddress` joins the parse-don't-validate family);
one-router-two-listeners extends the single-authority norm. **EXTENDS** — the deploy tier
gains its first third-party-daemon sidecar contract (torrc-as-lintable-artifact is a new
gate class, justified by the P36 lesson that unfenced "temporary" postures calcify).
**DOES-NOT-TRANSFER** — none claimed. Item 16: explicit reasoned N/A — QR is Reed-Solomon
over GF(256) (coding theory, not spectral); no eqc closed-form organ applies; the spectral
machinery is untouched.

---

## 6. DoD — falsifiable, RED→GREEN, per item (item 2)

| Item | RED (fails before) | GREEN (passes after) | Permanent regression (item 17) |
|---|---|---|---|
| O1 | no `OnionAddress`; parse tests absent | all §4.1 refusal arms + round-trip green | v2-rejection + charset tests |
| O2 | `r8`/`r9` absent; no onion concept in server (§0 row 3) | header exact on clearnet listener; absent on onion listener; absent with no config (bit-identical responses); invalid config = startup refusal | `r8`/`r9` + zero-config guard (ledger row) |
| O3 | no encoder (§0 row 6) | fixture matrices bit-exact; determinism; capacity refusal; external-decoder verification recorded in PR | fixture + capacity tests (ledger row) |
| O4 | no deploy artifacts | torrc fragment + README + checklist committed; torrc lint green; PoW line present | torrc-shape lint |
| O5 | no onion-listener auth proof | P37 cap tests green on BOTH listeners, identical codes; auth-blindness grep-lint green | dual-listener auth test (ledger row) |
| O6 | no discovery-surface spec | §4.6 spec consumed by P48's blueprint (citation present there); onion-QR scheme test pinned (`http://`) | scheme-pin test |
| W1 | — (operator-run) | hostname minted; Tor Browser reaches the hub; `curl --socks5-hostname` timing recorded; sidecar-kill drill recorded | checklist transcript in the deploy PR |

**Not-done clauses:** a design requiring the customer to TYPE the onion address = NOT done
(convenience IS the requirement — operator's word "зручної"); any auth difference between
listeners = NOT done regardless of green totals; an `arti` or `qrcode` cargo dep appearing
without its named trigger/unlock = NOT done; the header emitted on the onion listener =
NOT done (spec violation); courier live-navigation offered over Tor = NOT done (§1.4's
honest boundary).

---

## 7. Benchmark plan (item 10) — honest about what CI can and cannot measure

CI has no network and MUST NOT pretend to measure Tor. Split accordingly:

1. **In-CI (criterion, existing harness):** `qr_code/encode_onion_url` (62-char host +
   scheme/path — expect µs-class; the bench exists to catch an accidental O(n²) mask-penalty
   implementation, the classic QR-encoder mistake) and `onion/location_layer_overhead`
   (per-response cost of the header layer vs the bare router — expect ns-class; flatness
   proof in the P36 §6 sense). Baselines to `BENCH_HISTORY.md`.
2. **Deploy-time (W1, once, recorded — the ONLY honest Tor numbers):** `curl
   --socks5-hostname` `/healthz` timing ×10 from an off-site host (median + spread recorded
   in the deploy PR against §1.4's cited 0.5–1.5 s expectation — if measured reality is
   wildly worse, that is a finding for the L4 ledger, not a silent acceptance); one full
   first-load (menu + assets) timing over Tor Browser, recorded beside it.
3. **Telemetry:** the clearnet/onion split is deliberately NOT surfaced per-request in hub
   metrics beyond a listener-tagged request counter (local-only, M8) — counting is fine;
   profiling anonymous users is exactly what this tier promises not to do (stated as a
   design decision, not an omission).

---

## 8. Links to docs & memory (item 7)

Depends on / cites: `CORE-ROADMAP-STANDARD-2026-07-17.md` (contract) ·
`MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §9.2 L4 (the waiver this activates) +
§12/§13 (P51/P52 siblings; §14 = this phase's entry) · `CORE-ROADMAP-INDEX.md` §6 (L4 row
updated this pass) · `BLUEPRINT-P37-order-http-surface.md` (the HTTP surface O2/O5 extend;
`CAP_HEADER`; r-test convention) · `BLUEPRINT-P45-ops-security-monitoring.md` (deployment
tier W1 rides) · `BLUEPRINT-P48-owner-hub-surface.md` (O6 consumer) ·
`BLUEPRINT-P52-courier-working-surface.md` K6 (second QR consumer) ·
`BLUEPRINT-P34B-mesh-remaining-halves.md` §3.3 (deferred NAT unit; §5.3 cross-ref) ·
`BLUEPRINT-P36-bebop-remediation.md` (TLS posture untouched; the calcified-default lesson
behind O4's lint) · `BLUEPRINT-P51-open-map-routing.md` (budgets §0 row 12; a11y-text and
stale-label precedents; the structural template). Memory:
`rust-native-bare-metal-decision-2026-07-14` (DECART discipline; "older = adapters, no
purging" — the C-tor sidecar is exactly an adapter-tier call) ·
`never-bypass-human-gates-2026-06-29` (W1 is operator-run; key custody) ·
`secrets-exposure-incident-2026-07-03` (the custody discipline §4.4 inherits) ·
`verified-by-math-2026-07-07` + `test-integrity-rules-2026-06-27` (no fake-green; no
network-pretending CI) · `anu-ananke-strict-discipline-feedback-2026-07-17` (style; honest
rejections). Research sources: cited inline in §1 (Tor Project blog/docs/community pages,
docs.rs, PoPETs 2025, ctrl.blog, http.dev — all fetched 2026-07-18). Supersedes: the L4
row's "deferred" status everywhere it appears; nothing else — additive.

---

## 9. Hermetic principles honored (item 20 — load-bearing only)

- **P2 CORRESPONDENCE** (one concept, one primitive): one router for both ingresses; one
  QR encoder for both consumers (O6 + P52 K6); one validated address type as the sole path
  to the header; one deploy idiom for all sidecars.
- **P6 CAUSE-AND-EFFECT** (determinism as law): deterministic QR matrices with fixed mask
  tie-break; bit-identical zero-config responses; startup refusal over warn-and-drift.
- **P7 GENDER** (paired verification, no self-certification): the QR encoder is verified by
  an INDEPENDENT decoder (§4.3), not its own round-trip; the onion address is minted and
  checksummed by tor itself, not by us; arti's maturity is taken from the Tor Project's own
  published warnings rather than our preference for Rust — believing the vendor's
  self-criticism over our own bias IS the paired-witness discipline.

(P1/P3/P4/P5 not load-bearing here; not claimed decoratively.)

---

## 10. Standard-compliance map (all 20 points, checkable)

| §2 item | Where satisfied |
|---|---|
| 1 ground truth | §0 — 14 rows, live-verified; incl. the QUIC-cannot-ride-Tor finding and the uncached-dep constraint |
| 2 DoD | §6 (falsifiable rows + not-done clauses) |
| 3 spec/event-driven TDD | §3 spec-first; §4 RED-first throughout; §4.2 extends the existing r-test convention |
| 4 predefined types/consts | §3 (`OnionAddress`, `ONION_LOCATION_HEADER`, `QrMatrix`, CLI flags, torrc fragment verbatim) |
| 5 adversarial/breaking tests | §4.1 charset/length/v2 arms; §4.2 injection + refusal-at-startup; §4.3 capacity + external decode; §4.4 sidecar-kill drill; §4.5 red-impossible auth fork |
| 6 hazard-safety as math | §5.1 — six states, each unreachable-by-construction or honestly classified operational |
| 7 links docs/memory | §8 |
| 8 scaling axes | §5.2 (circuit throughput; MapPack worst case; onionbalance break point named) |
| 9 Linux discipline | §5.5 (all four verdict classes, EXTENDS justified by the P36 lesson) |
| 10 benchmarks+telemetry | §7 — CI/deploy-time split stated honestly; no fake network benches |
| 11 isolation/bulkhead | §5.2 (sidecar process boundary, drilled both directions) |
| 12 mesh awareness | §5.3 — the full PROTOCOL cross-ref: TCP-only constraint, wss-sibling shape, NAT note, trigger |
| 13 rollback/self-heal vocabulary | §5.4 — Self-Termination claimed with mechanisms; the other two legs explicitly refused |
| 14 error-propagation gates | §6 ledger rows; §4.5 grep-lint; §4.4 torrc lint |
| 15 living memory | §5.4 — reasoned exemption |
| 16 tensor/spectral + eqc | §5.5 — reasoned N/A (GF(256) coding theory, not spectral) |
| 17 regression ledger | §6 — four named ledger rows |
| 18 agent-executable instructions | §11 |
| 19 reuse-first | §0 rows 2/5/7 (headers middleware, deploy idiom, QR consumer); §1.1/§3 DECART rejections recorded; C-tor reuse over arti rewrite is the verdict itself |
| 20 Hermetic citations | §9 |

---

## 11. Clear instructions for other agentic workers (item 18 — zero session context assumed)

All W0 edits in `/root/dowiz`. T1-T4 are mutually independent (may fan out); T5 waits for
P37's api tests to exist; T6 is spec-consumption bookkeeping; W1 is OPERATOR-RUN.

1. **T1 (O1+O2).** Create `tools/native-spa-server/src/onion.rs` per §3 (types verbatim);
   RED-first: the §4.1 parse arms and §4.2's `r8`/`r9` integration tests (reuse the
   existing `spawn_server` helper in `tools/native-spa-server/tests/integration.rs`);
   then implement the type, the layer, the two CLI flags, the second listener. Acceptance:
   `cargo test -p native-spa-server` green incl. `r8`/`r9` + the zero-config bit-identical
   guard; startup refusal on an invalid address demonstrated in a test.
2. **T2 (O3).** Create `kernel/src/qr_code.rs` (NOT `qr.rs` — householder collision, §0
   row 6); register in `kernel/src/lib.rs` alphabetically. RED-first: commit the two
   fixture matrices + capacity/empty/determinism tests against a `todo!()` body; implement
   (byte mode, EC-M, v1-10, penalty-rule mask with lowest-id tie-break). Run the
   external-decoder verification once (render both fixtures, decode with zbar or a phone,
   paste results into the PR). Acceptance: `cargo test -p dowiz-kernel qr_code` green +
   the verification transcript.
3. **T3 (O4).** Add `deploy/tor-onion.torrc` (§3 fragment: three directives + PoW line) and
   the `deploy/README.md` "Tor onion tier" section per §4.4 (install, key custody 🔴,
   hostname read-out, meta-tag population, reachability check, sidecar-kill drill,
   operator-runs-it statement). Add the torrc-shape lint test. Acceptance: lint green;
   README section present; NO secret, NO real onion address committed.
4. **T4 (O6 spec).** Confirm §4.6 is cited from `BLUEPRINT-P48-owner-hub-surface.md`'s
   share-panel scope (add the citation there if absent — one line, append-style); add the
   onion-QR `http://`-scheme pin test beside T2's tests. Acceptance: cross-citation
   present; scheme test green.
5. **T5 (O5 — after P37's api.rs + r6-family land).** Re-bind the cap-gated route tests to
   the onion-ingress listener (parameterize the existing tests over both listeners; do not
   fork them); add the auth-blindness grep-lint (`grep -n onion tools/native-spa-server/
   src/api.rs` empty). Acceptance: both listeners return identical 401/403 arms.
6. **T6 (close-out).** Append the four §6 ledger rows to
   `docs/regressions/REGRESSION-LEDGER.md`; run the §7 in-CI benches once, record baselines
   in `BENCH_HISTORY.md`. Verify the master-roadmap §14 entry and the INDEX §6 L4 row
   (updated with this blueprint) are consistent.
7. **W1 (🔴 OPERATOR-RUN, with P37+P45):** execute the O4 checklist on the first hub host;
   record the hostname (never in-repo), the reachability transcript, the latency numbers
   (§7.2), and the sidecar-kill drill in the deploy PR.

**Stop-and-flag conditions (do not improvise past these):** (i) any impulse to add an
`arti` or `qrcode` cargo dep (triggers/unlocks named — §1.1/§3 — not yours to fire);
(ii) any listener-identity bit reaching auth code (§4.5's unrepresentable state — design
error, stop); (iii) the QR external-decoder verification failing (finding, not a reason to
weaken the fixture — the encoder is wrong); (iv) any onion hostname, `hs_ed25519_*` file,
or live address appearing in a commit (secrets discipline — stop and scrub before push);
(v) any W1 step executed by an agent instead of the operator; (vi) any request to make the
onion surface behave differently from clearnet beyond the header (§2 anti-scope 3 — that
is the moderation-evasion / policy-fork shape this blueprint explicitly refuses).
