# Lens C — Runtime, Transport & Identity for a Local-First Decentralized Delivery Hub

> **Research lens C** of the local-first-hub study. Question: can dowiz's hub logic actually *run
> on devices* (replacing Node/TS on Fly with a kernel Rust/WASM core + bebop2 PQ protocol), and if
> so with what transport and what identity/authz model, honestly?
>
> **Method:** read-only. No code in `bebop-repo` (which carries PRECIOUS uncommitted bebop2/core
> crypto) or `dowiz` was modified; the only file created is this report. Grounds cited with
> `file:line` (local) or URL (web). Every fresh technical claim is labeled **VERIFIED** (a source
> was fetched/confirmed this session) or **UNVERIFIED** (plausible, from docs/assessment, not
> independently confirmed). Web research on WASM/mobile-Rust runtimes and P2P transport was run
> fresh 2026-07-11.
>
> **Binding frame (non-negotiable):** NO single central server; hub logic runs on devices. This
> report is honest where that frame collides with 2026 mobile-OS reality — and it collides hard.
> Date: 2026-07-11.

---

## 0. The one-paragraph verdict (read this first)

The Rust/WASM kernel **can** run on every device class — but "runs on devices" does not mean "each
device is an always-on P2P server", because **no iOS app or PWA can be a background network node,
and Android can only with a foreground-service + battery-exemption fight** (§1). The honest target
is therefore **relay-assisted P2P, not pure serverless**: a device-held kernel that computes and
signs locally, gossiping over a mesh that falls back to **one small always-on relay** when phones
are asleep or behind CGNAT (§2). Identity cleanly replaces JWT+RLS: device-held PQ-hybrid keypairs
issue **signed capability tokens**, and dowiz's strongest security surface — the courier per-frame
WS re-authz (ADR-0013) — maps onto **per-frame signature + capability verification** almost
one-to-one (§3). The unavoidable cost: the offline/handoff guarantees (order-while-courier-offline,
device loss, clock skew on signed events, replay) demand exactly the machinery that pushes a pure
serverless design back toward a minimal relay + a recovery authority (§4). And a load-bearing
caveat threads all of it: the bebop2 PQ primitives are **hand-rolled and not yet FIPS-interoperable
or side-channel-audited** (G09) — they may only guard value as the PQ *half* of a hybrid, never
alone, until audited.

---

## 1. Per-device runtime matrix (audience × runtime × limits)

### 1.1 The three runtime candidates, at 2026 maturity

| Runtime | What it is | 2026 state | Fit for the kernel |
|---|---|---|---|
| **(a) Native Rust binary** | `bebop_core` compiled to the host triple; CLI/daemon | Mature. bebop already ships a `wasm32` core at **183 KB** and Docker/WASI/unikernel packaging tiers (`bebop-sovereign-node-DEPLOYMENT-2026-07-08.md §1-2`). | **Best.** Full threads, sockets, disk, background. The only place a device can truly host a server. |
| **(b) Rust→WASM in the browser** | `wasm32-unknown-unknown` via wasm-bindgen; runs in a tab or installed PWA | wasm-bindgen actively maintained (**0.2.126, 2026-06-24**, VERIFIED — github.com/wasm-bindgen/wasm-bindgen/releases). Threads+Atomics on iOS Safari ≥14.5 and Chrome Android (VERIFIED — caniuse.com/wasm-threads) but require COOP/COEP cross-origin isolation. SIMD128 universal. | **Good for compute, zero for background.** No install; the kernel's pure `decide/fold/replay` (`kernel.md`) runs fine. But a browser tab cannot hold a socket while backgrounded. |
| **(c) Rust in a mobile app shell** | Tauri 2 mobile, or (proven) a Rust core as a native lib behind Swift/Kotlin via UniFFI | **Tauri 2.0 stable 2024-10-02** with iOS+Android, but mobile DX "actively improving", not all plugins on mobile (VERIFIED — v2.tauri.app/blog/tauri-20). **The battle-tested pattern is Rust-core-as-native-lib**: Signal `libsignal`, Bitwarden `sdk-internal` (UniFFI), Mozilla app-services all ship a Rust core into official iOS/Android apps (VERIFIED — github.com/signalapp/libsignal, github.com/bitwarden/sdk-internal, github.com/mozilla/uniffi-rs). UniFFI is "production-ready, a long way from 1.0". | **The realistic mobile answer** — but the app shell buys packaging, not background freedom. The OS background limits (§1.3) bind regardless of shell. |

Server-class WASM (for the vendor node's sandboxed core) is mature: **wasmtime v46.0.1 (2026-06-24),
full WASI 0.2 + component model**; **WasmEdge v0.16.1 in Debian Jan 2026** (VERIFIED —
github.com/bytecodealliance/wasmtime/releases). This matches bebop's existing Phase-2 WasmEdge tier.

### 1.2 Storage & push on the no-install (PWA) path — the good news

The local-first data layer survives in a browser/PWA *if installed to the Home Screen*:

- **Storage quota is real once installed.** Safari 17+: origin quota up to **60% of disk** in the
  browser; **Home Screen web apps get browser-level quota** (not the 15% in-app-webview tier)
  (VERIFIED — webkit.org/blog/14403/updates-to-storage-policy). Chrome ~60% of disk shared across
  IndexedDB/OPFS (VERIFIED — web.dev/articles/storage-for-the-web). The signed event log
  (`store.ts` hash-chain, `kernel.md`) fits.
- **The 7-day ITP eviction cap does NOT apply to installed PWAs.** WebKit's own wording: *"Web
  applications added to the home screen … have their own counter of days of use"* — the counter
  resets each time the app is used, so script-writable storage (IndexedDB/OPFS/SW) is effectively
  exempt (VERIFIED — webkit.org/blog/10218/full-third-party-cookie-blocking-and-more). **A Safari
  *tab*, by contrast, loses all local-first storage after 7 idle days** — so the local-first claim
  requires "add to Home Screen", not a bare URL.
- **Web Push works on iOS ≥16.4, but only for Home-Screen web apps** (VERIFIED —
  documentation.onesignal.com/docs/en/web-push-for-ios); iOS 18.4 (Mar 2025) added Declarative Web
  Push (VERIFIED — webkit.org/blog/16535). Apple **reversed** the EU Home-Screen-app removal — PWAs
  continue, WebKit-only (VERIFIED — developer.apple.com/support/dma-and-apps-in-the-eu). Push is a
  *wake-to-notify* signal, not background compute.

### 1.3 Background execution — the killer, per OS (this decides "runs on devices")

**This is the single most decisive finding in lens C.** A courier's phone cannot run a delivery
server in the background. Sources VERIFIED unless noted.

- **iOS (through iOS 26.x): no persistent background node, full stop.** A suspended app holds **no
  sockets**. `BGAppRefreshTask` gives **~30s, OS-discretionary, shared across tasks**
  (developer.apple.com/documentation/backgroundtasks). iOS 26's new `BGContinuedProcessingTask`
  continues *user-initiated* work with a visible progress UI — it is **not a daemon**
  (developer.apple.com/documentation/backgroundtasks/bgcontinuedprocessingtask). The only persistent
  exceptions are location, audio, and VoIP/PushKit (CallKit-gated) — none of which a delivery-coord
  socket qualifies for without abuse. **Neither a native iOS app nor an iOS PWA can be an always-on
  P2P node.** Background sync on iOS Safari is entirely absent (no Background Sync / Periodic
  Background Sync / Background Fetch in any version — VERIFIED caniuse.com/background-sync).
- **Android (through Android 16, released 2025): possible, with friction.** A foreground service
  can hold a socket, but FGS types are mandatory (Android 14+) and **`dataSync`/`mediaProcessing`
  are capped at 6h per 24h** with an `onTimeout()`-then-crash (VERIFIED —
  developer.android.com/develop/background-work/services/fgs/timeout). A persistent socket needs a
  non-time-limited type (`connectedDevice`, or `specialUse` which requires Play-Console review —
  UNVERIFIED) **plus** a user battery-optimization exemption, because **Doze suspends network** for
  non-exempt apps (Google's sanctioned answer is FCM push, VERIFIED —
  developer.android.com/training/monitoring-device-state/doze-standby). **OEM task-killers remain
  real** — dontkillmyapp still ranks Xiaomi/Huawei/OnePlus/Samsung/Meizu as worst (VERIFIED —
  dontkillmyapp.com).

**Consequence for the architecture:** a phone is an *intermittent, push-wakeable* participant, not
a peer daemon. Real-time coordination needs a push-wakeable, always-reachable component. This is not
a dowiz limitation — it is the OS reality every stack (§2) is built around.

### 1.4 The per-audience verdict matrix

| Audience | Realistic runtime | Background-node verdict | Practical role |
|---|---|---|---|
| **Vendor / owner** (kiosk, back-office PC, or a €5/mo box) | **(a) Native Rust binary** — bebop's Docker/WASI/unikernel tiers (`bebop-sovereign-node-DEPLOYMENT §2`) | ✅ **Yes — the one true always-on node.** Can host the mesh entry-point, hold the authoritative event log, run the matcher, be the LAN rendezvous. | **The de-facto hub anchor.** Not a "central server" in the aggregator sense, but the vendor's *own* always-on node — the honest home of the "always-reachable" role §1.3 demands. |
| **Courier phone** | **(c) Rust core in a native app shell** (UniFFI/Signal pattern preferred over Tauri-mobile maturity); or **(b) installed PWA** for a no-install pilot | ❌ **No background daemon** (iOS: never; Android: 6h-cap + exemption + OEM-killer fight). Foreground: full kernel runs; can sign PoDs locally. | Computes & signs **while app is foreground**; otherwise **push-woken** (FCM/APNs/Web Push) to reconnect. The offline-courier gap in the current hub (§3.3) is exactly this OS wall. |
| **Customer phone** | **(b) Installed PWA** (no-install is the whole point for one-time customers); native app only for repeat users | ❌ No background node; ✅ push-to-notify only (iOS ≥16.4 Home-Screen). | Thin: build cart, place signed order, receive tracking push. Rarely needs the full kernel; a WASM verifier for its own order suffices. |

**Bottom line for §1:** the kernel is portable to all three device classes, so "runs on devices" is
*technically* true. But only the **vendor's own device** can be an always-on participant. The two
phone classes are intermittent signers/verifiers that must be **woken by push and reconciled on
reconnect** — which forces the relay-assisted topology of §2 and the offline guarantees of §4.

---

## 2. Transport without a central server — comparison + the honest relay floor

### 2.1 What bebop has today (the seam, not the wire)

bebop's transport is deliberately a **swappable port, not a protocol**:

- `mesh.md` defines a `MeshTransport` interface (`announce/query/pull`); today an **in-memory
  `Swarm`** implements it; "libp2p/hyperswarm implement the *same* port later — adding a real P2P
  backend is a swap, not a rewrite."
- `crates/bebop/src/zenoh.rs:1-11` is explicit: *"the seam, not the wire protocol … A real Zenoh
  would implement the same `Mesh` trait over the network; here we prove the routing/dispatch logic
  deterministically with no network, no rng, no clock."* It is a **process-local pub/sub stand-in**,
  not networked Zenoh.
- Content-addressing already gives verified exchange: a piece's address *is* its SHA-256, so "a
  malicious or buggy peer cannot inject bad data" (`mesh.md`). This is the property that makes
  *any* relay untrusted-safe (§2.4).

So the choice below is **which real backend to slot behind the existing port** — the kernel's
ordering/dedup logic is transport-agnostic by design.

### 2.2 The four candidates, 2026 (all facts VERIFIED unless noted)

| Transport | 2026 state | Discovery / rendezvous | NAT traversal | Mobile bindings | Needs infrastructure? |
|---|---|---|---|---|---|
| **Eclipse Zenoh** | **1.9.0 (2026-04-10)**, stable 1.x (crates.io/crates/zenoh) | Multicast scouting (UDP 224.0.0.224:7446) on LAN; gossip scouting fallback | **None built-in** — deployment docs don't address NAT; no STUN/hole-punch (zenoh.io/docs/getting-started/deployment; issue #78) | **Android yes** (zenoh-kotlin, Maven); **iOS: no official Swift binding** (zenoh-c #242) | Optional `zenohd` router for scale/brokering; **needs a reachable router to cross NAT** |
| **rust-libp2p** | **0.56.0 (2025-06-27)** — still 0.x and **no release in >12 months** (release-stall signal) | rendezvous + Kademlia DHT — but you run a bootstrap/rendezvous node or piggyback IPFS | AutoNAT + **DCUtR hole-punch ~70% ±7.1%** (VERIFIED — arXiv 2510.27500, 4.4M attempts Oct 2025) + circuit-relay v2 | No official iOS/Android; Rust cross-compiles. Browser: `webrtc-websys`, `webtransport-websys` crates | **Yes** — you operate bootstrap + relay + rendezvous |
| **Iroh (n0)** | **1.0 shipped 2026-06-15**, v1.0.2 (2026-07-06); wire+API stability commitment (iroh.computer/blog/v1) | Dial by **Ed25519 NodeId/public key** — key *is* the address | QUIC direct + relay fallback; **"95% of data passes directly"** (n0 claim, VERIFIED-as-claim); relays stateless-forward encrypted only | **Official Swift (iOS) + Kotlin (Android)** bindings (iroh-ffi); WASM/browser claimed in 1.0 (maturity UNVERIFIED) | Relay fallback — n0 runs free public relays; **relay binary is open-source + self-hostable** |
| **WebRTC data channels** | Browser-native; RTCPeerConnection **96.66% global** incl. iOS Safari 11+ (caniuse.com/rtcpeerconnection) | **App-provided signaling server (always)** | STUN for discovery; **TURN relay for ~17-20% of connections** (Hancke baseline; 40-70% in locked-down nets — order-of-magnitude VERIFIED, precise % UNVERIFIED) | Native in every mobile browser | **Yes** — you run signaling + STUN + TURN |

Notable alternatives: **Hypercore/Hyperswarm** (Holepunch/Pear, active into 2026, JS-centric, DHT +
own hole-punch, no published success rates); **Veilid** (Rust, iOS/Android/WASM, but veilid-core
**0.5.3** still young); and **BLE mesh (Bitchat, 2025)** — proven phone-to-phone, ~100 m, no
servers, **but had real impersonation-security problems and no external review** — relevant only for
the *same-location* courier-at-vendor handoff, not citywide coordination (all VERIFIED — see source
list).

### 2.3 The honest answer: "decentralized" in practice = P2P with relay fallback

**Pure serverless P2P between phones on cellular is not reliable, and every production stack agrees
by construction.** The evidence:

- Best-in-class hole-punching **plateaus at ~70%** (libp2p measurement, VERIFIED — arXiv 2510.27500);
  ~30% of attempts fail and fall to relay.
- Mobile carriers deploy **CGNAT**, which frequently behaves as endpoint-dependent ("symmetric")
  NAT where STUN-style punching fails and **relay is the only path** (arXiv 2311.04658; Tailscale/
  NetBird docs — prevalence % UNVERIFIED-precise, direction VERIFIED).
- **Every** stack studied — libp2p, iroh, WebRTC/ICE, Tailscale, hyperswarm — ships (a) a discovery/
  rendezvous mechanism and (b) a relay fallback. iroh, the strongest, still ships relays as a *core*
  component.

So the frame's "no central server" is honestly achievable **only** as: no central *business-logic*
server (the matcher/ledger stay decentralized per the blueprint's DANGER-#1 rule,
`UNIFIED-...-v3 §3`), while a **minimal, dumb, untrusted relay** forwards encrypted bytes when peers
can't reach each other directly.

### 2.4 The relay floor vs Fly today — and why it isn't re-centralization

**Minimal footprint: one small always-on publicly-reachable node.** Concrete options, cheapest
first:

1. **Self-hosted iroh relay** — open-source binary, **stateless, forwards only encrypted traffic**,
   never sees plaintext or business state. Cheapest operational answer (VERIFIED self-hostable).
2. A **coturn** STUN+TURN server (if the browser/WebRTC path is chosen).
3. A **zenohd** router (best if the vendor premises is the anchor and most traffic is LAN-side).

**Why this is not "DoorDash with extra steps":** the relay is a **transport hop, not a decision
point.** It cannot read, forge, or reorder business events, because (i) content-addressing already
rejects any tampered piece (`mesh.md`), (ii) every value-bearing event is signed by a device key
(§3), and (iii) the matcher/settlement logic runs on devices, not the relay (blueprint DANGER-#1/#3
mitigations). The blueprint's own invariant survives: *"decentralize the matcher, not just the
ledger"* — the relay decentralizes nothing and centralizes nothing; it just moves ciphertext.

**Footprint comparison:**

| | Today (dowiz on Fly) | Relay-assisted local-first target |
|---|---|---|
| Always-on servers | Node app + Postgres + worker + WS fan-out, **all trusted, all see plaintext + money** | **One dumb relay** (encrypted-forward only) + the vendor's own node holding *its own* data |
| Trust in the always-on tier | Total (RLS, JWT, DB all central) | **Zero** — relay is untrusted; trust lives in device keys |
| What breaks if it's seized/coerced | Everything (all tenants) | Only liveness of cross-NAT delivery; data + authority stay on devices; peers on a LAN keep working via multicast |
| Cost | Fly app + managed PG | One small VPS (or the vendor's own box doubling as relay on the shop's public IP) |

**Recommendation:** slot **iroh** behind the existing `MeshTransport` port as the internet-facing
backend (1.0-stable, Ed25519-keyed dialing that *aligns with bebop's self-cert identity §3*, official
mobile bindings, minimal self-hostable relay), and keep **Zenoh multicast** for the same-premises
LAN case (vendor+courier in the shop). rust-libp2p is powerful but 0.x, release-stalled a year, and
makes you run more infra for the same result. This is an assessment, grounded in the VERIFIED facts
above.

---

## 3. Identity & authz — replacing RS256 JWT + Postgres RLS with device keys + signed capabilities

### 3.1 What the current model is (the thing being replaced)

dowiz's production authz is a classic centralized stack (VERIFIED from code + memory):

- **RS256 JWTs**, kid+alg double-pinned; owner access-token TTL **24h**, refresh re-derives owner
  authority from **live memberships** (`role='owner' AND status='active'`) — ADR-0004
  (`adr-0004-owner-token-revocation`). The Rust port keeps RS256 (`rebuild/crates/api/Cargo.toml:75`
  `jsonwebtoken` use_pem).
- **Postgres RLS** as the tenant-isolation seam — but the NOBYPASSRLS flip is **deferred behind an
  8-item program** (`b3-auth-hardening-council-2026-07-03`): ~half the DML surface still bypasses
  `withTenant`. RLS is central, DB-resident, and not yet fully trustworthy even in the current
  design.
- **WS authz** rides the JWT: `wss://…` with `Sec-WebSocket-Protocol: bearer.v1,<jwt>`
  (`rebuild/.../ws/admission.rs:19-25`, keeping the token out of the URL/logs), then `subscribe`
  gated by `ownerCanAccessRoom` / courier binding (`cross-tenant-realtime-qa`).

### 3.2 The bebop replacement: self-cert identity → signed capabilities

bebop already has the identity substrate (VERIFIED from code):

- **Self-certifying PQ-hybrid identity** (`identity.md`, `bebop2/core`): `id = short_id(ML-KEM-768
  ek ‖ ML-DSA-65 spk ‖ X25519 ‖ Ed25519)`. A swapped/tampered key blob yields a *different* id →
  `self_certify()` refuses. **No CA, no directory** — you trust a node by its keys, mitigating the
  blueprint's DANGER-#4 (identity root). Vault sealed at rest with XChaCha20-Poly1305 + Argon2id,
  fail-closed on wrong passphrase.
- This maps directly onto iroh's **Ed25519 NodeId dialing** (§2.2) — the classical half of the
  hybrid id *is* the network address. Identity and transport addressing unify.

**The replacement mapping:**

| Current (central) | bebop replacement (device-held) |
|---|---|
| RS256 JWT signed by a central key | **Capability token** = a claim signed by the *issuer's device key* (hybrid ML-DSA-65 ⊕ Ed25519), verified against the issuer's self-cert id — no central signer |
| "role=owner" from a central `memberships` table | Owner mints a **signed capability** granting a courier a scoped right (e.g. "may read/relay `order:<id>` until `exp`"), signed by the owner's vault key |
| Postgres RLS row-scoping | **Capability scope + content-addressed event log** — a device only holds/serves events it has a capability for; tenant isolation becomes "which signed events can you decrypt/verify", not a DB policy |
| Central revocation (`status='active'` per request) | **Short-TTL capabilities + a signed revocation event** gossiped over the mesh (the reputation `KillSwitch`/suspension already exists, `reputation.rs`) |

The blueprint calls this exact seam: *"per-event content-hash + signature slot"* and *"per-actor PQ
identity … seams ready"* (`UNIFIED-...-v3 §5`). The signature slot is the JWT replacement.

### 3.3 Mapping the courier per-frame WS authz (dowiz's strongest surface) onto capabilities

This is the crux the task flags as "must survive." The current model (ADR-0013,
`adr-0013-courier-realtime-authz-dod`, and the Rust port `rebuild/.../ws/guard.rs`):

- **Admission** gates a *new* subscribe; but "a principal admitted and later revoked keeps streaming
  until disconnect unless **every frame is re-authorized at fan-out time**" (`ws/guard.rs:1-7`).
- A **tri-state `RelayGuard<Policy>`** (`ALLOW/DENY/UNAVAILABLE`, never throws) re-checks each
  courier's binding on **every** order-room frame, with a fixed ~10s TTL cache, an in-memory ~60s
  ceiling that fires *even under total DB starvation*, and `binding_revoked` eviction
  (`ws/guard.rs:44-68`, ADR-0013 C1). Relay-only-on-fresh-ALLOW; stale ⇒ **withhold, never
  relay-then-check**.

**The capability mapping (near one-to-one):**

| WS authz property (today) | Signed-capability equivalent |
|---|---|
| Per-frame re-authz against `courier_assignments` | Per-frame (or per-batch) **verify the courier's capability signature + `exp`** for `order:<id>`; the binding *is* an owner-signed capability |
| `binding_revoked` eviction | A **signed revocation event** for that capability, gossiped over the mesh; peers evict on receipt (fail-closed if unseen but TTL-expired) |
| Fixed TTL, no-refresh-on-read | **Capability `exp` = the TTL**; a device stops relaying when the capability expires, no callback needed — *strictly better offline*: expiry needs no server |
| In-memory ceiling under DB starvation | Same property **for free** — verification is local (signature + clock), needs no DB at all |
| `UNAVAILABLE` → withhold, retryable | Unseen-but-unexpired capability ⇒ withhold; can't verify freshness ⇒ don't relay |
| Session-liveness (logged-out-but-bound courier) `ws/guard.rs:20-30` | Session = a **short-lived capability**; logout = let it lapse or emit revocation. The "deactivation doesn't reset the binding" bug the port had to work around **disappears** — expiry is intrinsic |

**Key insight:** the current design already re-authorizes *every frame locally against a cache with
a hard ceiling that survives DB loss*. That is **exactly a capability-verification model wearing a
database costume.** Swapping the DB binding lookup for a signature+`exp` check preserves every DoD
property and removes the central DB dependency — the per-frame guard becomes *more* robust offline,
not less. This is the cleanest identity-migration story in the whole system.

### 3.4 What pod.rs / reputation.rs already provide

Two value-bearing primitives are **already built and tested** (VERIFIED from code):

- **`pod.rs` — Proof-of-Delivery.** `claim = "order:<id>|courier:<id>|at:<ts>|loc:<x,y>"`,
  `proof = vault.sign(SHA512(claim))` with hybrid ML-DSA-65 ⊕ Ed25519. `courier_id` is the
  *self-cert vault id, not PII* — pseudonymous, non-repudiable attribution. Fail-closed:
  `sign_delivery` refuses misattribution (`courier.id != claim.courier_id → None`, `pod.rs:73-83`);
  `verify_delivery` requires `self_certify()` + signature + id-binding. **Anti-replay is already
  present**: the claim binds `(ts, loc)`, and the test `pod_replay_at_wrong_location_fails`
  (`pod.rs:153-165`) proves a replay at a different location is rejected. This is the trustless
  anchor for the physical handoff (blueprint G7).
- **`reputation.rs` — deterministic trust ledger.** Valid PoD raises trust; a consensus suspension
  (`KillSwitch`) floors it to 0 and makes the node unreachable (`risk_premium → ∞`); unknown = 0.5
  "prove yourself"; recency decay; suspensions are *sticky* (never decay). Pure, RNG-free,
  auditable. This is the authz **input** for the open matcher — high-trust couriers preferred,
  suspended couriers excluded — replacing a central reputation service.
- **`zkvm.rs`** — an honest hash-commitment boundary (`receipt = H(prev‖input‖next‖meta)`), with a
  real STARK seam that **fails closed** without an injected verifier. This is the shape (not yet the
  substance) of a verifiable state-transition proof for settlement.

### 3.5 The honesty flag: hand-rolled crypto on value-bearing paths (cross-ref G09)

**This is the load-bearing caveat.** Per `G09-bebop2-crypto-assurance.md` (VERIFIED against
bebop2/core):

- bebop2 re-implements six primitives from scratch, zero-dep. Discipline is high (RFC/FIPS KATs, RED
  cases, a 3-model review that caught a real Ed25519 malleability bug) — **but KAT-green ≠
  constant-time ≠ side-channel-audited.**
- **The PQ set is not FIPS-interoperable by construction**: ML-KEM stores `t`/`s` in the coefficient
  domain (not NTT), ML-DSA samples `A` via CBD instead of uniform RejNTTPoly and uses a 32-byte
  (not 48-byte) challenge. So they are **bespoke lattice schemes, not the FIPS standards their names
  claim** — they cannot pass ACVP or match the `ml-kem`/`ml-dsa` crates until re-derived (G09 §2.1).
- **Real side-channel hotspots exist today**: ML-KEM `compress/decompress` divide by q=3329 on
  secret-derived data — **the exact KyberSlash class** (key recovery in ~4 min in the reference
  code); Ed25519 `scalar_mul` is a secret-scalar-dependent branch, not a Montgomery ladder
  (G09 §2.1).

**Therefore, the value-bearing policy G09 proposes must gate this design:** a bebop2 primitive may
only guard real value (PoD, reputation, escrow, capability tokens) **as the PQ half of a hybrid
whose classical half is an externally-audited implementation** (Tier 2) — never alone (Tier 3
requires an external audit + FIPS interop that don't exist yet). The existing `vault.rs` hybrid
(PQ ⊕ classical) is exactly right: even if the self-written ML-DSA is subtly broken, the audited
Ed25519 half still holds the signature. **Do not let a signed capability or a PoD rest on the
self-written PQ primitive alone.** This is the difference between "acceptable for a research
protocol" and "guarding a courier's payment."

---

## 4. The offline / handoff reality — what runtime + transport must guarantee

The §1 background wall + §2 relay reality mean the system is **eventually-consistent and
offline-first by necessity**. Four scenarios and what they demand:

### 4.1 Order placed while the courier is offline

**Reality:** the courier's phone is backgrounded/asleep (§1.3) — it is not listening. The current
hub already exhibits the failure: "a courier with a locked phone silently loses assignments" after
the 5-min accept-timeout sweep, because there is **no courier push** (hub review §4.6 gap #1).

**Requirements:**
- The order is a **signed event** persisted to the vendor's always-on node (§1.4) and the mesh —
  it survives regardless of courier reachability.
- Dispatch must be **push-woken**, not socket-pushed: FCM/APNs (native) or Web Push (installed PWA,
  iOS ≥16.4) wakes the courier app, which then reconnects, pulls the signed offer from the vendor
  node/relay, and verifies it. Push carries a *wake signal*, not trusted state — the state is
  re-fetched and signature-verified.
- **Durable re-dispatch with a timeout** (the existing `courier_dispatch_queue` +
  `CourierOfferSweepWorker`, hub review §4.2) must live on the vendor's always-on node, and the
  offer must carry a **capability with an `exp`** so an offline courier's claim auto-expires without
  a server callback (§3.3).

### 4.2 Device lost / stolen (key custody)

**Reality:** self-cert identity means **the key IS the identity** — lose the vault, lose the ability
to sign as that node; a stolen unlocked vault can sign as the courier.

**Requirements:**
- **At-rest protection already exists**: vault sealed with XChaCha20-Poly1305 + Argon2id, fail-closed
  on wrong passphrase (`identity.md`). A stolen *locked* device yields nothing.
- **Revocation without a CA**: a lost device's id must be added to a **signed revocation/suspension
  event** — `reputation.rs`'s `KillSwitch`/suspension (floors trust to 0, `risk_premium → ∞`) is the
  existing mechanism; gossip it over the mesh so peers stop honoring that id's capabilities and PoDs.
- **Recovery = re-enrollment, not key recovery**: because there is no directory, a lost key can't be
  "reset" — the owner must issue a *new* capability to a *new* courier id (re-invite). This is a
  recovery *authority* (the vendor node), which is honest: fully keyless recovery is impossible in a
  self-cert model. This is a genuine limit to state plainly, not paper over.
- **PQ harvest-now-decrypt-later** is mitigated by the hybrid KEM (ML-KEM-768 ⊕ X25519,
  `identity.md`) — but see §3.5: the PQ half is unaudited, so the classical half must remain
  load-bearing.

### 4.3 Clock skew on signed events (the pure-core has no clock)

**Reality:** bebop's core is **provably clock-free** — "no timestamps at kernel … the core never
reads a clock" (`bebop2/ARCHITECTURE.md`; dowiz kernel: "Every command carries caller-supplied
`Ts`", hub review §3.2). Timestamps are *caller-supplied inputs*, so two devices can disagree on
"now", and a malicious device can lie about `at:`.

**Requirements:**
- PoD binds `at:<ts>` into the signed claim (`pod.rs`), so the timestamp is **non-repudiable but not
  trusted** — the signer asserts it; it isn't ground truth.
- The event log is **causally ordered by hash-chain**, not wall-clock: `event[n].hash =
  H(payload ‖ event[n-1].hash)` (`kernel.md`). **Causal order (happens-before) is the real ordering
  primitive**, not timestamps — this is the correct design for a clock-skewed distributed system.
- For disputes, a timestamp must be **corroborated** (e.g., the vendor node counter-signs receipt
  time, or a threshold of devices attests) rather than trusted from one device. The blueprint's
  **threshold settlement** (≥k-of-n sigs, `UNIFIED-...-v3 §L3`) is the pattern: no single clock, no
  single oracle. Bounded acceptance windows (reject events with `at:` too far from the receiver's
  clock) limit skew abuse without trusting any one clock.

### 4.4 Replay protection

**Reality:** a signed event replayed elsewhere/later must not double-count (a PoD replayed to claim a
second payment; an order replayed).

**Requirements (mostly already present):**
- PoD binds `(order_id, courier_id, ts, loc)` — replay at a **different location fails**
  (`pod.rs:153-165`, VERIFIED test). Add: replay for the *same* order must be idempotent at
  settlement (the order_id is single-use).
- **Idempotency already exists** end-to-end (canonical request hash, tenant-scoped key, replay-200 /
  reuse-422, hub review §3.1 + Rust `kernel/idempotency.rs`). Port it: an event's **content-address
  is its dedup key** — same bytes = same hash = deduped for free (`mesh.md` "Dedup is free: same hash
  = same bytes").
- **Nonce + `exp` on capabilities** (the cart-token doctrine already uses `{…, nonce, exp≤15min}`,
  hub review §1.1.3) — single-use nonce defeats capability replay; short `exp` bounds the window.
- **Monotonic per-node sequence** (`Envelope{seq, at, cause}`, hub review §3.2) — a receiver rejects
  a `seq` it has already folded, defeating log replay.

### 4.5 The guarantee summary (what runtime + transport must deliver)

| Guarantee | Provided by | Status |
|---|---|---|
| Order survives courier offline | Signed event on vendor always-on node + mesh; push-wake | Node hub exists; **push-wake for couriers is the #1 gap** (hub review §4.6) |
| Offer auto-expires without server | Capability `exp` (§3.3) | Design-ready (cart-token has `exp`); not wired to courier capabilities |
| Lost device revoked | Signed suspension gossiped (`reputation.rs` KillSwitch) | Primitive exists; gossip transport is the seam (§2) |
| No trusted clock | Hash-chain causal order + caller-supplied `Ts` + threshold corroboration | Core is clock-free (VERIFIED); threshold settlement is blueprint-Phase-2 |
| No replay | `(ts,loc)`-bound PoD + content-address dedup + nonce/`exp` + `seq` | PoD anti-replay VERIFIED; idempotency VERIFIED; capability nonce design-ready |
| At-rest key safety | XChaCha20-Poly1305 + Argon2id vault | VERIFIED (fail-closed) |
| Value not on unaudited crypto alone | Hybrid PQ⊕classical, audited half load-bearing | **Policy gate required (G09 §3.5)** — not yet enforced |

---

## 5. Synthesis — the honest architecture lens C recommends

1. **Runtime:** vendor device = native Rust always-on node (the honest "hub anchor", not a central
   server). Courier/customer phones = Rust-core-in-native-shell (Signal/Bitwarden pattern) or
   installed PWA — **intermittent, push-woken signers/verifiers**, never background daemons. The
   iOS-background wall (§1.3) is non-negotiable and shapes everything.
2. **Transport:** iroh behind the existing `MeshTransport` port for the internet path (Ed25519-keyed
   dialing aligns with self-cert identity; official mobile bindings; self-hostable stateless relay),
   Zenoh multicast for same-premises LAN. **"Decentralized" honestly means relay-assisted P2P** — one
   dumb, untrusted, encrypted-forward relay is the minimal floor; it decentralizes nothing and
   centralizes nothing (§2.4). Far smaller trusted-surface than Fly today.
3. **Identity/authz:** self-cert PQ-hybrid keypairs issue **signed capability tokens** replacing
   RS256 JWT; the courier per-frame WS re-authz maps to **per-frame signature+`exp` verification**
   and gets *more* robust offline (§3.3); `pod.rs`/`reputation.rs`/`zkvm.rs` already provide the PoD,
   trust, and boundary-proof primitives.
4. **The gate:** none of this may put value solely on the hand-rolled PQ crypto — Tier-2 hybrid only,
   audited classical half load-bearing, until G09's external-audit + FIPS-interop path is walked.

**The one sentence:** the kernel *can* run on devices, but only the vendor's own device runs
always-on; phones are push-woken signers behind a minimal untrusted relay, their authority carried
in short-lived signed capabilities whose PQ half must never guard value alone.

---

## Sources

**Local (read-only, this session):** `bebop-repo/docs/design/UNIFIED-DELIVERY-PROTOCOL-BLUEPRINT-v3-2026-07-11.md`;
`bebop2/ARCHITECTURE.md`; `docs/features/{kernel,identity,mesh,governor}.md`;
`docs/design/bebop-sovereign-node-DEPLOYMENT-2026-07-08.md`;
`crates/bebop/src/{zenoh,zkvm,pod,reputation}.rs`; `dowiz/docs/research/2026-07-11-hub-architecture-review.md §4`;
`dowiz/docs/design/gap-blueprints-2026-07-11/G09-bebop2-crypto-assurance.md`;
`dowiz/rebuild/crates/api/src/{ws/guard.rs,ws/admission.rs,auth/}`, `Cargo.toml`; `dowiz/packages/config/src/index.ts`;
memory: `adr-0013-courier-realtime-authz-dod`, `adr-0004-owner-token-revocation`, `b3-auth-hardening-council-2026-07-03`, `cross-tenant-realtime-qa-2026-06-27`.

**Web (fetched/confirmed 2026-07-11):** wasm-bindgen releases; caniuse wasm-threads/simd/background-sync/rtcpeerconnection;
webkit.org storage-policy (14403) + ITP (10218) + Declarative Web Push (16535); developer.apple.com backgroundtasks + dma-and-apps-in-the-eu;
developer.android.com fgs/timeout + doze-standby + versions/16; dontkillmyapp.com; v2.tauri.app/blog/tauri-20;
github.com/{signalapp/libsignal, bitwarden/sdk-internal, mozilla/uniffi-rs}; bytecodealliance/wasmtime releases;
crates.io/crates/{zenoh,libp2p,veilid-core}; zenoh.io/docs deployment; github.com/eclipse-zenoh/{zenoh-kotlin,zenoh-c#242};
arXiv 2510.27500 (DCUtR ~70%); iroh.computer/blog/v1 + github.com/n0-computer/iroh-ffi; medium/rtcleague TURN figures;
arXiv 2311.04658 (NAT/CGNAT); docs.pears.com; veilid.com; techcrunch/wikipedia Bitchat.
Labels: VERIFIED = source fetched/confirmed; UNVERIFIED = assessment or secondary-source-only, flagged inline.
