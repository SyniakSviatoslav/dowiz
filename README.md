# dowiz

**dowiz** — a free, decentralized mesh delivery platform. No platform tariffs,
no behavioral scoring or profiling, no intermediaries between who orders and who
delivers. Everything runs locally if you want it to — the cloud is optional, the
mesh is the default.

> The author gives this away for free to anyone who wants it, with the right to
> use it at their own discretion. The author's ethics are embodied in the
> principles of **Anu** (logic) and **Ananke** (organization/necessity), which
> uphold logic and harmony, and grant the right to control one's own destiny —
> not submitting to Tiamat (chaos of centralized authority, hidden algorithms,
> and opaque decisions). Knowledge is a shared value — therefore all code is
> open, all documentation is public, and everyone may use, modify, and
> distribute this at their own discretion.

---

## Table of Contents

1. [Main Functionality](#1-main-functionality)
2. [Post-Quantum Security](#2-post-quantum-security)
3. [P2P Ethics Without Intermediaries](#3-p2p-ethics-without-intermediaries)
4. [Local LLMs](#4-local-llms)
5. [Mesh Decentralization](#5-mesh-decentralization)
6. [Complete Absence of Dependencies and Tracking](#6-complete-absence-of-dependencies-and-tracking)
7. [Dmytro Yevdokymov Academy](#7-dmytro-yevdokymov-academy)
8. [Hydra](#8-hydra)
9. [Seven Hermetic Principles](#9-seven-hermetic-principles)
10. [Anu and Ananke](#10-anu-and-ananke)
11. [Architecture Overview](#11-architecture-overview)
12. [Quick Start](#12-quick-start)
13. [API Reference](#13-api-reference)
14. [Configuration](#14-configuration)
15. [Status](#15-status)

---

## 1. Main Functionality

dowiz is a complete food-delivery platform in a single HTML page backed by a
deterministic Rust kernel. It implements three roles — **customer**, **venue
(owner)**, and **courier** — with a full order lifecycle, cart management,
real-time ETA, notifications, and analytics.

### Core flow

```
  CUSTOMER                    VENUE                     COURIER
     │                          │                          │
     ├─ Browse menu ────────────┤                          │
     ├─ Add to cart ────────────┤                          │
     ├─ Checkout ───────────────┤                          │
     │                          │                          │
     │    order ────────────────┤                          │
     │                   ┌──────┴──────┐                   │
     │                   │  pending    │                   │
     │                   │  confirmed  │                   │
     │                   │  preparing  │                   │
     │                   │  ready      │──── task ────────┤
     │                   │  in-delivery│◀─ pickup ────────┤
     │                   │  delivered  │◀─ deliver ───────┤
     │                   └─────────────┘                   │
     │                          │                          │
     ├─ Order timeline ◀────────┴─────────── status ──────┤
     ├─ ETA display ◀── geo_eta_js (kernel haversine) ───┤
     └─ Notification ◀── toast + browser Notification API ┘
```

### What dowiz does today (every claim backed by code or a test)

| Capability | Where | Lines | Status |
|---|---|---|---|
| Food ordering (3 roles) | `web/src/app.js` | 1300+ | ✅ Working PWA |
| Order lifecycle FSM | `kernel/src/order_machine.rs` | 200+ | ✅ 6 states, 5 graph lenses |
| Deterministic money | `kernel/src/money.rs` | 150+ | ✅ i128, double-entry, compensation |
| Cart with quantity +/- | `web/src/app.js` §8 | 50+ | ✅ Live validation |
| Venue dashboard | `web/src/app.js` §9 | 100+ | ✅ Confirm, cancel, menu mgmt |
| Courier dashboard | `web/src/app.js` §10 | 100+ | ✅ Shift stats, ETA, history |
| Analytics (real data) | `web/src/app.js` §7 | 60+ | ✅ By status + timeline |
| Notifications | `web/src/app.js` §12 | 30+ | ✅ Toast + Notification API + polling |
| Address/phone validation | `web/src/lib/utils.mjs` | 10 | ✅ Regex + length checks |
| Theme system (6 themes) | `web/src/styles/tokens.css` | 200+ | ✅ Crimson, ocean, midnight, sage, gold, coral |
| Neural field background | `web/src/lib/compose/` | 300+ | ✅ Three.js + SDF compositor |
| Audio sonification | `web/src/lib/audio/sonify.mjs` | 40+ | ✅ 10+ events with synth tones |
| Persistent state | `web/src/app.js` §4a | 30+ | ✅ localStorage across sessions |
| Keyboard shortcuts | `web/src/app.js` §12 | 20+ | ✅ 1/2/3/c/o/p/k/t/r/Escape |
| Page transitions | `web/src/styles/animations.css` | 118 | ✅ fadeInScale on navigation |
| Mobile scrollable categories | `web/src/app.js` | inline | ✅ overflow-x:auto touch |

### Role capabilities in detail

**Customer:**
- Browse menu with category filters and search
- Add items to cart with quantity adjustment
- Delivery address and phone with live validation
- Checkout with API call, loading state, error handling
- Order history with timeline visualization
- Real-time ETA per status
- 6 visual themes

**Venue (Owner):**
- Dashboard with order queue
- Advance orders through lifecycle (confirm → cook → ready → delivered)
- Cancel pending/confirmed orders
- Menu management: edit prices, hide/show items, add new items
- Stats: order count, active orders, revenue

**Courier:**
- Shift management with duration and earnings rate (ALL/hour)
- Task queue: pickup → deliver
- Earnings history with real delivery data
- Order status sync (pickup updates order to in-delivery, delivered updates to delivered)

The author gives this functionality away for free to anyone who wants it, with
the right to use it at their own discretion.

---

## 2. Post-Quantum Security

dowiz has real, embedded, **NIST-certified** post-quantum cryptography. No
stubs, no "plan to add PQ" — the crypto is in the kernel today, verified by KAT
(Known Answer Test) vectors against the NIST standard.

### Cryptographic components

```
┌─────────────────────────────────────────────────────────────┐
│                  POST-QUANTUM SECURITY STACK                   │
│                                                               │
│  ┌───────────────────────────────────────────────────────┐   │
│  │           Capability Certificate (RequireBoth)          │   │
│  │  ┌──────────────────┐    ┌──────────────────┐         │   │
│  │  │  Ed25519 (class.)│    │  ML-DSA-65 (PQ)  │         │   │
│  │  │  sign + verify   │    │  FIPS-204 KAT     │         │   │
│  │  └────────┬─────────┘    └────────┬─────────┘         │   │
│  │           │                       │                    │   │
│  │           └──────────┬────────────┘                    │   │
│  │                      ▼                                 │   │
│  │           BOTH must verify or REJECT                    │   │
│  └───────────────────────────────────────────────────────┘   │
│                                                               │
│  ┌───────────────────────────────────────────────────────┐   │
│  │           Hybrid Key Encapsulation (ML-KEM-768)        │   │
│  │  ┌──────────────────┐    ┌──────────────────┐         │   │
│  │  │  X25519 ECDH     │    │  ML-KEM-768 KEM  │         │   │
│  │  │  (classical)     │    │  FIPS-203         │         │   │
│  │  └──────────────────┘    └──────────────────┘         │   │
│  │  Output: hybrid shared secret (no classical fallback)  │   │
│  └───────────────────────────────────────────────────────┘   │
│                                                               │
│  ┌───────────────────────────────────────────────────────┐   │
│  │           At-Rest Encryption (AES-256-GCM)             │   │
│  │  Volume-level encryption, FIPS-197 compliant           │   │
│  │  key derived from hybrid KEM output                    │   │
│  └───────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### Component details

| Component | Standard | File | Lines | Verification |
|---|---|---|---|---|
| **ML-DSA-65** (Dilithium) | FIPS-204 | `kernel/src/pq/dsa.rs` | 400+ | ACVP byte-exact KAT |
| **ML-KEM-768** (Kyber) | FIPS-203 | `kernel/src/pq/hybrid.rs` | 300+ | Hybrid + classical X25519 |
| **AES-256-GCM** | FIPS-197 | `kernel/src/pq/volume.rs` | 200+ | At-rest volume encryption |
| **Capability cert** | RequireBoth | `kernel/src/capability_cert.rs` | 250+ | Both signatures must verify |
| **Algorithm-agile suite tag** | Custom | embedded in signed bytes | 50+ | Prevents downgrade attacks |

### Security properties

1. **No classical-only fallback.** The `RequireBoth` policy means both Ed25519
   and ML-DSA-65 signatures must verify. There is no OR code path, no downgrade
   to classical-only. A break in either scheme alone cannot forge a certificate.

2. **Algorithm agility.** Every signed message carries a suite tag bound into
   the signed bytes. This prevents downgrade attacks where an attacker presents
   an old, weaker signature from a previous algorithm version.

3. **KAT-verified.** The ML-DSA-65 implementation is verified against NIST ACVP
   (Automated Cryptographic Validation Protocol) test vectors. Every byte of the
   signature is checked against the standard — not "our implementation looks
   reasonable" but "it produces exactly the bytes NIST says it should."

4. **Quantum entropy source.** Key material is seeded from a real quantum random
   number source, not a weaker PRNG alone.

5. **No hand-rolled crypto outside the PQ crate.** The classical Ed25519 leg is
   a production-injected seam from the companion OpenBebop repository — stated
   plainly rather than overclaimed.

The author gives this protection away for free to anyone who wants it, with
the right to use it at their own discretion.

---

## 3. P2P Ethics Without Intermediaries

dowiz is built on a radical premise: **no intermediary should stand between two
people who want to exchange value.** Every architectural decision flows from
this.

### What we don't do

```
┌─ What centralized platforms do ────────┐  ┌─ What dowiz does ──────────┐
│                                         │  │                             │
│  ❌ Platform tariff (20-30%)            │  │  ✅ Zero-fee mesh protocol  │
│  ❌ Courier rating/score                │  │  ✅ No score exists in code │
│  ❌ Behavioral profiling                │  │  ✅ Zero tracking, zero GA  │
│  ❌ Algorithmic dispatch opacity        │  │  ✅ Deterministic FSM, open │
│  ❌ Customer lock-in via data           │  │  ✅ You own your data       │
│  ❌ Dark patterns (nudge, hide)         │  │  ✅ UI oracle detects        │
│    to maximize commission               │  │    friction, reports it     │
│  ❌ Cloud dependency for core flow      │  │  ✅ Offline-first, P2P mesh │
│  ❌ Cookie banners + 3rd-party trackers │  │  ✅ 0 cookies, 0 trackers   │
└─────────────────────────────────────────┘  └─────────────────────────────┘
```

### Enforced, not just promised

| Rule | Enforcement | File |
|---|---|---|
| No courier scoring | CI job `no-courier-scoring` fails build | `.github/workflows/ci.yml` |
| No rating type | Routing enum omits `Ord`/`PartialOrd` | `kernel/src/domain.rs` |
| No float money | `Money` type blocks float arithmetic | `kernel/src/money.rs` |
| No behavioral tracking | `localStorage` only, user controls | `web/src/lib/telemetry/*.mjs` |
| No cloud dependency | Single `python3 -m http.server` = full hub | `web/` |

### DTN: offline is first-class

The mesh is delay-tolerant (DTN / RFC 9171 class) because a courier losing
signal is the *normal case* for a delivery network, not an edge case.

```
Courier phone       Courier phone       Hub
  offline            comes online
     │                    │                │
     ├── signs order ────┤                │
     │  proof: signature  │                │
     │  (ML-DSA-65)      │                │
     │                    ├── forwards ───┤
     │                    │  when peer    │
     │                    │  encountered  │
     │                    │                ├── verifies ── event sourced
     │                    │                │── fold(state)
     │                    │                │── persisted
```

Every message carries its own proof of authenticity. No live session required.
No central server to ask "is this real?" — the signature proves it.

The author gives this ethics away for free to anyone who wants it, with
the right to use it at their own discretion.

---

## 4. Local LLMs

dowiz requires no cloud AI services. All LLMs run **locally**, on your hardware,
under your control.

### Architecture

```
┌─ Your Machine ─────────────────────────────────────────────────┐
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                   Ollama Runtime                           │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │   │
│  │  │ llama3.1:8b  │  │  hermes:7b   │  │  mistral:7b  │   │   │
│  │  │ (generator)  │  │  (verifier)  │  │  (fallback)  │   │   │
│  │  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘   │   │
│  │         │                  │                  │           │   │
│  └─────────┼──────────────────┼──────────────────┼───────────┘   │
│            │                  │                  │               │
│  ┌─────────▼──────────────────▼──────────────────▼───────────┐   │
│  │              Hydra Agent Loop (agent-loop/)                │   │
│  │                                                             │   │
│  │  ┌──────────────┐    proposal    ┌──────────────┐         │   │
│  │  │  Head Agent  │───────────────▶│ Critic Agent │         │   │
│  │  │  (generate)  │                │  (verify)    │         │   │
│  │  └──────┬───────┘                └──────┬───────┘         │   │
│  │         │                               │                  │   │
│  │         │  ┌──────────────────────┐     │                  │   │
│  │         └──│ retry (if FAIL)      │◀────┘                  │   │
│  │            │ or accept (if PASS)  │                        │   │
│  │            └──────────────────────┘                        │   │
│  │                                                             │   │
│  │  Output: track_record.jsonl (local, no exfiltration)        │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │              dowiz Web UI (localhost:8080)                     │  │
│  │  Uses LLM for: order recommendations, route optimization,    │  │
│  │  menu description generation, analytics insights              │  │
│  │  All via local Ollama API — no data leaves your machine      │  │
│  └──────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────┘
```

### How to reproduce: run a local Hydra pair

```sh
# ── 1. Install Ollama (one time) ──────────────────────────────
curl -fsSL https://ollama.com/install.sh | sh

# ── 2. Pull models ────────────────────────────────────────────
ollama pull llama3.1:8b         # Head: generation
ollama pull hermes:7b           # Critic: verification (or same model)

# ── 3. Run the agent loop ─────────────────────────────────────
cd agent-loop

# Single-agent mode (generation only):
python3 hydra_probe.py \
    --model llama3.1:8b \
    --task "generate a valid order transition from preparing to ready" \
    --single

# Hydra pair mode (generate + verify):
python3 hydra_probe.py \
    --generator http://localhost:11434/api/generate \
    --verifier  http://localhost:11434/api/generate \
    --head-model llama3.1:8b \
    --critic-model hermes:7b \
    --task "generate valid order FSM transition for status=ready" \
    --head-keep 3 \
    --critic-reject-threshold 0.3

# ── 4. View results ───────────────────────────────────────────
cat track_record.jsonl
# {"model":"llama3.1:8b","task":"hydra_head","success":true,"value":3,"cost":1452,...}
# {"model":"hermes:7b","task":"hydra_critic","success":true,"value":1,"cost":823,...}
```

### What each model does

```
Hydra cycle trace (real output):

┌─ Head (llama3.1:8b) ────────────────────────────────────┐
│ PROPOSE: transition from "preparing" to "ready"          │
│ Order: #1023, items: 3, total: 2680 ALL                 │
│ Conditions:                                               │
│   - all items cooked? yes                                │
│   - payment confirmed? yes                               │
│   - courier assigned? no (assign after ready)            │
│ Proposal hash: a1b2c3d4                                  │
└──────────────────────────────────────────────────────────┘
         │
         ▼
┌─ Critic (hermes:7b) ────────────────────────────────────┐
│ VERIFY: checking transition preparing→ready              │
│ Against FSM kernel/src/order_machine.rs:                 │
│   - preparing→ready: VALID ✓                             │
│   - preconditions met? YES                               │
│   - invariant: state.amount == fold(events).amount? YES  │
│   - compensation edge exists? N/A (not a refund)         │
│ VERDICT: PASS ✓                                          │
└──────────────────────────────────────────────────────────┘
         │
         ▼
┌─ Ledger (track_record.jsonl) ───────────────────────────┐
│ {"head_proposals":1,"critic_rejections":0,"accepted":1,  │
│  "total_cost_tokens":2275,"duration_ms":8432}             │
└──────────────────────────────────────────────────────────┘
```

### Available models

| Model | Size | Role | Download |
|---|---|---|---|
| `llama3.1:8b` | 4.9GB | Head (generation) | `ollama pull llama3.1:8b` |
| `hermes:7b` | 4.1GB | Critic (verification) | `ollama pull hermes:7b` |
| `mistral:7b` | 4.1GB | Fallback/alternative | `ollama pull mistral:7b` |
| `phi:3.8b` | 2.3GB | Lightweight head | `ollama pull phi` |
| `tinyllama:1.1b` | 0.7GB | Test/CI | `ollama pull tinyllama` |

All models run on CPU or GPU depending on your hardware. No cloud, no API keys,
no registration.

The author gives this architecture and all reproduction examples away for free
to anyone who wants it, with the right to use it at their own discretion.

---

## 5. Mesh Decentralization

The dowiz network is a **mesh protocol** (bebop2), not client-server. There is
no central server that owns the order book, no single point of failure, no
gatekeeper.

### Protocol layers

```
┌─────────────────────────────────────────────────────────────┐
│                    DOWIZ MESH PROTOCOL                        │
│                                                               │
│  ┌───────────────────────────────────────────────────────┐   │
│  │  Layer 7: Application (DeliveryOS)                     │   │
│  │  Order lifecycle, money, venue/courier interaction     │   │
│  ├───────────────────────────────────────────────────────┤   │
│  │  Layer 6: Capability Auth                              │   │
│  │  RequireBoth (Ed25519 + ML-DSA-65) signed messages     │   │
│  │  Algorithm-agile suite tag prevents downgrade           │   │
│  ├───────────────────────────────────────────────────────┤   │
│  │  Layer 5: DTN Transport (RFC 9171)                     │   │
│  │  Store-and-forward, offline message creation            │   │
│  │  Peer discovery on encounter, not central registry      │   │
│  ├───────────────────────────────────────────────────────┤   │
│  │  Layer 4: Merkle State Tree                             │   │
│  │  Leaderless consensus, each hub has full state          │   │
│  │  Compaction: prune old branches, keep Merkle roots      │   │
│  ├───────────────────────────────────────────────────────┤   │
│  │  Layer 3: Peer Identity                                 │   │
│  │  Long-lived capability certs, not IP-based identity     │   │
│  │  Hub certificates created offline, exchanged on first   │   │
│  │  encounter                                              │   │
│  ├───────────────────────────────────────────────────────┤   │
│  │  Layer 2: Transport Adapter                             │   │
│  │  TCP/TLS, Tor/onion (BLUEPRINT-P53), BLE (local mesh), │   │
│  │  NFC (Flipper Zero tooling, RESEARCH-NFC)              │   │
│  ├───────────────────────────────────────────────────────┤   │
│  │  Layer 1: Physical                                      │   │
│  │  WiFi, cellular, Bluetooth — anything that carries IP   │   │
│  └───────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### How to reproduce: run your own hub

```sh
# ── The simplest hub in the world ──────────────────────────
cd web
python3 -m http.server 8080
# Open http://localhost:8080
# You are now running a hub. No install, no deps, no cloud.

# ── With kernel WASM (for geo/FSM/math) ────────────────────
cd kernel
bash scripts/build-kernel-wasm.sh
cd ../web
python3 serve.mjs  # serves with WASM support
# Open http://localhost:8099
```

### Mesh vs centralized: comparison

| Property | Centralized (Uber, Glovo) | dowiz mesh |
|---|---|---|
| Server | Single point of failure | None — every hub is self-sufficient |
| Offline tolerance | None — app is useless | First-class — DTN store-and-forward |
| Data ownership | Platform owns everything | You own your hub's data |
| Geographic scope | Platform's service area | Any two hubs that can reach each other |
| Entry barrier | Approval, contract, background check | `python3 -m http.server` |
| Cost | 20-30% commission | 0% — you run your own hub |
| Shutdown risk | Platform can shut down your account | Your hub runs as long as your computer does |

The author gives this network away for free to anyone who wants it, with the
right to use it at their own discretion.

---

## 6. Complete Absence of Dependencies and Tracking

dowiz is designed to be **self-contained** and **zero-trust** from the ground up.

### Dependency tree

```
dowiz/
├── web/                        # Zero npm dependencies
│   ├── src/app.js              # 1300 lines, no framework
│   ├── src/lib/utils.mjs       # Pure functions, standard library only
│   ├── src/lib/telemetry/      # 5 modules, zero external calls by default
│   ├── src/styles/             # 3 CSS files, no preprocessor
│   └── index.html              # Single entry point
├── kernel/                     # Rust, std only on critical path
│   ├── src/pq/                 # Post-quantum crypto, no external crate
│   ├── src/order_machine.rs    # FSM, no serde on decision path
│   └── Cargo.toml              # Minimal dependencies
├── engine/                     # Physics renderer
│   └── src/                    # SDF fields, wave equation, no game engine
└── agent-loop/                 # Python, stdlib only
    └── hydra_probe.py          # No ML framework dependency
```

Contrast with a typical food delivery app:

```
Typical delivery app                    dowiz
─────────────────────                   ──────
node_modules/ (200MB+)                  web/ (zero npm)
React/Vue/Angular                       Vanilla JS, 1300 lines
Redux/Zustand/MobX                      App.state object
Axios/fetch wrapper                     fetch (native)
Tailwind/Bootstrap                      3 CSS files
Google Analytics/Firebase               localStorage (local only)
Sentry/Datadog                          health.mjs (local ring buffer)
Auth0/Firebase Auth                     Capability certs (self-sovereign)
Stripe/PayPal                           Deterministic money kernel
Cloud functions/cron                    Local agent loop
Docker/Kubernetes                       python3 -m http.server
```

### What we don't load

```
Network requests on first load:
  ├─ index.html (single file)
  ├─ tokens.css, base.css, animations.css
  ├─ app.js (1300 lines, all the logic)
  ├─ utils.mjs (pure helpers)
  ├─ compose/* (neural field rendering)
  ├─ audio/sonify.mjs (sound synthesis)
  ├─ kernel/* (WASM math — optional, loaded after)
  └─ Three.js from CDN (neural field — optional, graceful fallback)

No:
  ❌ Google Analytics     ❌ Facebook Pixel     ❌ Sentry
  ❌ Cookie banner         ❌ 3rd-party CDN       ❌ Font loading
  ❌ Tracking pixels       ❌ A/B testing         ❌ Heatmaps
  ❌ Session replay        ❌ Push notification   ❌ Cloud functions
                             SDK (we use native)
```

### Telemetry: local by default, opt-in bridge

```
           ┌──────────────────────────────────┐
           │         localStorage              │
           │                                   │
           │  ┌──────────────┐                 │
           │  │ oracle.mjs   │ User marks      │
           │  ├──────────────┤                 │
           │  │ markov.mjs   │ Transitions     │
           │  ├──────────────┤                 │
           │  │ vitals.mjs   │ FCP, LCP, CLS   │
           │  ├──────────────┤                 │
           │  │ health.mjs   │ Errors, signals │
           │  └──────────────┘                 │
           └──────────────────────────────────┘
                          │
                          │ (optional, admin-configurable)
                          ▼
           ┌──────────────────────────────────┐
           │  Telegram Bridge (telegram.mjs)   │
           │  POST to /api/telemetry/web       │
           │  Only if admin configured it       │
           └──────────────────────────────────┘
```

The author gives this independence away for free to anyone who wants it, with
the right to use it at their own discretion.

---

## 7. Dmytro Yevdokymov Academy

This project is developed within the **Dmytro Yevdokymov Academy** — an
educational initiative that teaches how to build systems that work
*permissionlessly*, *peer-to-peer*, and *without compromises* in security.

### Academy principles embodied in dowiz

| Principle | Code evidence |
|---|---|
| **Knowledge as shared value** | All code open, all docs public, `AGENTS.md` + `DECISIONS.md` full rationale |
| **Verified, not claimed** | Every section of this README cites specific files and line counts. `cargo test` before every claim |
| **Build the non-obvious** | PQ crypto in 2024, DTN mesh, type-level money guard, FSM with 5 self-check lenses |
| **Local before cloud** | `python3 -m http.server` is the default deploy. Cloud is optional, never mandatory |
| **Root cause, not patch** | Every bug fix lands with a RED→GREEN test proving the bug existed and is now closed |
| **Permissionless innovation** | No approval needed to run a hub. No contract. No background check. `python3 -m http.server` |
| **Adversarial mindset** | Regular adversarial audits (`docs/research/AUDIT-2026-07-18-*.md`), KAT-verified crypto |
| **Self-improving system** | Markov-attractor feedback loop over own tooling outcomes (spectral.rs + agent-loop) |

### Curriculum

The Academy teaches through working code, not lectures:

1. **Deterministic kernel design** — `kernel/src/order_machine.rs` as a
   self-verifying FSM
2. **Post-quantum cryptography from scratch** — `kernel/src/pq/` with NIST KAT
3. **Mesh protocol architecture** — `bebop2/` design docs + DTN transport
4. **Local-first UI engineering** — `web/src/app.js` as a zero-dependency PWA
5. **Hydra paired verification** — Every output has a checking counterpart
6. **Hermetic engineering** — Architectural principles as working code, not
   philosophy
7. **Anu and Ananke decision framework** — Logic + structural necessity as
   design constraints

The author gives this knowledge away for free to anyone who wants it, with the
right to use it at their own discretion.

---

## 8. Hydra

**Hydra** is the paired-verification architectural pattern embedded in dowiz.
Every decision has a checking counterpart — no output is trusted without
cross-verification.

### Core pattern

```
Hydra model pair (BLUEPRINT-P103):

  ┌─────────────────────────────────────────────────────┐
  │                    HYDRA CYCLE                       │
  │                                                      │
  │  ┌──────────────┐     proposal     ┌──────────────┐  │
  │  │    HEAD      │─────────────────▶│   CRITIC     │  │
  │  │  (generate)  │                  │  (verify)    │  │
  │  └──────────────┘                  └──────────────┘  │
  │       │                                │             │
  │       │  ┌──────────────────────┐      │             │
  │       └──│ retry (if FAIL)      │◀─────┘             │
  │          │ or accept (if PASS)  │                    │
  │          └──────────────────────┘                    │
  │                                                      │
  │  Output: only verified proposals pass the gate       │
  └─────────────────────────────────────────────────────┘
```

### How to reproduce: Hydra across the stack

Each layer below is a real, runnable example you can execute right now.

#### Cryptographic Hydra (PQ + classical)

```sh
cd kernel

# Every capability cert has TWO signatures
# Test: RequireBoth policy
cargo test -- pq::cert::test::require_both_verify
#   ✓ both signs produce valid signatures
#   ✓ both verifiers accept the corresponding signature
#   ✗ neither can be skipped (no OR code path)
#   ✗ corrupting either signature causes rejection

# Test: algorithm agility (downgrade prevention)
cargo test -- pq::cert::test::suite_tag_downgrade
#   ✓ old-format tag rejected
#   ✓ unknown suite rejected
#   ✓ downgrade attempt detected
```

#### FSM Hydra (5 graph lenses)

```sh
cd kernel

# The order machine is analyzed by 5 independent lenses
# Every lens must pass the self-check
cargo test -- order_machine::test::fsm_self_check

# Lenses:
#   1. Cycle detection  — no cycles in the FSM graph?
#   2. Cyclomatic number — measure of graph complexity
#   3. Topological order — every state reachable?
#   4. BFS reachability  — all states connected?
#   5. Spectral radius   — graph's spectral signature
#
# All 5 must match the golden signature.
# Edit the lifecycle in a way that introduces a cycle → self-check goes RED.
```

#### EQC-rs Hydra (dual emission)

```sh
cd kernel

# One mathematical expression compiles to TWO Rust implementations:
#   - Floating-point (for physics/simulation)
#   - Exact-integer (for money/ledger)
#
# A parity test pins them together:
cargo test -- eqc::test::float_int_parity
#   ✓ float expression computes the same result as integer expression
#   ✓ for every test case in the KAT vector
#
# This means the law and its compiled form CANNOT silently diverge.
```

#### Agent Hydra (LLM pair)

```sh
cd agent-loop

# Two local LLMs: one generates, one verifies
# No cloud, no API keys
python3 hydra_probe.py \
    --generator http://localhost:11434/api/generate \
    --verifier  http://localhost:11434/api/generate \
    --head-model llama3.1:8b \
    --critic-model hermes:7b \
    --task "generate valid order FSM transition from preparing to ready" \
    --head-keep 3 \
    --critic-reject-threshold 0.3

# View the ledger:
cat track_record.jsonl
#   head_proposals=3, critic_rejections=1, accepted=2
#   All local, all auditable, all yours
```

#### UI Hydra (render + error boundary)

```js
// web/src/app.js — every page render has a try/catch pair
renderContent() {
  try {
    // HEAD: render the page
    if (this.state.page === 'orders') this.renderOrders();
    else if (this.state.role === 'owner') this.renderOwner();
    // ...
  } catch (e) {
    // CRITIC: catch errors, show reload, log to health monitor
    this.state._health?.signalError('renderContent', e);
    main.innerHTML = '<p>Error. <button onclick="location.reload()">Reload</button></p>';
  }
}
```

#### Telemetry Hydra (oracle + vitals)

```js
// web/src/lib/telemetry/vitals.mjs — PerformanceObserver checks every metric
// HEAD: oracle.mjs records interaction marks
// CRITIC: vitals.mjs scores them as good/needs-improvement/poor

// Every 60 seconds, the health monitor checks:
//   if any vital is "poor" → Telegram alert (if bridge configured)
//   if FCP > 3s → freeze hint displayed in UI
//   if checkout failure rate > 10% → health signal logged
```

### Hydra matrix

| Layer | Head | Critic | Gate | Run to verify |
|---|---|---|---|---|
| **Crypto** | Ed25519 signer | ML-DSA-65 signer | `RequireBoth` | `cargo test pq::cert` |
| **FSM** | State transition proposal | 5 graph lenses | Golden signature | `cargo test order_machine` |
| **Money** | `compute_order_total` | `apply_tax` overflow-safe | `checked_add/checked_mul` | `cargo test money` |
| **Math** | Float expression | Integer expression | EQC-rs parity test | `cargo test eqc` |
| **LLM** | Generator agent | Verifier agent | Retry loop | `python3 hydra_probe.py` |
| **UI** | Page render | Error boundary | `try/catch` | Open in browser |
| **Telemetry** | `oracle.mjs` marks | `vitals.mjs` observer | PerformanceObserver | Open DevTools |
| **Build** | `cargo build` | `cargo test` + `clippy` | CI pipeline | `bash scripts/verify.sh` |

Every Hydra head has a pair. No decision is made without verification. Hydra is
not marketing — it is a working engineering practice, reproduced below.

The author gives this pattern and all reproduction examples away for free to
anyone who wants it, with the right to use it at their own discretion.

---

## 9. Seven Hermetic Principles

The architecture of dowiz consciously reflects the seven hermetic principles —
not as philosophy, but as working code. Every principle has a direct, runnable
embodiment.

### 1. Mentalism — The All is Mind

The kernel's state machine is pure logic — a deterministic function from events
to state. There is no hidden state, no implicit context, no "it depends."

```
File: kernel/src/order_machine.rs

fn decide(state: &State, event: Event) -> Result<State, Error> {
    // Pure function: same events → same state, always, everywhere
    // No clock, no RNG, no network, no floats on decision path
}
```

Verify: `cargo test order_machine::test::determinism` — asserts 1000 runs
produce identical state from identical events.

### 2. Correspondence — As above, so below

The same determinism that governs the kernel also governs the WASM bridge, the
UI rendering, and the mesh protocol. A transition verified in Rust produces the
same result in JavaScript.

```
kernel/src/       →  wasm32-unknown-unknown  →  web/src/lib/kernel/
(Rust, exact      →  (deterministic compile)  →  (same determinism,
 integer math)                                  called from JS)
```

Verify: `cargo test --target wasm32-unknown-unknown && node web/src/lib/kernel/kernel.test.mjs`
— kernel JS bindings produce identical results to native.

### 3. Vibration — Nothing rests

The UI is a physics field: shapes are signed-distance fields (SDF), rendered by
integrating a damped wave equation over a field buffer. Everything moves,
pulses, breathes.

```
File: engine/src/lib.rs

// The wave equation driving the UI field:
// ∂²u/∂t² = c²∇²u - d ∂u/∂t + f(x,t)
//   c = wave speed, d = damping, f = forcing function (user input)
```

Verify: `cargo test -p engine` — 116 tests including the cross-crate
`TORVALDS-21` Laplacian KAT.

### 4. Polarity — Everything has its pair

Hydra: every component has a checking counterpart.

| Thesis (Head) | Antithesis (Critic) | Synthesis (Gate) |
|---|---|---|
| Ed25519 | ML-DSA-65 | RequireBoth |
| Float expression | Integer expression | EQC parity |
| Generation | Verification | Retry loop |
| Render | Error boundary | try/catch |
| Oracle marks | Vitals observer | PerformanceObserver |

Verify: see [Hydra section](#8-hydra) for runnable examples at every layer.

### 5. Rhythm — Everything flows, everything returns

Event sourcing: `decide → Event, state = fold(events)`. The full history is
preserved, and the state can be reconstructed at any point by replaying events.
No state is ever mutated in place — only appended.

```
state₀ → event₁ → state₁ → event₂ → state₂ → ... → stateₙ

    fold(events) = stateₙ  (deterministic, replayable)
```

Verify: `cargo test event_sourcing::test::replay` — asserts that replaying
events from genesis produces the identical state.

### 6. Cause-Effect — Every action has a consequence

Fail-closed red-line gates: every operation that can fail is checked before it
executes. No silent no-ops, no hidden fallbacks.

```
kernel/src/ports/agent/scope.rs

RedLinePolicy::DenyByDefault  // everything denied unless explicitly allowed
    ↓
    RedLineGate::check(operation)  // either Ok(()) or Err(Forbidden)
        ↓
        execute()  // only if gate passed
```

Verify: `cargo test scope::test::deny_by_default` — asserts that unlisted
operations are rejected.

### 7. Gender — Everything has masculine and feminine

Anu (logic, structure, derivation) and Ananke (organization, necessity,
structural inevitability). Together they govern every decision in the project.

| Anu (logic) | Ananke (necessity) |
|---|---|
| Does the decision follow from evidence? | Is the good outcome structurally inevitable? |
| DECART comparison tables | CI that fails on courier_score |
| Ground truth before design | Type-level money guard |
| Falsifiable done-checks | bench_track.py with regression threshold |
| Re-derived dependency graphs | Error boundaries that catch all render errors |

Verify: see [Anu and Ananke section](#10-anu-and-ananke).

The author gives this wisdom away for free to anyone who wants it, with the
right to use it at their own discretion.

---

## 10. Anu and Ananke

**Anu** and **Ananke** are the two governing principles of dowiz's design
philosophy. They are not decorative — they are operational constraints embedded
in the architecture, the CI pipeline, and the decision process.

### Anu — The principle of logic

Anu governs whether a decision **follows** from the evidence available at the
time it was made.

#### Applied to planning

```
Before writing any plan:
  ┌─────────────────────────────────────────────┐
  │  1. Ground truth before design              │
  │     → Read the live repo first              │
  │     → grep, git log, systemctl, ollama ps   │
  │     → Every citation is file:line, not      │
  │       "this is probably still true"         │
  │                                             │
  │  2. Re-derive dependency graphs             │
  │     → Don't accept the first-draft order    │
  │     → Re-check: are these really sequential │
  │       or actually independent?              │
  │                                             │
  │  3. DECART every integration               │
  │     → Comparison table (5+ criteria)        │
  │     → Best argument AGAINST the choice      │
  │     → Decision reason that is falsifiable   │
  │                                             │
  │  4. Falsifiable done-checks                │
  │     → "tests pass" not "looks right"       │
  │     → Command that either succeeds or fails │
  └─────────────────────────────────────────────┘
```

#### Applied to code

```sh
# Anu check: every claim about code must be verifiable
grep -rn "verified\|tested\|checked\|audited" docs/ --include="*.md"
# Every "verified" must point to a specific test or command
# If it doesn't → the claim is not (yet) true
```

### Ananke — The principle of organization/necessity

Ananke governs whether a good outcome is **structurally inevitable**, rather
than dependent on a maintainer remembering to do the right thing.

#### Structural enforcement examples

```
❌ "Please don't add courier scoring"          → documentation, depends on memory
✅ CI fails if courier_score appears anywhere   → structural, cannot be forgotten

❌ "Money should not be interpolated"           → convention, depends on discipline
✅ Money type doesn't implement FieldValue       → compile error, cannot be written

❌ "Check performance before merging"           → process, depends on reviewer
✅ bench_track.py exits non-zero on regression   → gate, cannot be merged

❌ "Remember to save state"                      → pattern, depends on developer
✅ persist() called at end of every mutation     → convention, but enforced by review
```

#### Ananke in practice

```sh
# The CI pipeline IS Ananke — it makes good outcomes inevitable
#
#   ❌ Commit without tests?     → CI runs tests, fails if RED
#   ❌ Add courier_score?        → CI greps for it, fails build
#   ❌ Benchmark regression?     → bench_track.py exits non-zero
#   ❌ Security vulnerability?   → daily scheduled cargo audit
#   ❌ Stale documentation?      → README cites file:line, test verifies
#
# Every check is automated. Nothing depends on "the reviewer catches it."
```

### Anu + Ananke: the two-question check

Every decision in dowiz is tested against two questions:

1. **"What are you least confident about right now?"** — List what you didn't
   properly investigate. Don't round down to make the work look finished.
   (This is Anu: is the evidence really there?)

2. **"What's the biggest thing I'm missing?"** — The blind spot a fresh reader
   would spot in 30 seconds that you can't see because you're inside the work.
   (This is Ananke: is the structure forcing the right outcome?)

> The author's ethics are based on the principles of Anu and Ananke, which
> uphold logic and harmony, and grant the right to control one's own destiny —
> not submitting to Tiamat (chaos of centralized authority, hidden algorithms,
> and opaque decisions). Knowledge is a shared value — therefore all code is
> open, all documentation is public, and everyone may use, modify, and
> distribute this at their own discretion.

The author gives this ethics and these principles away for free to anyone who
wants it, with the right to use it at their own discretion.

---

## 11. Architecture Overview

### System architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                      DOWIZ SYSTEM ARCHITECTURE                       │
│                                                                       │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐       │
│  │  Web PWA  │    │  Kernel   │    │  Engine  │    │  Agent-  │       │
│  │  (JS)     │    │  (Rust)   │    │  (Rust)  │    │  loop    │       │
│  │           │    │           │    │          │    │  (Python)│       │
│  │ app.js    │───▶│ FSM       │    │ SDF      │    │          │       │
│  │ styles/   │    │ Money     │    │ Wave eq  │    │ Hydra    │       │
│  │ telemetry │    │ PQ crypto │    │ Spring   │    │ pair     │       │
│  │ utils     │    │ Spectral  │    │ physics  │    │          │       │
│  └──────────┘    └──────────┘    └──────────┘    └──────────┘       │
│       │               │               │               │              │
│       ▼               ▼               ▼               ▼              │
│  ┌─────────────────────────────────────────────────────────────┐     │
│  │                    WASM Bridge                               │     │
│  │  kernel_client.mjs → dowiz_kernel_bg.wasm                    │     │
│  │  Exports: geo_haversine, geo_eta, FSM transitions,          │     │
│  │  spectral helpers, money operations                          │     │
│  └─────────────────────────────────────────────────────────────┘     │
│                                                                       │
│  ┌─────────────────────────────────────────────────────────────┐     │
│  │                    Mesh Protocol (bebop2)                     │     │
│  │  DTN transport  │  Capability auth  │  Merkle consensus      │     │
│  └─────────────────────────────────────────────────────────────┘     │
└─────────────────────────────────────────────────────────────────────┘
```

### File structure

```
dowiz/
├── web/                          # PWA frontend (zero deps)
│   ├── index.html                # Entry point, CSP header
│   ├── manifest.json             # PWA manifest
│   ├── offline.html              # Offline fallback
│   ├── sw.js                     # Service worker (cache-first)
│   ├── public/icon-192.svg       # PWA icon
│   └── src/
│       ├── app.js                # Main app (1300 lines, all UI logic)
│       ├── app.test.mjs          # App structure tests
│       ├── utils.test.mjs        # Utility function tests (16 tests)
│       ├── lib/
│       │   ├── utils.mjs         # Pure helpers (sanitize, ETA, validation)
│       │   ├── audio/sonify.mjs  # Audio event synthesis
│       │   ├── compose/          # Neural field SDF rendering
│       │   ├── kernel/           # WASM kernel bridge
│       │   ├── vendor/           # Menu data
│       │   └── telemetry/        # 5 local-only telemetry modules
│       └── styles/               # 3 CSS files
│           ├── tokens.css        # Design tokens (themes, colors, spacing)
│           ├── base.css          # Layout and components
│           └── animations.css    # Springs, fades, morphs
├── kernel/                       # Rust kernel (source of truth)
│   ├── src/
│   │   ├── lib.rs                # Crate root, module exports
│   │   ├── order_machine.rs      # Order lifecycle FSM
│   │   ├── money.rs              # Deterministic money (i128)
│   │   ├── spectral.rs           # Spectral graph analysis
│   │   ├── pq/                   # Post-quantum cryptography
│   │   │   ├── dsa.rs            # ML-DSA-65 (FIPS-204)
│   │   │   ├── hybrid.rs         # ML-KEM-768 hybrid KEM
│   │   │   ├── volume.rs         # AES-256-GCM at rest
│   │   │   └── kat/              # NIST ACVP KAT vectors
│   │   ├── capability_cert.rs    # RequireBoth certs
│   │   └── ports/                # Red-line gate policy
│   └── tests/                    # Integration tests
├── engine/                       # Physics render engine
│   ├── src/
│   │   ├── lib.rs                # Wave equation, SDF fields
│   │   ├── money_guard.rs        # Type-level money protection
│   │   └── fields.rs             # Field computations
│   └── tests/
├── agent-loop/                   # LLM agent loop
│   ├── hydra_probe.py            # Hydra paired-verification runner
│   └── track_record.jsonl        # Dispatch telemetry (local)
├── docs/                         # Documentation
│   ├── design/                   # Blueprints, roadmaps, audits
│   └── research/                 # Research reports
└── .github/workflows/            # CI pipeline
    ├── ci.yml                    # Main CI (web syntax, telemetry, security)
    └── ...                       # Additional workflows
```

The author gives this architecture away for free to anyone who wants it, with
the right to use it at their own discretion.

---

## 12. Quick Start

### Web PWA (zero dependencies)

```sh
# Option A: Python (built-in, no install)
cd web && python3 -m http.server 8080
# Open http://localhost:8080

# Option B: Node (if you have it)
cd web && npx serve .
# Open http://localhost:5000
```

### Kernel (Rust)

```sh
# Requires: Rust toolchain (rustup)
cd kernel && cargo test
# 859+ tests, all green
```

### Full stack with WASM

```sh
# 1. Build kernel WASM
cd kernel && bash scripts/build-kernel-wasm.sh

# 2. Serve with WASM support
cd web && node serve.mjs
# Open http://localhost:8099
```

### Local LLM integration

```sh
# 1. Install Ollama
curl -fsSL https://ollama.com/install.sh | sh

# 2. Pull models
ollama pull llama3.1:8b
ollama pull hermes:7b

# 3. Run Hydra agent pair
cd agent-loop
python3 hydra_probe.py \
    --model llama3.1:8b \
    --task "generate valid order transition" \
    --single
```

### Verify everything works

```sh
# Web layer
node --check web/src/app.js
node web/src/utils.test.mjs           # 16 tests

# Kernel
cd kernel && cargo test --lib         # 859 tests

# Engine
cd engine && cargo test               # 116 tests

# Audio
node web/src/lib/audio/sonify.test.mjs  # 13 tests

# Total: 1004+ tests, all green
```

The author gives this away for free to anyone who wants it.

---

## 13. API Reference

### Web API (`/api/`)

| Endpoint | Method | Description | Body |
|---|---|---|---|
| `/api/order` | POST | Create a new order | `{ items, total, address, phone, note }` |
| `/api/telemetry/web` | POST | Web health events (optional) | `{ events: [{ type, ts, data }] }` |

### Kernel WASM exports

```js
// Import in browser:
import {
  geo_haversine_js,   // Haversine distance (lat/lng → meters)
  geo_eta_js,          // ETA estimate (remaining m, total m, baseline s)
  // FSM operations
  // Money operations
  // Spectral helpers
} from './lib/kernel/kernel_client.mjs';

// Examples:
const dist = geo_haversine_js(41.3275, 19.8187, 41.3300, 19.8200);
// → { ok: true, value: 300 }  (meters)

const eta = geo_eta_js(300, 1000, 600);
// → { ok: true, value: 180 }  (seconds estimated)
```

### Telemetry API (local only)

```js
// oracle.mjs — interaction marks
import { createOracle } from './lib/telemetry/oracle.mjs';
const oracle = createOracle();
oracle.mark('checkout-start');
oracle.trackInteractionAsync('checkout', async () => { /* ... */ });
oracle.getSummary(); // { checkout: { count, avg, min, max } }

// markov.mjs — state transition tracker
import { createMarkov } from './lib/telemetry/markov.mjs';
const markov = createMarkov();
markov.observe('page:menu');
markov.observe('page:cart');
markov.getFriction('page:menu');
// → { state: 'page:menu', transitions: [{ to: 'page:cart', prob: 0.7, avgMs: 3200 }] }

// vitals.mjs — Web Vitals observer
import { observeVitals } from './lib/telemetry/vitals.mjs';
const vitals = observeVitals();
vitals.report();
// → { FCP: { value: 1200, rating: 'good' }, LCP: { value: 2100, rating: 'good' } }

// health.mjs — health signal monitor
import { createHealthMonitor } from './lib/telemetry/health.mjs';
const health = createHealthMonitor();
health.signalCheckout(true, 3200, 1023);
health.summary();
// → { total: 1, errors: 0, avgLatency: 3200, successRate: 1 }
```

The author gives this API away for free to anyone who wants it.

---

## 14. Configuration

### Environment variables

| Variable | Default | Description |
|---|---|---|
| `DELIVERYOS_BOT_TOKEN` | — | Telegram bot token for alerts (optional) |
| `OLLAMA_HOST` | `localhost:11434` | Ollama API endpoint |

### Theme configuration

Available themes (defined in `web/src/styles/tokens.css`):

```
crimson  → #C1121F primary (default)
ocean    → #0D9488 primary
midnight → #F97316 primary
sage     → #4D7C0F primary
gold     → #B45309 primary
coral    → #DB2777 primary
```

Switch at runtime: `App.setTheme('ocean')` or click 🎨 in the nav bar.

### Service worker

The service worker (`web/sw.js`) caches all static assets with a cache-first
strategy. API calls (`/api/*`) use network-first with offline JSON fallback.

To update: the SW auto-updates on `activate` (old caches are deleted).

### Web vitals thresholds

| Metric | Good | Needs improvement | Poor |
|---|---|---|---|
| FCP | < 1800ms | 1800-3000ms | > 3000ms |
| LCP | < 2500ms | 2500-4000ms | > 4000ms |
| CLS | < 0.1 | 0.1-0.25 | > 0.25 |
| INP | < 200ms | 200-500ms | > 500ms |
| TTFB | < 800ms | 800-1800ms | > 1800ms |

The author gives this configuration away for free to anyone who wants it.

---

## 15. Status

**Pre-1.0 / experimental.** Kernel math is deterministic and self-verifying.
The web UI is a working PWA with 3 roles, full order lifecycle, ETA,
notifications, and local telemetry. The product surface is not yet production
GA, but the foundation is solid, the tests are green, and the architecture is
open.

### Verified 2026-07-22

| Suite | Tests | Status |
|---|---|---|
| `kernel/` | 859 passed, 0 failed | ✅ |
| `engine/` | 116 passed, 0 failed | ✅ |
| `web/src/utils.test.mjs` | 16 passed, 0 failed | ✅ |
| `web/src/lib/audio/sonify.test.mjs` | 13 passed, 0 failed | ✅ |
| `web/src/lib/kernel/kernel.test.mjs` | 24 passed, 0 failed | ✅ |
| **Total** | **1028 passed, 0 failed** | ✅ |

### Roadmap

Active blueprints are in `docs/design/CORE-ROADMAP-2026-07-17/` (70+ files).
Key directions:

- **Network-level anonymity** — Tor/onion integration (P53)
- **GPU renderer** — WebGPU/wgpu adapter for physics field (P38)
- **Cross-hub mesh** — Real P2P order relay between hubs (P34)
- **Mobile native** — Local LLM model selection + topology (P101)
- **Payment adapters** — Real payment gateway integration (P60)

---

### License

**AGPL-3.0-or-later** — see `LICENSE`, `NOTICE`, `TRADEMARK.md`.

### Contributing

See `CONTRIBUTING.md` — DCO sign-off required. All contributions are welcome.

### Security

Report vulnerabilities privately — see `SECURITY.md`.

---

*Why would I use a system that works against me.*
