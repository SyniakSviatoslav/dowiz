# QKD & Quantum Security — Deep-Dive Against dowiz's Real Topology

> Research-only. Zero code, zero branch, zero deploy. Author: Opus 4.8. Date: 2026-07-18.
> Follow-up to today's settled points (taken as GIVEN, not re-derived here):
> (a) ANU QRNG in dowiz is an entropy *seed*, not an auth replacement;
> (b) "1-bit" auth is a coin flip regardless of trust topology;
> (c) real QKD gives information-theoretic security but needs physical quantum hardware
> between two *specific fixed* endpoints — a precondition dowiz's mesh (courier phones on
> cellular, semi-trusted relay) cannot meet.
> This doc builds on those: it gets the current QKD facts right, checks whether dowiz has
> *any* fixed physical link where QKD's precondition could ever hold, and states honestly
> whether QKD is worth exploring or purely theoretical for the real deployment.

---

## 0. Provenance & method (read this first)

- **WebSearch budget was exhausted for this session** (200/200 used before this task). Live
  keyword search was unavailable. I substituted **WebFetch** (a different channel, not
  subject to that budget) against primary sources, plus **direct repo investigation**
  (Read/Grep on the actual working tree). ~14 successful tool calls.
- Numbers marked **[web-verified]** were fetched live this session (Wikipedia QKD / Quantum
  network / Post-quantum cryptography pages). Numbers marked **[cutoff-knowledge]** are from
  the Jan-2026 model cutoff and were **not** re-confirmed live this session (vendor product
  pages 404'd / were not fetchable); treat those as directionally reliable but not
  freshly-cited. Repo facts are cited as `path:line` against the live tree.
- Style: logical + structural rigor throughout (Anu/Ananke discipline). No metaphor framing,
  no market/country bias. Claims are falsifiable and sourced.

---

## 1. QKD physics, precisely — the "1 bit per photon" point vs the final key

The operator's phrasing ("quantum security truly needs a physical device or physical key") is
correct. The finer point worth nailing:

### 1.1 BB84 encodes 1 bit per photon — but the *final key* is a normal-length symmetric key
- **[web-verified]** BB84 encodes **1 bit per photon** via photon polarization across two
  complementary bases (rectilinear + diagonal). Each photon carries one classical bit.
- After transmission, Alice and Bob publicly compare *basis choices* and **discard the ~50%
  where bases didn't match** → **sifting** keeps roughly half the raw bits. **[web-verified]**
- The sifted string is then run through two more classical stages:
  1. **Information reconciliation** (error correction over a public channel — Cascade, or
     modern LDPC / polar / turbo codes) to remove the ~QBER mismatch.
  2. **Privacy amplification** (universal hash / randomness extractor) to shrink Eve's
     partial information to negligible.
- **The crucial correction to any "1-bit" intuition:** the *final* key is **not** one bit.
  It is a continuous secret-key *stream*. You run the photon source long enough to accumulate
  whatever length you need — a 256-bit AES key, or bytes for a one-time pad. The per-sifted-bit
  yield after PA follows the Shor–Preskill bound for BB84, ≈ `1 − 2·h₂(QBER)` (h₂ = binary
  entropy); a link with a few-percent QBER still nets a large positive fraction. So: **1 bit
  *per photon* at the physical layer; a full-size, conventional-length symmetric key at the
  output.** The "1-bit" figure is a per-carrier encoding density, never the key size.

### 1.2 Real key rates and distances (why topology dominates)
**[web-verified]** from the QKD literature timeline:

| Distance (fiber) | Secret key rate | Source / year |
|---|---|---|
| 20 km | **~1 Mbit/s** | Cambridge/Toshiba, 2008 |
| 100 km | **~10 kbit/s** | same experiment |
| 307 km | **12.7 kbit/s** | Univ. Geneva, 2015 (fiber record era) |
| 404 km | reached, but **"too slow to be practical"** | Corning/China, 2016 |

- Rate **decays exponentially with distance** (fiber loss on a channel where you *cannot*
  amplify — amplification would destroy the quantum states). **[web-verified]**
- **Point-to-point limit ≈ 100–400 km.** Beyond that you need **trusted-node relays** (real,
  deployed) or **quantum repeaters** (not yet a deployable product). **[web-verified]**

### 1.3 The trusted-node caveat that quietly kills the "information-theoretic" promise at scale
This is the single most important QKD nuance for any real network, and it is often glossed:

- At each **trusted relay node**, the key is **decrypted and re-encrypted** — the node
  operator *sees the key in the clear*. Information-theoretic security holds **only for a
  single unbroken point-to-point link**. A multi-hop QKD network reduces to **trusting every
  relay operator** along the path.
- **[web-verified]** Deployed multi-hop networks confirm this is the operating reality, not a
  corner case: Beijing–Shanghai backbone (2017) = **2000 km via 32 trusted nodes**, extended
  to ~4600 km by 2021 with satellite links (Micius). SECOQC Vienna (2008) = 200 km, 6 nodes,
  "trusted repeater architecture." Tokyo QKD (2010), SwissQuantum Geneva (2009–2011) —
  metro-scale, single/few hops.

**Consequence for any distributed system:** QKD's headline property (unconditional security)
is available *only* between two endpoints joined by one dedicated quantum channel. The moment
you relay, you are back to trusting intermediaries — exactly the trust problem PQC/classical
crypto already solves in software without the fiber.

---

## 2. Commercial QKD in 2024–2026 — availability, cost, hardware precondition

**[web-verified]** Commercial vendors exist and have for two decades: **ID Quantique**
(Geneva; brought the first commercial QKD system to market in **2004**), **Toshiba**, **MagiQ
Technologies** (NY), **QNu Labs** (India), **QuintessenceLabs** (Australia), and others.

**[web-verified]** Metro / backbone deployments that are (or were) real and operating over
fiber: SECOQC Vienna, SwissQuantum Geneva, Tokyo QKD Network, Beijing–Shanghai trunk. These
are the concrete "QKD-as-a-service over fiber in specific cities" data points — they exist,
they are **metro-scale and trusted-node-based**, and they serve high-value fixed institutions
(government, banks, research labs), never mobile consumer endpoints.

**[cutoff-knowledge]** (not re-fetched live — vendor product pages were not fetchable this
session; flag as directional):
- Product families: ID Quantique **Cerberis XG / Clavis** series; Toshiba QKD appliances
  (the same platform behind the BT/Toshiba London Quantum-Secured Metro Network, ~2022).
  EuroQCI is the EU's multi-year effort to build a QKD/quantum-comms backbone across member
  states.
- **Hardware precondition (the load-bearing fact):** a QKD link is a **1U-ish appliance at
  *each* of two *fixed* endpoints**, joined by a **dedicated dark fiber or a dedicated DWDM
  channel** (the quantum channel cannot share amplified/lit fiber with normal traffic in the
  naive case; co-existence over lit fiber is an active research/engineering constraint, not a
  free lunch). It is inherently **point-to-point between two immobile boxes**.
- **Cost order-of-magnitude:** tens of thousands to ~$100k+ USD **per node-pair**, plus the
  fiber lease and operations. Not a per-user or per-device cost — a per-*fixed-link* capex.

**Structural takeaway:** every commercial QKD offering requires (1) two fixed endpoints, (2) a
dedicated physical channel between them, (3) five-to-six-figure capex per link, and (4) a
distance under a few hundred km or a chain of trusted relays. This is infrastructure for a
bank connecting two owned buildings, or a state connecting two data centers — **not** for an
internet-facing app whose endpoints are phones.

---

## 3. dowiz's actual topology — is there ANY fixed physical link QKD could ever attach to?

I investigated the live tree and ops docs. The honest answer: **no.**

### 3.1 What the deployment actually is (verified)
- **No `fly.toml` in-repo** (Fly retired 2026-07-18 per CLAUDE.md; confirmed absent on disk).
  Live target is **Hetzner + Cloudflare**.
- **Single Hetzner box.** `docs/ops/P8-SINGLE-PANE-SPEC.md` describes exactly one server:
  live Postgres on the box's `sda`, plus a **same-account** 50 GB Hetzner volume
  `/mnt/volume-fsn1-1` (Falkenstein `fsn1` region) used as backup *staging* (P8 §, lines
  ~32, 82–91). Cloudflare fronts it.
- **The backup "offsite" leg is MISSING and is the top gap.** P8's 3-2-1-1-0 analysis
  (`docs/ops/P8-SINGLE-PANE-SPEC.md:76–91,130`): copies 1 and 2 are *both on the same Hetzner
  account/region*; the "1 offsite, immutable" leg is **🔴 MISSING**. The proposed offsite
  target is **rsync.net with Object-Lock** — a **third-party cloud object store reached over
  the public internet**, not a dowiz-owned second data center.
- **No second owned server, no multi-region owned infra, no dark fiber.** The only
  "multi-region" language in the tree is future *hub-provisioning capacity planning* —
  `BLUEPRINT-P67-hub-provisioning-claim.md:568` ("multi-region = multiple pools") — which
  means *more rented cloud pools*, still commodity cloud, still no owned point-to-point fiber.

### 3.2 Map QKD's precondition onto every candidate link
| Candidate link in dowiz's real graph | Two *fixed* endpoints? | Dedicated physical channel dowiz controls at both ends? | QKD-eligible? |
|---|---|---|---|
| Courier phone ↔ relay/server | ❌ phone is mobile, on cellular | ❌ | **No** (settled) |
| Customer device ↔ server | ❌ mobile/browser | ❌ | **No** |
| Cloudflare edge ↔ Hetzner origin | origin fixed, edge is a global anycast fleet | ❌ (public internet, CF-owned edge) | **No** |
| Hetzner box `sda` ↔ same-account volume `fsn1-1` | same physical/region | it's a *local block volume*, not a two-endpoint link | **Moot** (one location) |
| Hetzner origin ↔ rsync.net offsite backup | both fixed | ❌ public internet; dowiz owns neither the path nor the rsync.net end | **No** |
| Primary DC ↔ owned backup DC | — | **does not exist** — there is no owned second DC | **N/A** |

Every row is either mobile, over the public internet, single-location, or nonexistent. **There
is no pair of dowiz-controlled fixed endpoints joined by a channel dowiz owns at both ends.**
QKD's precondition is unmet everywhere in the real architecture.

### 3.3 The repo already reached this conclusion (consensus, not a new claim)
This is not a fresh contrarian take — the codebase's own planning docs say the same thing:
- `docs/design/integration-ports/INTEGRATION-PORTS-PLAN.md:314–316`: physical-layer quantum
  crypto (Y-00 / QKD) is **"Не досяжно в софті (marketing-пастка)"** — *unreachable in
  software (a marketing trap)* — because it needs optical hardware / dedicated fiber; "a
  web-app / phone **cannot** do physical-layer quantum-noise encryption."
- `docs/design/integration-ports/BLUEPRINTS-INTEGRATION-PORTS.md:262`: same — Y-00/QKD need
  optical hardware, software can't.
- Standing stance across the tree: **QRNG seeds/mixes, never replaces** OS entropy
  (`MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md:912`). Confirmed in code at
  `kernel/src/pq/entropy.rs:1–35` — the mix is `SHAKE256(quantum ‖ os)`, feature-gated behind
  `qrng`, explicitly kept *out of the crypto hot path*.

---

## 4. The actually-relevant answer: PQC — and dowiz already has it, real and in-tree

For dowiz's real threat model (mobile courier/customer devices, internet-facing mesh, a
semi-trusted relay, standard cloud hosting), the *practically-relevant* "quantum-resistant"
answer is **post-quantum cryptography**, because PQC is **pure software that runs over any
existing internet link with no special hardware** — the exact opposite of QKD's constraint.
**[web-verified]** NIST finalized the PQC standards in **August 2024**: **FIPS 203 ML-KEM**
(Kyber), **FIPS 204 ML-DSA** (Dilithium), **FIPS 205 SLH-DSA** (SPHINCS+). **[web-verified]**
Industry ships PQC in **hybrid** classical+PQ form as the default (Google, Apple **PQ3**,
Signal **PQXDH**) precisely so security holds if *either* leg survives.

**dowiz is already on this path, and it is real code, not a plan:**

| PQC surface | Location | State (verified) |
|---|---|---|
| **Hybrid KEM** X25519 + ML-KEM-768, BOTH mandatory | `kernel/src/pq/hybrid.rs:1–13` | RED gate **forbids** classical-only downgrade; combine KDF = `SHAKE256(mlkem_ss ‖ x_ss)`. Matches industry hybrid pattern. |
| **ML-KEM-768** (FIPS 203) from scratch | `kernel/src/pq/kem.rs:1–28` | Full Cooley–Tukey NTT over Z_q[x]/(x²⁵⁶+1), q=3329; KAT-gated for encaps==decaps + tamper. |
| **ML-DSA-65** (FIPS 204, Dilithium mode 3) from scratch | `kernel/src/pq/dsa.rs:1–18` | Byte-exact port of pq-crystals reference; verified against **official NIST ACVP** keyGen/sigGen/sigVer vectors (`kernel/src/pq/kat/acvp/*.json`). |
| **Hybrid signer** `Ed25519 ⊕ ML-DSA-65` | `kernel/src/capability_cert.rs:3,16–23,77`; `kernel/src/lib.rs:155–156` | Algorithm-agile cert chain via `SignatureVerifier` seam; production injects real bebop2 Ed25519 + ML-DSA-65. (Memory: P06 key_V HybridSigner closed 2026-07-18, commit `58987d79d`, bebop2-kv real sign/verify.) |
| **Entropy mix** (QRNG seeds, never replaces) | `kernel/src/pq/entropy.rs:1–55` | RNG-free core; `SHAKE256(quantum ‖ os)`; ANU QRNG behind `qrng` feature, off the hot path. |

Honest comparison for dowiz's real threat model:

| Property | **PQC (in-tree, in-progress)** | **QKD** |
|---|---|---|
| Works over phones / cellular / browser | ✅ pure software | ❌ needs fixed optical endpoints |
| Works over the existing internet / Cloudflare | ✅ | ❌ needs dedicated dark fiber / DWDM |
| Endpoint mobility | ✅ any device | ❌ two immobile boxes only |
| Cost per endpoint | ✅ ~zero (software) | ❌ $10⁴–$10⁵+ per fixed link |
| Distance | ✅ unbounded (internet) | ❌ ~100–400 km or trusted relays |
| Trust at scale | ✅ end-to-end, no relay trust | ❌ multi-hop = trust every relay node |
| Auth / signatures | ✅ ML-DSA/Ed25519 (QKD needs this anyway) | ❌ QKD does key-agreement only; still needs classical/PQC auth to prevent MITM |
| Standards maturity | ✅ FIPS 203/204/205 (Aug 2024) | mature hardware, but niche applicability |

Note the last two rows are decisive: **QKD does not even provide authentication** — a QKD link
must be authenticated by a *classical or post-quantum signature* to stop a man-in-the-middle,
so you need PQC-style crypto *regardless*. QKD would be an *addition* to, never a replacement
for, the ML-DSA/Ed25519 work already in-tree. **[web-verified]** the NSA/GCHQ even argue
*against* hybrid's added complexity for national-security use; that debate is about PQC
transition mechanics — QKD is not on that table for general internet/mobile endpoints at all.

---

## 5. Verdict — is there ANY near-term-relevant QKD target in dowiz's real architecture?

**No. Purely theoretical, no real target given the current (and planned) infrastructure.**
Same honesty as the prior passes today — I am not manufacturing a use case.

Reasoning, structurally:
1. QKD's precondition is a **dedicated physical quantum channel between two dowiz-controlled,
   immobile endpoints**. §3 shows dowiz has **zero** such links: single Hetzner box, mobile
   client/courier devices, public-internet paths, third-party (rsync.net) backup, and only
   *rented-cloud* "multi-region" on the roadmap. There is nothing to attach QKD hardware to.
2. Even the one link that is *conceptually* fixed-to-fixed and high-value — **primary origin →
   offsite backup** — fails on every count: the second endpoint is a **third-party cloud store
   over the public internet** (you cannot install a QKD appliance at rsync.net's end, and there
   is no dark fiber), the distance/relay problem applies, and PQC-in-transit + Object-Lock
   already solves the real threat (backup confidentiality/immutability) for ~$0 hardware.
3. QKD provides **no authentication** and, beyond one hop, **reduces to trusting relay
   operators** (§1.3). Both properties dowiz needs are already delivered *better* in software by
   the in-tree ML-KEM/ML-DSA hybrid stack (§4).
4. The repo's own prior analysis already ruled this out (§3.3) and chose the QRNG-seed +
   PQC-in-software path. This deep-dive *confirms* that call with current 2024–2026 facts; it
   does not overturn it.

**The one honest "long-shot future" caveat, stated so it can't be mistaken for a
recommendation:** QKD could *only* ever become relevant to dowiz if the business one day owned
**two physical data centers within a metro (<~100 km) joined by dowiz-leased dark fiber**, and
had a regulatory/threat reason that PQC's computational-security assumption was deemed
insufficient for the *inter-DC* link. That is several strategic pivots away (owning DCs,
owning fiber, a threat model where FIPS-204 lattice hardness is distrusted), serves *only* that
single owned-metro link, and still needs PQC/classical auth on top. **Until dowiz owns fixed
physical endpoints and the fiber between them, QKD has no attachment point. Investment: none
warranted. Direction: keep finishing the PQC hybrid stack that is already real in-tree.**

---

## 6. One-line summary for the operator
Your instinct is exactly right: quantum security needs a physical device/link. dowiz has no
two owned fixed endpoints with fiber between them (single Hetzner box, mobile clients,
internet/third-party backup), so **QKD has literally nothing to attach to — purely
theoretical, zero near-term target.** The real, already-in-progress answer for the actual
mobile/internet threat model is **post-quantum crypto (ML-KEM-768 + ML-DSA-65 hybrid, both
already coded and KAT-verified in `kernel/src/pq/`)** — software that needs no special
hardware and works over the phones and internet dowiz actually runs on. Keep pushing PQC;
QKD stays a whiteboard curiosity unless dowiz someday owns data centers *and* the dark fiber
between them.

---

### Sources
Web (fetched live this session via WebFetch):
- [Quantum key distribution — Wikipedia](https://en.wikipedia.org/wiki/Quantum_key_distribution)
- [Quantum network — Wikipedia](https://en.wikipedia.org/wiki/Quantum_network)
- [Post-quantum cryptography — Wikipedia](https://en.wikipedia.org/wiki/Post-quantum_cryptography)
- [ID Quantique — Wikipedia](https://en.wikipedia.org/wiki/ID_Quantique)

Repo (live tree, this session): `docs/ops/P8-SINGLE-PANE-SPEC.md`;
`docs/design/integration-ports/INTEGRATION-PORTS-PLAN.md`;
`docs/design/integration-ports/BLUEPRINTS-INTEGRATION-PORTS.md`;
`docs/design/MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md`;
`docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P67-hub-provisioning-claim.md`;
`kernel/src/pq/hybrid.rs`, `kem.rs`, `dsa.rs`, `entropy.rs`, `capability_cert.rs`, `lib.rs`.

Provenance note: NSA/GCHQ hybrid-complexity stance is web-verified; ID Quantique/Toshiba
product-line names and per-link cost order-of-magnitude are Jan-2026 cutoff-knowledge (vendor
pages were not fetchable this session and WebSearch budget was exhausted), flagged as such
in §2.
