# R4 — Reed-Solomon / FEC: which problem it actually solves, grounded against existing plans

> Research pass for the `00-SOURCE-DIALOGUE.md` line "ECC (Reed-Solomon) at the tensor-protocol
> level — integrity becomes part of the compute formula, not a separate check … heals a damaged
> fragment in one compute cycle without a retransmission round-trip or a supervisor."
>
> Scope: web-grounded + read-only against the live wire-protocol code
> (`/root/bebop2-verify-redteam/bebop2/`) and this session's verification corpus. Grounded against
> the 185-item mesh-masterwork ledger, the V-series red-team/verification docs, and the
> degrade-closed / no-watchdog doctrine, per the operator's "враховуючи наявні плани та роадмапи."
> Write-only into this worktree. No product code touched.

---

## 0. CONFIRM-FIRST: was FEC ever evaluated before? — NAMED-AS-GAP, NEVER EVALUATED-FOR-FIT

Not virgin ground, but not a prior decision either. Two exact prior mentions exist, both in the
Batch-3 self-healing analysis, and **both name Reed-Solomon only to file it as an unbuilt gap —
neither assesses whether it fits the mesh's actual failure model:**

1. **Ledger item 185** (`BLUEPRINT-BEBOP2-MESH-MASTERWORK-SYNTHESIS-V2.md:572`) — the authoritative
   three-way self-healing frame. Verdict: *"Self-Healing PARTIAL (dynamical+replay real; M7
   topological + **redundancy-ECC gaps → ADOPT M7**)."* The **adopted** resolution of the redundancy
   axis was **M7 topological reconnection (Dijkstra/Union-Find)** — *not* FEC. Reed-Solomon was left
   as a named-but-unadopted gap.
2. **Batch-3 finding** (`12-BATCH3-safety-selfhealing-findings.md:215`) — *"Gap: true redundant /
   error-correcting-code healing (N-of-M redundancy, ECC, Reed-Solomon reconstruction) is **absent**
   … M7 heal … unimplemented."* Again: lumped with M7, filed as absent, never analysed.

Zero hits in the entire `bebop2-verify-redteam` tree (product code + V-series). So: the **token**
"Reed-Solomon" appeared twice as a gap label; the **analysis** — reliability-vs-security fit,
noisy-vs-adversarial channel, courier-phone constraints, crypto layering — **has never been done.**
That analysis is genuinely new ground, and it is the whole value of this pass. **Confirmed.**

A first-order correction falls straight out of this: **FEC and M7 are not the same axis and must not
be conflated as the ledger's "redundancy-ECC" label did.** M7 heals the *graph* (a lost node/link is
routed around); FEC heals the *bytes* (a corrupted/lost packet fragment is reconstructed). Adopting
M7 does not address, and is not addressed by, FEC. They are orthogonal.

---

## 1. The threat model this mesh actually faces is ADVERSARIAL. FEC does nothing for it.

FEC (Reed-Solomon, RaptorQ, any code) is a **reliability** mechanism for an **honest-but-noisy**
channel: random bit-flips from RF interference, packet loss on a lossy link. Its entire theory
assumes the corruption is *random and bounded* — it recovers up to a fixed capacity `t` and no
adversary is *choosing* the errors.

The V3 red-team catalog (`V3-red-team-attack-catalog.md`) is, top to bottom, the **opposite** threat
class. All 11 HIGH findings are deliberate, crafted, adversarial:

| V3 finding | Nature | Does FEC help? |
|---|---|---|
| 4.2 / 4.3 unauthenticated forged revocation (permanent identity / anchor DoS) | attacker crafts a valid-form 32-byte gossip entry | **No** — bytes are already "correct," just malicious |
| 4.1 unbounded delegation-chain-depth CPU DoS | attacker sends validly-signed 10⁶-link chain | **No** |
| 2.2 nonce eviction-then-replay (half-drop prune) | attacker floods to evict a consumed nonce | **No** |
| 5.5 / 4.4 rate-limit primitives dead, no throttle | attacker floods the admit path | **No** |
| 6.4 dormant red-line gate in facade | attacker submits validly-signed money-leg frame | **No** |
| 6.1 caller-controlled `now=0` expiry bypass | attacker/caller supplies `now=0` | **No** |
| 5.1/5.3/5.6 unbounded growth (peers/revocations/half-open conns) | Sybil / gossip flood | **No** |

**Precise statement of the mis-application risk the operator flagged:** an attacker who controls the
channel does not produce *random* corruption for FEC to average out. They either (a) craft a payload
that is *already valid* under the FEC scheme — FEC will faithfully "reconstruct" the attacker's
chosen bytes and hand them up as pristine — or (b) corrupt *more* than the code's capacity `t`,
guaranteeing a decode failure. In **neither** case does FEC add a security property. It was never
designed to, and presenting it as "integrity becomes part of the formula" (dialogue line 24)
**conflates two different meanings of the word integrity**: FEC gives *channel* integrity
(bit-exactness vs noise); the mesh's threat model needs *authenticity* integrity (this came from an
authorised key), which is, and must remain, the **signature's** job (Ed25519 / ML-DSA-65). FEC is
**not** a substitute, an augmentation, or a partial contributor to that. Do not let it be documented
as one.

---

## 2. Where FEC has an honest fit — and it is the V5 conversation, not the V3 one.

The dialogue's own analogies (QR codes, CD/DVD/Blu-ray, satellite, RAID 6, DVB) are **all
noisy-channel, non-adversarial** cases. The mesh's matching real-world problem is **couriers on real
cellular / spotty Wi-Fi** — a legitimate reliability concern, and a **completely different
conversation** from the red-team surface.

That conversation already has a home: **V5 §4 "Network-condition matrix"**
(`V5-cross-platform-device-test-matrix.md:260-304`). V5 records that couriers face *"200-2000ms RTT,
1-10% loss, partition,"* that **no test injects loss/latency/reorder on the real carrier**, and
prescribes `tc netem` recipes (`delay 300ms 100ms … loss 5% reorder 2%`). **If FEC belongs anywhere,
it belongs in that resilience discussion, gated behind those netem measurements — not in the security
model.** This is the honest re-home of the dialogue's idea.

**But even inside V5, there is a decisive catch that shrinks the fit to near-zero *today*:** the two
live carriers already do ARQ + integrity for us.

- `wss_transport.rs` = WebSocket over **TLS over TCP**. TCP retransmits lost segments; TLS records
  carry a MAC. The application's `framing::decode` (`framing.rs:41-65`) *never sees* a bit-flipped or
  partially-lost frame — the stream either delivers complete correct bytes or the connection drops.
- `iroh_transport.rs` = **QUIC**. QUIC does per-packet loss detection + retransmission, and its
  packet protection is AEAD.

So the "packet loss from RF interference" that FEC targets is **already handled below the app, by the
transport's retransmission** — and adaptively (ARQ backs off to the *actual* loss rate), whereas
fixed-rate FEC pays a constant redundancy tax whether the channel is clean or not. On a TCP/QUIC
carrier, per-packet Reed-Solomon is **redundant with machinery that already exists and is strictly
more efficient.** At the application layer, "loss" is not bit-corruption at all — it is *whole-event
gaps*, and those are already handled by **pull-based anti-entropy** (`core/src/anti_entropy.rs`),
which is ARQ (resend the missing suffix), not FEC. And a **fork** is a consistency problem, not a
noise problem — FEC is irrelevant to it.

**FEC earns its place only on a carrier with NO built-in ARQ:** a future raw-UDP/datagram, true
broadcast/multicast to many couriers at once, or a physical mesh (BLE-mesh, Wi-Fi Aware, LoRa) where
a retransmission round-trip is genuinely expensive or impossible (one-to-many, no back-channel).
None of those carriers exists in the tree today.

**Verdict: DEFER-WITH-TRIGGER.** Trigger = "the mesh adds a datagram/broadcast/BLE/LoRa carrier
without transport-level ARQ." Until that trigger fires, FEC is machinery that buys reliability the
transport already provides, while **adding** adversarial attack surface (see §4). Net-negative today.

---

## 3. Concrete technical fit: erasure — not bit-error — coding, and the crate options.

### 3a. The dialogue's "heal a bit-flipped fragment" model is the wrong model for a packet mesh.

Two distinct FEC modes matter, and the dialogue silently assumes the harder, wrong one:

- **Error correction** (unknown-position bit-flips): costs **2 parity symbols per corrupted symbol**
  (positions unknown). This is the CD/DVD model.
- **Erasure recovery** (known-missing symbols): costs **1 parity symbol per lost symbol** (positions
  known). This is the RAID-6 / network-packet model.

On a packet network you **know which packets did not arrive** (sequence numbers), and each packet
already carries its own integrity check (TLS MAC / QUIC / or an app CRC) — so a *corrupted* packet is
simply **discarded and treated as an erasure**. The right model is therefore **erasure coding across
packets**, at half the parity cost of error coding, and the dialogue's "one compute cycle to heal a
bit-flip" framing should be dropped. (Also: RS/RaptorQ decode is `O(n log n)` matrix/FFT work, not
"one cycle" — that phrase is aspirational, not literal.)

### 3b. Real, maintained Rust crates (web-verified, July 2026).

| Crate | Kind | State | Notes |
|---|---|---|---|
| **`reed-solomon-simd`** (AndersTrier) | Fixed-rate RS **erasure**, GF(2¹⁶), O(n log n) Leopard/FFT | **Actively maintained, fastest** | Runtime SIMD (AVX2/SSSE3/Neon) + scalar fallback; 1–32768 shards; ~<10ms one-time table init. Clean-room (patent-free). |
| **`reed-solomon-novelpoly`** | RS **erasure**, novel-poly FFT | Maintained; **used by Polkadot/Substrate for availability erasure** | The closest thing to a production-hardened, web3-vetted option. |
| `reed-solomon-erasure` (rust-rse) | Classic RS erasure | **"Looking for new owners/maintainers" (issue #88)** | Widely used (Solana forked it) but a **maintenance-risk** signal; prefer `-simd`. |
| **`raptorq`** (cberner, redb author) | **RaptorQ fountain / rateless**, RFC 6330, GF(256) | **Actively maintained, high-perf** | Rateless; reconstruct after K+h symbols w.p. `1 − 1/256^(h+1)`; the standardised choice for **broadcast/multicast** (3GPP MBMS, ATSC 3.0). |

### 3c. Recommendation: Reed-Solomon vs fountain codes.

- **For a fixed, small redundancy over a unicast datagram** → **classical RS erasure via
  `reed-solomon-simd`.** Patent-free, tiny, deterministic overhead, fast. Best when the loss rate is
  roughly known and stationary (provision e.g. RS(k+m, k) for the measured netem loss). Weakness:
  fixed rate — worse-than-provisioned loss → decode fail; better → wasted bandwidth/battery.
- **For true broadcast/multicast to many couriers with heterogeneous, unknown loss** → **RaptorQ
  (`raptorq`)** is the technically-superior, rateless, near-optimal fit (each receiver just needs
  "enough" symbols, whichever ones). **BUT flag a DECART patent item:** RaptorQ (Qualcomm / Digital
  Fountain) has a heavy patent history; some core patents are only now expiring (~2025-2027). For a
  project whose stated end-goal is **AGPLv3 open-source (ADR-020)**, patent-clean classical RS is the
  safer default and RaptorQ needs an explicit licensing check before adoption.
- **Courier-phone battery/bandwidth tuning** is a *redundancy-ratio* knob: parity `m/k` trades
  bandwidth+CPU+battery against correction capacity. That knob must be set from the **V5 netem
  measurements**, not guessed — which is exactly why this is DEFER-until-V5-measures, not adopt-now.

---

## 4. Correct crypto/FEC layering — get this right, it is a security property.

**FEC-decode MUST sit BELOW crypto-verify (on raw transport bytes, before parse/verify).** The order
is forced by dependency, not preference: you **cannot verify a signature over corrupted or
incomplete bytes** — you must reconstruct the original bytes first, *then* verify. The only correct
pipeline is:

```
carrier bytes → [FEC reconstruct] → framing::decode → SignedFrame::verify (Ed25519/ML-DSA) → apply
```

The inverse layering ("crypto-verify below FEC," i.e. FEC applied to already-verified plaintext) is
**nonsensical**: once a frame is verified you already hold the correct bytes, so there is nothing left
to correct.

**Why FEC-below-crypto is safe against the §1 adversarial model — and the exact condition on which
that safety depends:** the signature remains the **sole** authenticity gate and runs *after*
reconstruction, so an attacker crafting malicious parity/repair symbols can at most (a) produce
garbage that then **fails signature verification**, or (b) exceed correction capacity → **decode
failure → drop**. Neither path forges a valid signature. **FEC authenticates nothing, and must never
be documented as if it does.**

**The real, honest cost of this layering** (the reason it is net-negative today): the FEC decoder
becomes a **new hostile-input parsing surface positioned *below* the signature** — reachable by any
on-path attacker *before* a single signature check has run. It must therefore obey the **exact same
discipline the V3 catalog already demands of the frame decoder**: bounded allocation (cf. V3 §1.1
pre-alloc cap), **no panic on adversarial input** (cf. V3 §1.2/§3.3), fail-closed on any anomaly, and
be fuzzed against crafted codewords. Adding FEC means adding one more adversarial parser to harden —
**the opposite of a free lunch.** This is the decisive argument for DEFER: on the current
ARQ-carriers it *adds* attack surface to buy reliability we already have.

---

## 5. Doctrine tensions surfaced for operator adjudication (Descartes-square, not silently resolved)

1. **Gradient State / determinism (the dialogue's final unanswered question) — FEC is on the SAFE
   side of it, but a sibling mechanism is not.** The operator asked whether running on "damaged
   (interpolated) data" breaks determinism. **FEC erasure decode is fully deterministic and *exact***
   — same received symbols → bit-identical reconstruction, or a clean decode-failure. It does **not**
   create a determinism problem, and it is **degrade-closed-compatible** (decode-fail → typed drop,
   never a fabricated `Ok`). The determinism problem the operator worried about belongs to the
   *other* leg of the same dialogue — **Temporal Interpolation** (extrapolate the last-valid tensor),
   which is lossy approximation and genuinely non-deterministic. **Recommendation to surface:** keep
   these two firmly separate — FEC = exact recovery (safe, degrade-closed); interpolation = the real
   determinism red-flag (a separate proposal, not covered here).

2. **No-watchdog / no-proxy — FEC is compatible.** Inline transport math, no supervisor. Consistent
   with the "self-healing as emergent math, not a supervisor" doctrine (item 185, §6b). No tension.

3. **vs. the already-adopted M7 (item 185).** No conflict, but the ledger's "redundancy-ECC" label
   must be **un-conflated**: M7 (adopted) heals topology; FEC (this pass, deferred) heals bytes. If
   FEC is ever adopted it is a *new* line item on a *different* axis, not a re-opening of M7.

**Net verdict:** DEFER-WITH-TRIGGER (raw/broadcast carrier without ARQ). If/when triggered:
erasure-mode `reed-solomon-simd` as the patent-clean default, `raptorq` only for genuine multicast
(with a patent/licensing DECART), FEC-below-crypto layering, decoder hardened to V3 parser
discipline, redundancy ratio tuned from V5 netem measurements. **Not a security control — a V5
reliability control.**

---

### Sources (web-grounded, July 2026)
- reed-solomon-simd — https://github.com/AndersTrier/reed-solomon-simd · https://docs.rs/reed-solomon-simd
- reed-solomon-erasure (maintainer-wanted, #88) — https://github.com/rust-rse/reed-solomon-erasure
- reed-solomon-novelpoly (Polkadot availability) — https://lib.rs/crates/reed-solomon-16
- raptorq (RFC 6330, cberner) — https://github.com/cberner/raptorq · https://crates.io/crates/raptorq
- RFC 6330 RaptorQ — https://datatracker.ietf.org/doc/html/rfc6330
- Local grounding: `V3-red-team-attack-catalog.md`, `V5-cross-platform-device-test-matrix.md:260-304`,
  `BLUEPRINT-BEBOP2-MESH-MASTERWORK-SYNTHESIS-V2.md:572` (item 185),
  `12-BATCH3-safety-selfhealing-findings.md:215`, `proto-wire/src/framing.rs`,
  `proto-wire/src/{wss_transport,iroh_transport}.rs`, `core/src/anti_entropy.rs`
