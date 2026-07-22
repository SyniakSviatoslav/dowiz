# dowiz

**dowiz** — a free, decentralized mesh delivery platform. No platform tariffs on
participation, no behavioral scoring or profiling of any participant, no
intermediaries between who orders and who delivers.

> The author gives this away for free to anyone who wants it, with the right to
> use it at their own discretion. The author's ethics are embodied in the
> principles of **Anu** and **Ananke**, which uphold logic and harmony, and grant
> the right to control one's own destiny — not submitting to Tiamat. Knowledge is
> a shared value.

---

## 1. Main Functionality

What dowiz does today (every claim backed by code or a test):

| Capability | Where | Status |
|---|---|---|
| Food ordering (3 roles: customer, venue, courier) | `web/src/app.js` (1300+ lines) | ✅ Working PWA |
| Order lifecycle (pending → delivered) | `kernel/src/order_machine.rs` | ✅ FSM + 5 graph lenses |
| Deterministic money accounting (i128, no floats) | `kernel/src/money.rs` | ✅ Double-entry, compensation |
| Cart, quantity, address, phone validation | `web/src/app.js` §8 Cart | ✅ Live field validation |
| Venue dashboard (confirm, cook, ready) | `web/src/app.js` §9 Owner | ✅ Menu management + pricing |
| Courier dashboard (pickup, deliver, shift) | `web/src/app.js` §10 Courier | ✅ Shift stats + ETA |
| Analytics on real data | `web/src/app.js` §7 Renderers | ✅ By status + timeline |
| Notifications (toast + browser API) | `web/src/app.js` §12 Events | ✅ + 15s polling |
| Markov interface friction analysis | `web/src/lib/telemetry/markov.mjs` | ✅ Local, zero tracking |
| Web Vitals (FCP, LCP, CLS, INP, TTFB) | `web/src/lib/telemetry/vitals.mjs` | ✅ PerformanceObserver |
| Health monitoring | `web/src/lib/telemetry/health.mjs` | ✅ 200-event ring buffer |
| Telegram alert bridge | `web/src/lib/telemetry/telegram.mjs` | ✅ POST to /api/telemetry/web |

The author gives this functionality away for free to anyone who wants it, with
the right to use it at their own discretion.

---

## 2. Post-Quantum Security

dowiz has real, embedded, **NIST-certified** post-quantum cryptography. No
stubs, no "plan to add PQ":

| Component | Standard | File |
|---|---|---|
| ML-DSA-65 (signature) | FIPS-204, NIST ACVP byte-exact | `kernel/src/pq/dsa.rs` + KAT |
| ML-KEM-768 (hybrid KEM) | X25519 + ML-KEM-768 hybrid | `kernel/src/pq/hybrid.rs` |
| AES-256-GCM (at rest) | FIPS-197 | `kernel/src/pq/volume.rs` |
| Capability certificates (RequireBoth) | Classical + PQ signature — both must verify | `kernel/src/capability_cert.rs` |
| Algorithm-agile suite tag | Prevents downgrade | embedded in signed bytes |

**No classical-only fallback.** Certificates require both signatures (Ed25519 +
ML-DSA-65) with a `RequireBoth` policy — no OR code path. Entropy is seeded
from a real quantum random source.

The author gives this protection away for free to anyone who wants it, with the
right to use it at their own discretion.

---

## 3. P2P Ethics Without Intermediaries

dowiz is built on the principle of **no intermediaries between people**:

- **No platform tariffs** — every hub belongs to whoever runs it
- **No ratings** — zero `courier_score`, `rating`, or `reputation` in code
  (CI enforces: `no-courier-scoring` fails the build)
- **No behavioral profiling** — zero tracking, zero cookies, zero server-side
  telemetry
- **All telemetry is local** — `localStorage`, IndexedDB, no external requests
  (except an optional Telegram bridge under admin control)
- **Trust = signed capability certificate**, not a score
- **DTN (delay-tolerant networking)** — a courier losing signal is the normal
  case, not an edge case. Messages are signed offline and forwarded on peer
  encounter

The author gives this ethics away for free to anyone who wants it, with the
right to use it at their own discretion.

---

## 4. Local LLMs

dowiz requires no cloud AI services. All LLMs run **locally**, through:

| Component | Description |
|---|---|
| **Ollama** | Local model runtime (llama3.1, hermes, others) |
| **Hermes native** | BLUEPRINT-P21 — native MCP connection |
| **Hydra model pair** | BLUEPRINT-P103 — two agents (generate + verify) |
| **Agent loop** | `agent-loop/track_record.jsonl` — real LLM dispatch telemetry |
| **Markov feedback** | `kernel/src/spectral.rs` — closed-loop self-improvement |
| **Zero cloud dependency** | No API keys, no external LLM provider by default |

Every local model can be swapped via configuration — no hardcoded model.

The author gives this architecture away for free to anyone who wants it, with
the right to use it at their own discretion.

---

## 5. Mesh Decentralization

The dowiz network is a **mesh protocol** (bebop2), not client-server:

- **Store-and-forward (DTN/RFC 9171)** — messages live until they arrive
- **No central server** — hubs communicate directly
- **No libp2p-gossipsub** — reliability over latency (DECISIONS.md D3)
- **Post-quantum authentication** — every message carries its own proof
- **Merkle state tree** — leaderless consensus
- **Offline-first** — all transactions are created and signed offline

Deploy: `cd web && python3 -m http.server 8080` — you now have a working hub.
Zero dependencies, zero npm install, zero cloud.

The author gives this network away for free to anyone who wants it, with the
right to use it at their own discretion.

---

## 6. Complete Absence of Dependencies and Tracking

**dowiz runs with zero dependencies:**

```
web/src/ — no npm install, no node_modules
    app.js — 1300 lines, one file, no framework
    lib/utils.mjs — pure functions, zero deps
    styles/ — 3 CSS files, no Tailwind/Bootstrap

kernel/ — Rust, standard library
    cargo test — single command to verify
    zero cloud, zero serde on critical path
```

**Tracking:** zero. No:
- Google Analytics / Plausible / Umami
- Facebook Pixel / Twitter Pixel
- Sentry / Datadog / NewRelic
- Cookie banners (0 cookies)
- External telemetry requests
- Server-side IP logs

All analytics are local, in `localStorage`, under the user's full control.

The author gives this independence away for free to anyone who wants it, with
the right to use it at their own discretion.

---

## 7. Dmytro Yevdokymov Academy

This project is developed within the **Dmytro Yevdokymov Academy** — an
educational initiative that teaches how to build systems that work
*permissionlessly*, *peer-to-peer*, and *without compromises* in security.

Academy principles embodied in dowiz:
- **Knowledge as a shared value** — all code is open, all documentation is public
- **Verified, not claimed** — every assertion is backed by a test or code
- **Build the non-obvious** — don't copy existing solutions, find the real
  root cause
- **Local before cloud** — if it can run locally, don't depend on a server
- **Ethics of Anu and Ananke** — logic, harmony, self-determination, rejection
  of Tiamat (centralized authority, hidden algorithms, opaque decisions)

The author gives this knowledge away for free to anyone who wants it, with the
right to use it at their own discretion.

---

## 8. Hydra

**Hydra** is the paired-verification architectural pattern embedded in dowiz:

```
Hydra model pair (BLUEPRINT-P103):
  ┌──────────────┐     ┌──────────────┐
  │    Head      │────▶│   Critic     │
  │ (generate)   │     │ (verify)     │
  └──────────────┘     └──────────────┘
       │                      │
       ▼                      ▼
  Proposal             Refutation or
                       confirmation
```

Every Hydra head has a pair — no decision is made without verification.
This is applied in:
- **PQ cryptography** — RequireBoth: classical + PQ signature
- **FSM verification** — 5 graph lenses (cycles, topology, BFS, spectrum)
- **EQC-rs** — dual emission: float + exact integer, parity-pinned by test
- **Agent loop** — Markov-attractor feedback over own metrics
- **UI oracle** — Markov friction detection + freeze notifications

Hydra is not marketing — it is a working engineering practice. Every component
has a pair that checks it.

The author gives this pattern away for free to anyone who wants it, with the
right to use it at their own discretion.

---

## 9. Seven Hermetic Principles

dowiz's architecture consciously reflects the seven hermetic principles:

| Principle | Embodiment in dowiz |
|---|---|
| **1. Mentalism** — The All is Mind | `kernel/src/spectral.rs` — spectral graph analysis of state |
| **2. Correspondence** — As above, so below | Kernel → WASM → UI — same determinism at every layer |
| **3. Vibration** — Nothing rests | `engine/` — wave equation, SDF fields, spring physics |
| **4. Polarity** — Everything has its pair | Hydra: head + critic, PQ + classical, float + exact integer |
| **5. Rhythm** — Everything flows, everything returns | Event sourcing: `decide → Event, state = fold(events)` |
| **6. Cause-Effect** — Every action has a consequence | Deterministic kernel, fail-closed red-line gates |
| **7. Gender** — Everything has masculine and feminine | Anu (logic, structure) + Ananke (organization, necessity) |

Every principle is not a declaration — it is working code.

The author gives this wisdom away for free to anyone who wants it, with the
right to use it at their own discretion.

---

## 10. Anu and Ananke

**Anu** — the principle of logic. Does the decision follow from evidence? Does
the dependency graph hold when re-derived? Does a technology choice survive
verification against live code?

- `docs/design/ARCHITECTURE.md` — every architectural decision is derived, not
  merely asserted
- DECART integration rule — no dependency is added without a comparison table
  and the strongest argument AGAINST
- Ground truth before design — no plan is written without checking the live
  repository

**Ananke** — the principle of organization/necessity. Is the good outcome
*structurally inevitable*, rather than dependent on the maintainer's memory?

- CI fails the build if `courier_score` appears (not "documentation asks not to
  add it" — structurally impossible)
- Type-level money guard — `Money` does not implement `FieldValue`, so
  `interpolate(money)` is a compile error
- `bench_track.py` — criterion + baseline.json, exits with error on regression
  >10%
- All telemetry is local — nothing leaves the browser without permission

**Anu and Ananke together** are what make dowiz a system you can trust not
because "the author promises," but because it structurally cannot work otherwise.

> The author's ethics are based on the principles of Anu and Ananke, which
> uphold logic and harmony, and grant the right to control one's own destiny —
> not submitting to Tiamat (chaos of centralized authority, hidden algorithms,
> and opaque decisions). Knowledge is a shared value — therefore all code is
> open, all documentation is public, and everyone may use, modify, and
> distribute this at their own discretion.

The author gives this ethics and these principles away for free to anyone who
wants it, with the right to use it at their own discretion.

---

## Quick Start

```sh
# Web PWA (zero dependencies)
cd web && python3 -m http.server 8080
# Open http://localhost:8080 in your browser

# Kernel (Rust)
cd kernel && cargo test        # 859+ tests, all green
```

No `npm install`, no API keys, no registration.

---

## Status

**Pre-1.0 / experimental.** Kernel math is deterministic and self-verifying.
The web UI is a working PWA with 3 roles, full order lifecycle, ETA,
notifications, and local telemetry. The product surface is not yet production GA.

Verified 2026-07-22:
- `kernel/` — 859 passed, 0 failed
- `engine/` — 116 passed, 0 failed
- `web/` — 53 passed, 0 failed (utils 16 + sonify 13 + kernel 24)

---

*Why would I use a system that works against me.*
