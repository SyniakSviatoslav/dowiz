# R1 — Layout-Oriented Versioning + Zero-Copy Bridges: grounding against the decided corpus (2026-07-17)

> Research + adversarial-grounding pass over the "Layout-Oriented Versioning" + "Zero-Copy
> Bridges/Adapters" cluster of `00-SOURCE-DIALOGUE.md`, per the operator's "враховуючи наявні
> плани та роадмапи" framing. This is an EXTENSION pass: the job is to locate where these ideas
> extend, duplicate, or conflict with already-decided work — not to re-derive settled ground.
>
> Style: Anu (every load-bearing claim is a live `file:line` read or a ledger-row cite) /
> Ananke (a good outcome must be structural, not hoped). No metaphor. Read this pass:
> `BLUEPRINT-P-A/-B/-C`, doc 19 (Parts 1+2), mesh-masterwork V2 §A + verdict ledger,
> bebop2 `proto-wire/framing.rs`+`envelope.rs`, kernel `spectral_cache.rs`/`spectral.rs`,
> bebop2 `core/rng.rs`, spectral-evolution `spectral.rs`.

---

## 0. One-paragraph answer

Layout-versioning / Header-as-Type is **NOT new ground** — the mesh already carries a
self-describing, fail-closed version discriminant *inside the signed envelope*
(`bebop2/proto-wire/src/framing.rs:54-60`, `envelope.rs::ENVELOPE_VERSION`), with an
unknown-version → hard-reject test that closes a prior red-team finding
(`framing.rs:102-114`, "closes red-team B3 F4"). The dialogue's cleartext "Magic Number in the
first bytes" is the exact carrier the ledger already **REJECTED-ON-SECURITY** (items 93/157:
cleartext L2 header = security regression; fields go *inside* the signed envelope). The
type-law philosophy the dialogue calls "Layout as Law / Assertion by Panic" is already the
kernel's `NormalizedTile`/`TileAddress` discipline (P-B §3.2: hashing an unnormalized tile is a
*compile* error). The side-car is admissible **iff** it grants zero trust (a re-verified
supply-path translator), inadmissible if the kernel trusts its output (a control-path proxy) —
the no-proxy rule targets the latter, not the former.

---

## 1. Layout-versioning / Header-as-Type — verdict: **EXTEND-EXISTING (mostly ALREADY-EQUIVALENT)**

### 1.1 What already exists (live reads)

| Dialogue concept | Already-built / already-decided | Cite |
|---|---|---|
| SchemaID / version in the block header | `Envelope.version` gated fail-closed at the framing boundary: `if envelope.version != ENVELOPE_VERSION { return Err(WireError::VersionMismatch(v)) }` | `bebop2/proto-wire/src/framing.rs:54-60`; `envelope.rs::ENVELOPE_VERSION` |
| "unknown SchemaID → Self-terminate, no conversion attempt" | already the behavior — an off-version envelope does **not** decode; a red-team test pins it | `framing.rs:102-114` (`unknown_version_is_rejected_on_decode`, "closes red-team B3 F4") |
| "Magic Number in the first bytes" (cleartext) | **REJECTED-ON-SECURITY**: a cleartext header is a security regression; version is carried *inside the signed envelope* | mesh-V2 items 93/157; `framing.rs:57` comment ("carry version inside the signed envelope") |
| EpochID in the header | **ADOPT (new ground)** — monotone logical epoch, max-merge, no wall-clock, inside the signed envelope | mesh-V2 items 21/32/156/159 (W2-L4 / W3-L6) |
| "Layout as Law / Assertion by Panic" (type-enforced, physics-not-policy) | already the `NormalizedTile`/`TileAddress` newtype invariant: an unnormalized tile is *unrepresentable*; a bad read is a compile hole, not a runtime check | P-B §3.2; landed in `dowiz/kernel/src/spectral_cache.rs:20,209-214,272` + `csr.rs` newtypes |
| Immutable Pinned snapshot + atomic pointer swap | **PARTIAL-EXISTING + bridge**: `RetainedBase` admission (P-B §4) + arena snapshot-then-drop; durable half is the P12 gap | mesh-V2 item 151; doc 19 §1.2; P-B §4.3 |
| Hash(Snapshot_N) heartbeat compare → rollback | **ADOPT-the-structural-form**, but as a degrade-closed *inline* consequence (content-address compare → re-derive), **not** a polling corrector | mesh-V2 item 152 |
| Merkle-per-tile / per-tile root for cross-node trust | **EXTEND-EXISTING + BRIDGE**: `matrix_content_address` per tile IS a per-tile root; bridge-gap #1 (normalize-before-hash) makes it cross-node-meaningful | mesh-V2 items 57/91; doc 19 §1.3 |

### 1.2 The one nuance the dialogue flattens — and it is already decided both ways

The dialogue says unknown-schema → **Self-terminate** everywhere. The decided corpus splits this
by **layer**, and the split is load-bearing for a *fail-operational* mesh:

- **Wire-decode layer** — unknown `version` → **hard reject / self-terminate the decode**
  (already built, `framing.rs:59-60`). Correct: a byte-stream you cannot parse is not
  fail-operational salvageable.
- **DecisionUnit / semantic layer** — a stale or out-of-context unit → **Escalate / re-pull,
  unconditionally**, *not* death (mesh-V2 items 20/22: "auto-reject-stale-and-Escalate ADOPT;
  auto-detect-degradation-by-quality-scoring REJECT"). This is precisely the fail-operational
  posture the *later* half of the same dialogue argues for (Gradient State, graceful
  degradation) — so the dialogue self-corrects; the corpus already encodes the resolution.

Flat "Self-terminate on unknown SchemaID" is therefore right at the parser and wrong at the
policy layer; both verdicts already exist. This is an EXTEND (naming the two-layer split
explicitly in a header spec), not new architecture.

### 1.3 Verdict

**EXTEND-EXISTING.** Header-as-Type duplicates the envelope-version machinery and the
`NormalizedTile` type-law; its cleartext-Magic-Number variant is **REJECT-ON-SECURITY** (items
93/157). The only genuinely additive sliver is a *tile/tensor-block-level* schema tag distinct
from the envelope version — and even that must land **inside the existing sha3
content-addressed registry, not as a second authority** (items 23/47: a second content-addressed
authority is the dual-authority hazard). Net: substantially covered; no fresh blueprint
warranted, only a one-section spec note folding the two-layer self-terminate/Escalate split into
the envelope doc.

---

## 2. CRC32/CRC64 hardware-accelerated integrity — verdict: **DEFER-WITH-TRIGGER (do NOT add as a competing authority)**

### 2.1 Ground truth

`grep -rniE "crc32|crc64|_mm_crc32"` over `kernel/` and `bebop2/**/*.rs` → **zero hits.** CRC is
literally unbuilt. But "unbuilt" ≠ "gap," because the role is already served at two distinct
points, and CRC would be a **third** hash authority:

- **Cross-node / tamper integrity** — sha3 content-address (Merkle root over sorted content-ids,
  `bebop2/proto-wire/src/sync_pull.rs:412-481`) + FNV-1a-64 per-tile content-address
  (`spectral_cache.rs:103-111`, `matrix_content_address` / `canonical_content_address`). FNV is
  the *fast, non-crypto* per-tile fingerprint; sha3 is the *tamper-evident* cross-party root.
  The FNV/sha3 split is deliberate (doc 11 §3).
- **Per-packet wire integrity** — QUIC transport checksums. Item 95 ("silence = success, QUIC
  handles reliability") makes a per-packet CRC redundant. Note CRC32 appeared in the dialogue's
  intellectual ancestor — the 32-byte L2 frame `MAC/EtherType/TileID/Seq/Payload/CRC32` (item
  93) — and that **whole carrier was rejected** (no shared L2; QUIC already covers it).

### 2.2 The one niche CRC alone could fill, and why it still defers

The dialogue's specific pitch is **1-cycle HW CRC for real-time L1-L2 integrity of a pinned
immutable snapshot** — i.e. detecting silent *RAM bit-rot* between the atomic pointer swap and
the next read. Nothing else targets exactly memory-fault (as opposed to tamper or wire) detection.
But the existing model already **recomputes the content-address on cache access and compares** —
item 152's "checksum-vs-expected = the content-address compare" — so a divergent FNV address *is*
the corruption signal today. CRC is therefore **complementary only as a speed optimization** of
that recompute (HW CRC is ~1 cycle/8B; FNV over a full tile is O(nnz)), and only worth building
if a measured maintenance-pass profile shows the content-address recompute is hot.

**Verdict: DEFER-WITH-TRIGGER.** Build a HW-CRC fast-path for snapshot self-integrity *iff* the
FNV content-address recompute is measured on a hot path (append the number to `BENCH_HISTORY.md`
first). Until then the FNV content-address already **is** the integrity check — adding CRC as a
standing second checksum authority is the dual-authority hazard (items 23/47), not a gap closure.
If built, it must be a private *acceleration of* the content-address recompute (same verdict
bytes), never an independent trust signal.

---

## 3. The three bridge patterns — verdict each (they are NOT equivalent)

### 3.1 Zero-Copy Shims (inline raw-pointer transformers) — **ADOPT-AS-PATTERN, with the P-B constraint**

No separate process; compile-time/inline; "memcpy with a struct-offset." The repo's zero-copy
intent is *already* met by the content-addressed opaque payload + fixed-layout TLV envelope (item
55 **REJECT** FlatBuffers-as-dep: `framing.rs` is zero-dep, byte-deterministic). An inline shim
that reads old-layout bytes and emits new-layout bytes is admissible **iff** two constraints hold,
both already law in the corpus:

1. **Determinism boundary** — the transform stays rational / fixed-order, no transcendental
   (`rng.rs:22-28`; doc 19 §1.3). A shim that rescales or re-pools with `exp`/softmax breaks
   cross-node hash convergence — the same physics that killed the RGB-seed codec.
2. **Output is a canonical form, not raw bytes** — the shim's product must land as a
   `NormalizedTile` (P-B §3.2), else it reintroduces bridge-gap #1 (a translated-but-unnormalized
   tile hashes differently on every node). Type the shim's return as `NormalizedTile`, not
   `&[u8]`, and the invariant is structural.

Low risk. This is EXTEND of the existing zero-copy discipline.

### 3.2 Layout Aliasing (union/struct-overlay + CommonHeader, static dispatch) — **ADOPT-THE-INTENT via tagged enum; REJECT the literal C-union**

The *intent* — read a `CommonHeader`, statically dispatch on the version to the right view — is
exactly what the envelope already does (`framing.rs` matches on `version`). But the literal
mechanism (a C `union` / `repr(C)` overlay read through a raw pointer) is **`unsafe` and
type-law-violating**: a mis-tagged union read reinterprets bytes silently — the precise opposite
of `NormalizedTile`'s "invalid state is unrepresentable." The safe-Rust equivalent that preserves
static dispatch is a **`#[non_exhaustive]` enum tagged by the version discriminant + `match`** —
static dispatch on a CommonHeader **without** `unsafe`. (Note item 114, the `AtomicU32`-versioned
cache-line-aligned tile header, is itself only **DEFER-WITH-TRIGGER** — it needs `repr(align(64))`
and an `AtomicPtr` consumer that does not yet exist.) **Verdict: adopt the tagged-enum form;
reject the union overlay** — it buys nothing the enum doesn't and forfeits the type-safety-as-law
the whole architecture rests on.

### 3.3 "Bridge as a Side-car" (separate adapter process/thread) — see §4 (the careful adjudication)

---

## 4. Side-car vs the no-proxy rule — the toy-model adjudication (doc-19 method)

**The tension, stated exactly.** The operator's standing rule this session (doc 19 header; P-C
§2): *"remove the proxy/watchdogs/so-called authority"* — zero separate supervising processes in
the safety architecture. The dialogue's "Bridge as a Side-car" is explicitly *"окремий
адаптерний потік/процес"* (a separate adapter process/thread) the kernel depends on to see only
translated data (`00-SOURCE-DIALOGUE.md:54-58`). Literally a separate process. Does it violate
the rule?

### 4.1 The doc-19 categorical test, applied

Doc 19 §2.3 established (with a compiled, executed toy) that "a check by a separate process"
splits on **three axes**, and the operator's target is axis 1 (liveness/supervision):

- **Watchdog** (rejected): absence is a **silent runtime gap** — the bad state sits there
  unobserved (`balance = −20` forever), correctness *depends on a separate live process*, and
  "who watches the watchdog" is a live regress.
- **Inline verify-before-persist / `key_V`** (adopted): absence is a **compile hole** — the
  guarded result *cannot be produced* without the check, because the check sits on the causal
  path of the value it guards. Zero liveness regress; bad state never representable.

Apply each axis to the side-car. The verdict **branches on one question: does the kernel trust
the side-car's output, or re-verify it?**

### 4.2 The toy (mirroring doc 19's Pattern A/B, adapted to translation)

```
INVARIANT under guard: no tile is committed unless it passes the SAME inline gate
(NormalizedTile::canonicalize → classify_drift reject-Unstable → content_address)
that every raw network tile passes. (mirrors event_log.rs:389 commit_after_decide_drift_gate.)

── PATTERN A' : SIDE-CAR AS TRUSTED PROXY (control-path) ──
  legacy tile ──▶ [side-car translates + self-validates] ──▶ kernel COMMITS on the
                                                             adapter's word (no re-verify)
  A translation BUG in the adapter emits a tile that violates the drift bound.
  Kernel trusts it ⇒ bad state COMMITTED. Correctness depends on a separate process
  being both ALIVE and HONEST. Adapter death/compromise = SILENT fail-open.
  ⇒ axis-1 SILENT GAP. This IS the watchdog/proxy class. VIOLATES the rule.

── PATTERN B' : SIDE-CAR AS UNTRUSTED TRANSLATOR (supply-path) ──
  legacy tile ──▶ [side-car translates] ──▶ kernel input buffer
                                            ──▶ kernel runs the SAME inline gate it
                                                runs on ALL input, then commits
  A translation BUG emits a non-canonical / Unstable tile ⇒ FAILS the inline gate
  ⇒ rejected pre-persist ⇒ bad state NEVER produced (doc 19 §2.3 axis 2).
  Adapter DEATH ⇒ that schema's input buffer goes empty ⇒ kernel disables that
  adapter (00-SOURCE-DIALOGUE:63-64 Data Containment) — an OBSERVABLE, fail-
  operational degraded state, NOT a silent correctness gap.
  ⇒ axis-1 VISIBLE. The AUTHORITY stays inline (key_V shape). DOES NOT violate.
```

### 4.3 The load-bearing distinction (the "more-restricted vs grants-trust" question, answered)

The task asks: is the distinction *"a side-car that only ever produces MORE-RESTRICTED data than
raw, vs a watchdog that GRANTS additional trust/authority"*? **Yes — and it is decisive, on all
three doc-19 axes:**

1. **Direction of the arrow (axis 1).** A watchdog points **at** the kernel — it observes the
   protected thing and issues a verdict on it (control path). A translator points **away** — it
   observes *old nodes* and produces data the kernel then judges (supply path). The no-proxy rule
   is about the control path: "remove the thing that supervises the kernel." A supply-path
   translator supervises nothing; it is judged **by** the kernel, not the reverse.
2. **Trust delta (axis 3).** In Pattern B' the side-car grants the kernel **zero** trust it would
   not compute itself — its output runs the identical inline gate as raw bytes, so the side-car
   can only ever cause a tile the kernel *would already reject* to also be rejected; it can never
   cause a tile the kernel *would reject* to be accepted. That is the literal meaning of "produces
   more-restricted data, never more-trusted." A watchdog/proxy (Pattern A') grants trust — the
   kernel accepts on the adapter's word — which is `key_K` self-certification by a separate party.
3. **Absence class (axis 1 again).** B' absence is visible (empty buffer, disabled adapter,
   fail-operational). A' absence is invisible (subtly-wrong tiles committed, or the kernel blindly
   trusting a dead source).

### 4.4 Where the dialogue's own spec is INSUFFICIENT (the honest catch)

The dialogue puts validation **in the bridge**: *"Валідація зосереджена на самому мосту (adapter
self-terminates при неможливості коректного перекладу)"* (`:56-58`). That is **necessary but not
sufficient**: adapter self-validation *alone* is `key_K` — a separate party certifying its own
output. If the kernel relies on it, we are in Pattern A' (rejected). The correction is **defense
in depth, with the authority inline**: the adapter MAY self-terminate on failure (good,
degrade-closed, one fewer bad tile emitted), **but the kernel MUST ALSO re-verify** every side-car
output through its own inline gate — never trusting the adapter's self-validation. Adapter
self-check + kernel inline re-verify = admissible; adapter self-check *as the kernel's reason to
trust* = the proxy the rule removes.

### 4.5 Verdict + recommendation

**ADOPT the side-car ONLY as an untrusted, re-verified, supply-path translator (Pattern B'), and
encode the constraint as a type-law:** the side-car's output type is *raw untrusted bytes*, and
the only path from there into a committed/`RetainedBase` tile is
`NormalizedTile::canonicalize → classify_drift → content_address` — identical to network input.
Under that law the separate process is a **decode/translate offload, not a supervisor**; it passes
the no-proxy rule on all three doc-19 axes. Additional constraints already in the corpus:

- Prefer the **in-process** forms (§3.1 Zero-Copy Shim / §3.2 tagged-enum static dispatch) whenever
  the translation is cheap and the legacy codec is trusted. Reserve a *separate process* strictly
  for **isolation of an untrusted/crashy legacy codec** — and then the isolation is **Data
  Containment / MPU-style memory sandbox** (`:62`), which is fail-operational bulkheading
  (P-C's bulkhead class, item 11), **not** supervision.
- The side-car "only reads old, never writes old" property (`:57`) is correct and worth keeping —
  it bounds blast radius (an adapter crash cannot corrupt network state) — but it is a bulkhead
  argument, **orthogonal** to the trust argument above. Both must hold.

**Strongest one-sentence justification (the axis that decides it):** a watchdog's absence leaves
the bad state sitting unobserved with no signal (a silent gap; correctness depends on a separate
live+honest process), whereas an untrusted translator's absence merely empties one input buffer
(a visible, fail-operational degrade) and its *output* is judged by the kernel's own inline gate —
so a Pattern-B' side-car sits on the **supply** path being judged, never on the **control** path
doing the judging, which is exactly the process the no-proxy rule removes.

---

## 5. Cross-check against today's verification findings (point 4) — honest, not forced

### 5.1 bebop2 `core/rng.rs` Android/iOS compile failure — connection **WEAK/ABSENT**

Ground truth (`bebop-repo/bebop2/core/src/rng.rs`): the `PlatformEntropy` impl is `cfg`-dispatched
for `linux+x86_64` (`:245-251`), `linux+aarch64` (`:299-305`), and `x86/x86_64` RDRAND (`:350+`);
the module doc states the crate is **designed to fail compilation** on any target with no wired
entropy (`:8-23`, "crate fails to compile instead"). Android is `target_os = "android"` and iOS
`target_os = "ios"` — neither matches `target_os = "linux"`, so an aarch64 phone (the actual
courier target of the mesh) selects **no** arm → compile error. This is a real portability gap for
the phone-mesh, but it is a **`cfg`-dispatch completeness** boundary, not a data-layout/versioning
boundary. A Header-as-Type / layout law operates on *runtime data blocks*; it cannot and would not
"catch" a *build-target* omission. The only honest link is a shared *philosophy* — both are
fail-closed on the unrepresentable (unknown schema → self-terminate; unwired platform → won't
compile) — and rng.rs is *already* correctly fail-closed. **Layout-versioning does not structurally
catch this.** Do not force the connection; report it as a real but separate finding (add
`target_os="android"`/`"ios"` aarch64 arms, or a documented deliberate refusal, in whichever phase
owns crypto portability).

### 5.2 spectral-evolution `spectral_radius` NaN-masking — connection **REAL, but the catcher is the dialogue's *value-bound* sibling idea, not the layout header**

Ground truth (`dowiz-spectral-evolution/kernel/src/spectral.rs:217-218`):
`spectral_radius(a) = eigenvalues(a).map(|e| e.abs()).fold(0.0, f64::max)`. Because `f64::max`
returns the non-NaN operand, `fold(0.0, f64::max)` **silently masks a NaN** eigenvalue-modulus →
returns `0.0` (or the max of the finite values). A garbage/degenerate tile whose spectrum is NaN
is then reported as ρ≈0 → `classify_drift` → **Damped** → admitted as *healthy*. That is a
**fail-OPEN masking** (RC-2 vacuity shape): a corrupt tile reads as stable and passes the very gate
(`RetainedBase::admit`, drift-gate) meant to reject it.

This is a genuine "missing law" — but name it precisely: it is a **hard value-bound on physically
possible values**, which is the *other* half of the same source dialogue (the State-Keeper "exit
criteria as hard mathematical bounds — a value bound on physically possible values"), **not** the
Header-as-Type layout part of my cluster. The layout header would not catch a NaN produced by a
*correctly-typed, correctly-versioned* tile. The value-bound law would: a NaN ρ is not a physically
possible spectral radius, so the gate must **reject/self-terminate on `!rho.is_finite()`**, never
fold it to 0.0.

Sharpest corroboration that this is a real, narrow, buildable gap: the **landed** dowiz kernel
*already knows this law* — `spectral_cache.rs:117,131-135,149` canonicalizes NaN to
`CANONICAL_QUIET_NAN` and `-0.0→+0.0`, and `hydra.rs`/`integrity_check` guards `rho.is_finite()`
(P-C §1) — but that NaN discipline is applied on the **hash path** and the **hydra** path, and is
**absent from the `spectral_radius` measurement feeding the drift gate**. So the law exists in the
codebase, applied in two places and missing in the third. **Recommendation:** make `spectral_radius`
(or the `admit`/`classify_drift` caller) reject non-finite ρ as `UnstableSpectrum`/self-terminate,
mirroring `hydra.rs`'s `is_finite()` guard — a one-predicate fix, pinned by a RED-first
NaN-bearing-tile test. This belongs to P-B/P-C, and it validates the dialogue's *value-bound*
concept concretely — while confirming the *layout-header* concept is orthogonal to it.

---

## 6. Net verdicts (one line each)

| Item | Verdict | Basis |
|---|---|---|
| Header-as-Type / layout-versioning | **EXTEND-EXISTING** (mostly ALREADY-EQUIVALENT); cleartext Magic Number **REJECT-ON-SECURITY** | envelope `version`+`VersionMismatch` (`framing.rs:54-114`); items 20/21/22/23/91/93/157; P-B `NormalizedTile` type-law |
| CRC32/CRC64 HW integrity | **DEFER-WITH-TRIGGER**; never a second authority | zero CRC in tree; role served by FNV content-address (item 152) + sha3 Merkle + QUIC; item 93 CRC-frame already rejected |
| Zero-Copy Shims | **ADOPT-AS-PATTERN** (output must type as `NormalizedTile`; rational/fixed-order only) | item 55; bridge-gap #1; determinism boundary `rng.rs:22-28` |
| Layout Aliasing | **ADOPT-THE-INTENT via tagged enum; REJECT literal C-union** (unsafe, breaks type-law) | P-B §3.2 type-law; item 114 DEFER |
| Bridge as a Side-car | **ADOPT ONLY as untrusted re-verified supply-path translator** (Pattern B'); trusted-proxy form **REJECT** | doc-19 three-axis test; §4 toy |
| rng.rs Android/iOS | real gap, **NOT a layout-law miss** (cfg-completeness axis) | `rng.rs:245-350` cfg arms |
| spectral_radius NaN-mask | real fail-open; caught by the dialogue's **value-bound** law, not the layout header; **one-predicate `is_finite` reject fix** | `spectral.rs:217-218`; already-known law at `spectral_cache.rs:131-149` / `hydra.rs` |

## 7. Provenance

Live reads this pass: `bebop2/proto-wire/src/framing.rs` (`:15,54-60,102-114`),
`bebop2/core/src/rng.rs` (`:8-31,223-370`), `spectral-evolution/kernel/src/spectral.rs`
(`:205-230`), `dowiz/kernel/src/spectral_cache.rs` (`:20,92-214,272,396-427`); grep
`crc32|crc64|_mm_crc32` over `kernel/` + `bebop2/**/*.rs` → 0 hits.
Docs: `00-SOURCE-DIALOGUE.md`, `CORE-ROADMAP-2026-07-17/BLUEPRINT-P-A/-B/-C`,
`bebop2-mesh-tensor-hermetic-2026-07-17/19-SYSTEM-COHERENCE-AND-AUTHORITY-BOUNDARY-REDO.md`,
`BLUEPRINT-BEBOP2-MESH-MASTERWORK-SYNTHESIS-V2.md` (§A + verdict ledger, items cited inline).
Method: doc-19 compiled-toy adjudication reused for §4 (not re-executed; the Pattern-A/B categorical
result is cited, the translation-specific A'/B' variant is reasoned on the same three axes).
