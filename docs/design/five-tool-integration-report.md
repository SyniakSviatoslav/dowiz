# Integration-Ready Research: 5 External Tools → bebop coding-agent

> **Task:** deep reverse-engineer Video-use, Sentinel Pro 3.1, Torlink, Shattermind, Cochlea; map onto bebop's EXISTING primitives (vault/pod/guard/reputation/matcher/stabilizer/wavefield/zenoh/sandbox/rust-core); deliver a falsifiable RED+GREEN Verified-by-Math spec per tool + a Dossier→primitive mapping table.
> **Repo:** `/root/bebop-repo`, branch `feat/wire-native-core`. 54 Rust modules + `rust-core`.
> **Discipline:** sovereign-core red line = offline, deterministic, 0 deps. External tools are reverse-engineered into their CORE PATTERN and re-implemented natively (falsifiable), OR deferred (external service/weights/crypto/UI), OR authorized-offensive (own-project-only, gated by `TargetScope`), OR refused (harm-to-others). This is the *established* policy in `crates/bebop/src/research_patterns.rs` + `docs/design/research-12tool-ev-2026-07-10.md`. We follow it.
> **Read-only:** this report does NOT edit the repo. Suggested insertion points are given; no code is written to `bebop-repo`.

---

## Pre-flight: source confidence & ethics triage

| Tool | Source reached? | Verdict | Bucket (per repo policy) |
|------|----------------|---------|--------------------------|
| Video-use | ✅ GitHub `browser-use/video-use` (16.4k★, MIT) | Verified | INTEGRATE (core pattern; defer network glue) |
| Sentinel Pro 3.1 | ⚠️ Instagram reels only (no code, no repo, no license) | **Unverified-offensive** | **REFUSE / AUTHORIZED-OFFENSIVE only** (see flag) |
| Torlink | ✅ GitHub `baairon/torlink` (3.4k★, MIT) | Verified | INTEGRATE (as gated Transport) |
| Shattermind | ❌ No tool by this name — only fictional chars / music labels | **Unverified / unknown** | DEFER (do not engineer until a real source exists) |
| Cochlea | ✅ GitHub `mrkrd/cochlea`, `cochlea3`, `cochlea.xyz` | Verified (bio-signal lib) | INTEGRATE (as signal front-end) |

Ethics hard-line applied: **NO military/warfare, NO surveillance-for-harm, NO invisible-broker/snake-surprise.** Sentinel Pro 3.1 trips the surveillance/offensive line for third-party targets and is gated accordingly. Shattermind cannot be responsibly engineered without a verifiable source (no fabrication).

---

## 1) Video-use  — *edit videos with coding agents*

**Identity / purpose (VERIFIED — github.com/browser-use/video-use, MIT, 16.4k★, active Jun 2026).**
A shell-capable coding-agent skill that turns raw footage into `final.mp4` via ffmpeg. Pipeline: `Transcribe → Pack → LLM Reasons → EDL (edit-decision-list) → Render → Self-Eval` (re-render up to 3× on self-eval failure). Design principles: (1) *text + on-demand visuals* — never frame-dump; the transcript is the reasoning surface; (2) *audio-primary* — cuts come from speech boundaries + silence gaps; (3) *ask → confirm → execute → self-eval → persist*; (4) *12 hard production rules* (30 ms fades, burnt subtitles, no visual jumps/pops). Self-eval runs `timeline_view` on the *rendered* output at every cut boundary.

**Core mechanism (reverse-engineered).**
1. Transcribe audio → a structured transcript (text + a few PNGs, ~12 KB, not 45 M tokens of frames).
2. Pack transcript into a compact edit spec.
3. LLM proposes an **EDL** (cut list: `{in_ts, out_ts, transition}`).
4. ffmpeg renders the EDL.
5. **Deterministic self-eval gate**: inspect rendered cut boundaries for pops/jumps/hidden subs; if any fail → re-plan.

The *deterministic, testable* core is steps 2/3/5 — the EDL generation and the self-eval gate. Steps 1/4 are ffmpeg/LLM glue (network/weights → out of sovereign-core).

**Threat / attack surface + how bebop constrains it.**
- *Prompt-injection via transcript*: a malicious subtitle/transcript could steer the cut list. → Constrained by `garak`/`injection_probe` pattern already in `research_patterns.rs` + `redteam.rs`.
- *Arbitrary shell exec* (ffmpeg flags): → `sandbox.rs` must wrap the render; egress tokens (network pull of remote assets) REFUSED unless `network` opted in.
- *Non-reproducible output*: → the EDL→self-eval rule set is a **pure function**; emit the EDL + eval verdict as a ContentHash (`fnv1a`/`fingerprint`) so two nodes agree the edit is "production-correct".

**Integration point.**
- EXTEND `crates/bebop/src/research_patterns.rs` (add `edit_decision_list` + `eval_cuts` pure functions) and map the self-eval gate onto `guard.rs::io_guard` (the Output bouncer).
- Primitive map: **Self-eval gate ⇄ `guard::io_guard`** (fail-closed: an edit failing the 12-rule gate is `Refuse`d, never emitted). **EDL reproducibility ⇄ `matcher::fingerprint`** (content-hash the cut list).
- NEW crypto? **No** — vault exists. NEW math? **No** — silence-gap cut detection is a threshold on a signal envelope, expressible with `rust-core` `sinc`/envelope helpers.

**RED + GREEN Verified-by-Math spec.**
```
fn eval_cuts(edl: &[Cut], rules: &ProductionRules) -> GuardVerdict
  where Cut = {in_ts, out_ts, fade_ms}; rules require fade_ms>=30,
  in_ts<out_ts, no overlap, no jump> threshold.

GREEN: edl=[Cut(0.0,5.0,30),Cut(5.0,10.0,30)] ⇒ Permit.
RED-1: edl=[Cut(0.0,5.0,0)]  (fade<30ms ⇒ pop) ⇒ Refuse.
RED-2: edl=[Cut(0.0,5.0,30),Cut(4.0,9.0,30)] (overlap) ⇒ Refuse.
RED-3: edl=[Cut(5.0,2.0,30)] (in>out ⇒ impossible cut) ⇒ Refuse.
RED-4: fingerprint(edl_a) == fingerprint(edl_b) for two independently
       derived identical EDLs (determinism of the pure pack step).
```
**Ethics flag:** ✅ clean (video editing). Only the LLM/ffmpeg/ElevenLabs *glue* is deferred (network/weights) — the deterministic EDL+gate core is integrable offline.

---

## 2) Sentinel Pro 3.1  — *autonomous AI OSINT + offensive-security platform*

**Identity / purpose (⚠️ UNVERIFIED — only Instagram reels; no repo, license, or code reachable).**
Per the only reachable descriptions, it is advertised as a "fully autonomous, AI-driven OSINT and **offensive security** platform… built entirely from scratch," a "6.5 GB fully localized ecosystem powered by a custom AI Brain, 19 autonomous agents," with "Multi-layer Username Tracking (Surface + **Dark Web**)," Q-Learning adaptation, and "SentinelProxy & Secure Credential" modules. No source, no verifiable architecture, no terms-of-use found.

**Core mechanism (reverse-engineered from claims — treat as BEST-KNOWLEDGE, unverified).**
A multi-agent offensive-recon stack: OSINT harvesting (usernames across surface + dark web), autonomous target profiling, Q-learning-driven attack-path selection, credential/proxy handling. The *pattern* (recon orchestration + scope gate) already exists natively in bebop as `TargetScope`/`Ipv4Cidr` + `crawl_frontier` + `wordlist_paths` + `follow_redirects` (all in `research_patterns.rs`, gated own-project-only).

**Threat / attack surface + how bebop constrains it.**
- *Third-party surveillance / harm*: autonomous OSINT on arbitrary persons is exactly the **surveillance-for-harm / invisible-broker** prohibition. → Per repo `AUTHORIZED-OFFENSIVE` bucket, recon is permitted **only against your own project/scope**, enforced by `TargetScope` (out-of-scope target ⇒ deterministic `Refuse`). The gate is load-bearing and RED-proved in tests.
- *Autonomous agents acting without confirmation*: → `guard::KillSwitch` (≥2/3 consensus suspension) + `io_guard` envelope bound any proposed action.
- *Credential/proxy misuse*: → `vault.rs` secrets boundary; `sandbox.rs` egress REFUSE by default.

**Integration point.**
- **Do NOT engineer the offensive platform.** If any *defensive* pattern is worth keeping, it is already covered: `TargetScope` gate (`research_patterns.rs`), `scan_secret` (gitleaks-class), `injection_probe` (garak-class). No new file needed.
- NEW crypto? No. NEW math? No.

**RED + GREEN Verified-by-Math spec (of the gate that would contain it).**
```
fn permit_action(scope: &TargetScope, target: &str) -> GuardVerdict
GREEN: target in scope (own project CIDR/domain) ⇒ Permit.
RED-1: target = third-party personal username ⇒ Refuse (surveillance gate).
RED-2: scope=empty + any target ⇒ Refuse (fail-closed, no implicit allowance).
RED-3: a "dark-web" target outside declared scope ⇒ Refuse.
```
**Ethics flag:** 🚩 **HARD FLAG — do NOT engineer as described.** Autonomous OSINT/offensive capability against third parties violates the no-surveillance-for-harm / no-invisible-broker line. Integrate ONLY the defensive/scope-gate pattern already present, gated by `TargetScope` (own-project-only). The unverifiable, non-reproducible "6.5 GB ecosystem" claim is itself a reason to defer — it cannot be reviewed for hidden centralization (snake-surprise).

---

## 3) Torlink  — *zero-setup terminal torrent finder/downloader*

**Identity / purpose (VERIFIED — github.com/baairon/torlink, MIT, 3.4k★, v1.4.0 Jul 2026).**
A terminal-native P2P tool: concurrent search across a *curated* list of torrent sources, `webtorrent` download engine with true pause/resume, magnet paste, Ink+React TUI. Recent hardening (v1.4.0): constant-time token compare on auth, **loopback-only Host header check** to block DNS-rebinding from a webpage, HTTP(S) tracker fallback when UDP/DHT is blocked, `413` on oversize body. No indexer server, zero config.

**Core mechanism (reverse-engineered).**
1. Curated multi-source search → streamed, source-tagged results (no central indexer).
2. Magnet/info-hash → WebTorrent swarm over TCP (WebRTC/HTTP trackers as UDP fallback).
3. Content-addressed pieces (BitTorrent SHA-1 piece hashes) → deterministic reassembly.
4. Headless HTTP daemon (locked down: loopback Host, constant-time token) for serve/files.

The *transfer* layer is a **content-addressed, peer-to-peer transport** — structurally analogous to bebop's mesh transport need.

**Threat / attack surface + how bebop constrains it.**
- *Egress / P2P to arbitrary peers*: → must run only via `sandbox.rs` with explicit `network=true`; the piece manifest it shares must carry a **POD proof** (`pod.rs`) so only authorized delivery payloads propagate.
- *DNS-rebinding / malicious tracker*: → Torlink already mitigates (loopback Host, constant-time token); bebop adds `guard::io_guard` on *what* is fetched (only items with a valid `fingerprint`/`MatcherResponse`).
- *Content illegality (piracy)*: → NOT bebop's concern to police content, but bebop uses it ONLY as a Transport for **authorized, content-addressed delivery artifacts** (matcher payloads, ledger snapshots), gated by POD + sandbox.

**Integration point.**
- EXTEND `crates/bebop/src/zenoh.rs` (mesh transport) **and/or** implement a `RemoteMatcherClient` Transport backend in `matcher.rs` that delivers `MatcherRequest`/`MatcherResponse` blobs over a Torlink-style content-addressed swarm.
- Primitive map: **Torlink swarm ⇄ `zenoh` mesh transport / `matcher::Transport`**. Piece hash ⇄ `matcher::fingerprint` (content-address already in the protocol's DNA).
- NEW crypto? **No** — BitTorrent piece hashing is internal; bebop wraps payloads in `vault`/SHA512 + POD. NEW math? No.

**RED + GREEN Verified-by-Math spec.**
```
fn accept_piece(manifest: &Manifest, piece: &[u8], pod: &PodProof) -> GuardVerdict
  where Manifest carries expected SHA-like fingerprint per piece + courier POD.

GREEN: piece fingerprint matches manifest AND pod verifies ⇒ Permit (ingest).
RED-1: piece fingerprint mismatch (bit-flip / wrong content) ⇒ Refuse.
RED-2: no valid POD on the manifest (unknown courier) ⇒ Refuse (sandbox+reputation gate).
RED-3: network requested but sandbox egress NOT opted in ⇒ Refuse (fail-closed).
RED-4: two honest nodes recomputing fingerprint(resp) over the same
       matched blob agree (replaces proprietary transport lock-in).
```
**Ethics flag:** ✅ neutral transport primitive. Flag: only integrate as an **authorized content-distribution transport** for bebop's own delivery/ledger payloads; do not ship it as a general torrent client. Piracy exposure is contained by POD + sandbox-network gate.

---

## 4) Shattermind  — *no verifiable tool by this name*

**Identity / purpose (❌ UNVERIFIED — source NOT reachable).**
Web search returns only: a *Forgotten Realms* fictional illithid dwarf ("Cernd Shattermind"), a music label ("Shattermind Recordings"), and unrelated social posts. **No software project, repo, or spec named "Shattermind" was found.** Per task rules: never fabricate a source.

**Best-knowledge interpretation (labeled HYPOTHESIS, not fact).**
If this refers to a *fragmentation / adversarial-chaos / agent-shattering* resilience concept (a plausible reading of the name), the relevant bebop primitive already exists: `stabilizer.rs` + `wiring.rs` **ensemble-disagreement freeze** and `guard::io_guard` (reject proposals that shatter the safe envelope). The "autodidactic universe" precedence-decay (`reputation.rs`) also models recovery from disruption.

**Integration point (conditional — only if a real source appears).**
- Candidate: EXTEND `wiring.rs` ensemble-disagreement freeze + `stabilizer::potential_well` (drift back to ground state). Primitive map: **fragmentation-resistance ⇄ `stabilizer` ground_state / `wiring` ensemble-disagreement**.
- NEW crypto? No. NEW math? No (potential-well + Lyapunov already in `stabilizer.rs`).

**RED + GREEN Verified-by-Math spec (of the *concept*, already implemented — reuse).**
```
GREEN: ensemble_disagreement < threshold ⇒ adapt normally.
RED: ensemble_disagreement > threshold ⇒ freeze (θ̇=0) regardless of L5 proposal
     (already RED-proved in stabilizer tests).
```
**Ethics flag:** ⚠️ **DEFER — cannot responsibly engineer an unverifiable tool.** No source ⇒ no architecture to reverse-engineer, and fabricating one would violate the task's "never fabricate a source" rule. Recommend: hold until a real repo/paper is supplied; if supplied, re-triage under the bucket policy (likely INTEGRATE the resilience pattern above).

---

## 5) Cochlea  — *inner-ear / bio-inspired signal front-end*

**Identity / purpose (VERIFIED — github.com/mrkrd/cochlea, `cochlea3`; `cochlea.xyz` sparse audio codec; `dfl/cortix` "Organ of Corti" gammatone filterbank).**
A collection of **inner-ear (cochlea) models**: take a sound signal → return spike trains / auditory-nerve representations. Core bio-primitive: the **gammatone filterbank** (ERB-spaced bands) that decomposes a waveform into frequency channels — the biological spectrum analyzer. Related: `cochlea.xyz` "sparse, interpretable audio codec" (event/spike-based encoding).

**Core mechanism (reverse-engineered).**
1. Input waveform `x(t)`.
2. Pass through an ERB-spaced **gammatone filterbank** → per-channel envelopes.
3. (Optional) half-wave rectify + adapt → spike-train / sparse event encoding.
The output is a **multi-channel, time-localized feature representation** — a front-end that turns raw signal into structured, low-dimensional events.

**Threat / attack surface + how bebop constrains it.**
- *Adversarial audio (hidden commands)*: a cochlea front-end is input to any downstream reasoner → must pass `injection_probe`/`guard::io_guard` on the *events* it emits.
- *Determinism*: the filterbank is a pure DSP transform (no RNG) → satisfies sovereign-core.
- Low risk overall; it is a benign signal primitive.

**Integration point.**
- EXTEND `rust-core/src/lib.rs` with a `gammatone_bank(signal, erb_centers) -> Vec<Vec<f64>>` built on the existing `sinc` primitive (gammatone = modulated sinc/ERB window). Then bridge to `wavefield.rs` as a **novelty/spike front-end**: spike events become field "impulses" whose propagation is already simulated by `field_physics.rs`.
- Primitive map: **Cochlea filterbank ⇄ `rust-core` vector math (`sinc`, `cosine_similarity`)**; **spike events ⇄ `wavefield` impulses**. Candidate to finally wire the **Bargmann-Fock** (Hilbert/tensor) Dossier item (see table) — a cochlea's channels form a Hilbert-space basis; spike events are occupation-number-like states.
- NEW crypto? No. NEW math? **Minimal** — a gammatone/ERB filterbank, constructible from existing `sinc`.

**RED + GREEN Verified-by-Math spec.**
```
fn gammatone_bank(x: &[f64], centers: &[f64], fs: f64) -> Vec<Vec<f64>>
GREEN: white-noise input through N ERB bands yields N non-negative,
       energy-conserving channel envelopes (Σ band-energy ≤ input-energy + ε).
RED-1: a band centered at fs/2 (Nyquist) must NOT amplify (no >0 gain above
       Nyquist — aliasing guard, ties to Nyquist stability already in stabilizer).
RED-2: identical input ⇒ byte-identical output across two runs (determinism;
       no RNG in the bank).
RED-3: cosine_similarity between two channels of a pure tone matches the
       expected ERB overlap (math check, not eyeball).
```
**Ethics flag:** ✅ clean (bio-signal DSP). No surveillance implication; it is a general signal front-end.

---

## Dossier → bebop primitive mapping table

Master-Dossier items (29149–29164) status in `feat/wire-native-core`:

| Dossier item | Math it contributes | bebop home | Status |
|--------------|--------------------|-----------|--------|
| Platonic solids | Spatial index (V−E+F=2) | `geometry_field.rs` (`Platonic`, `node_harmonic_field`, Euler-invariant tests) | ✅ **WIRED** |
| Spherical Harmonics Y_l^m | Wave propagation / Novel Wave | `geometry_field.rs` (`node_harmonic_field` coeffs `(l,m,a)`) | ✅ **WIRED** |
| Princess Pi | Pseudonymous attribution | `pod.rs` (DeliveryClaim + vault hybrid sign) | ✅ **WIRED** |
| Kill-switch | Consensus suspension | `guard.rs` (`KillSwitch`, ≥2/3, self-vote ignored) | ✅ **WIRED** |
| Nyquist stability | L5 adapt must not oscillate | `stabilizer.rs` (V̇≤0 freeze; alias/oscillation guard) | ✅ **WIRED** (per context) |
| Noether (flow conservation) | Conserved flow / charge | — *not present in `src`* | ⬜ **NOT YET** — candidate: `ledger.rs`/flow-conservation invariant on token/delivery flux |
| Bargmann-Fock (Hilbert/tensor) | Tensor / occupation basis | — *not present in `src`* | ⬜ **NOT YET** — candidate: **Cochlea** channel-space as Hilbert basis; `rust-core` tensor ops |
| Catalan (combinatorics of paths) | Count of non-crossing route paths | — *not present in `src`* | ⬜ **NOT YET** — candidate: `matcher.rs` path-enumeration bound / non-crossing assignment count |

**Recommended next wiring (priority order):**
1. **Catalan → `matcher.rs`** — bound the number of non-crossing courier↔order assignments; falsifiable combinatorial count.
2. **Noether → `ledger.rs`** — assert delivery-token flux conservation (Σ issued = Σ delivered + Σ in-flight); RED test on leakage.
3. **Bargmann-Fock → `rust-core` + Cochlea** — represent cochlea channels / field modes as a Hilbert space; `cosine_similarity`/`vsa_similarity` already there.

---

## Summary of deliverables

- **Video-use** → INTEGRATE (EDL + self-eval gate → `research_patterns.rs` + `guard::io_guard`). Clean.
- **Sentinel Pro 3.1** → 🚩 REFUSE as described (autonomous offensive OSINT, unverified, surveillance-for-harm). Only the already-present `TargetScope` defensive gate applies.
- **Torlink** → INTEGRATE as a gated `zenoh`/`matcher` Transport for authorized content-addressed payloads. Neutral; POD + sandbox-gated.
- **Shattermind** → ⚠️ DEFER — no verifiable source exists; do not fabricate. Resilience pattern (if real) maps to `stabilizer`/`wiring` ensemble freeze.
- **Cochlea** → INTEGRATE (`rust-core` gammatone bank on `sinc`; spike events → `wavefield`). Clean; also unlocks the Bargmann-Fock Dossier item.
- **Dossier table** produced: 5 items already wired (Platonic, Spherical Harmonics, Princess Pi, Kill-switch, Nyquist); 3 candidate next-integrations identified (Noether, Bargmann-Fock, Catalan) with concrete target modules.

> All five specs reuse EXISTING bebop primitives — no new crypto (vault/SHA512 present), no reinvention. No file in `/root/bebop-repo` was modified (read-only task).
