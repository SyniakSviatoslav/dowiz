# BLUEPRINT — Item 28: Optical/pixel context compression for living-memory archival tiers (NEEDS-OPERATOR-DECISION first)

- **Date:** 2026-07-19 · **Tier:** parallel lane / §F living-memory (roadmap §F) · **Status:**
  BLUEPRINT (planning artifact, no code) — **PURSUE ruling (roadmap §0), pilot scoped to the archival
  plane only, sequenced after item 20** ("since it consumes the same durability machinery").
- **Sources (read this session):** `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §F (line 408:
  item 28 optical compression, ruled PURSUE, after item 20), §0 gate (line 20, "archival/display-plane
  content only, never the P0/P1 determinism planes");
  `SPACE-GRADE-KERNEL-ARCHITECTURE-SYNTHESIS-2026-07-19.md` §20 (lines 364–370, the full research +
  two tensions), §9 addendum item 28 (line 402), §21 (energy-first telemetry); item-20 blueprint (the
  durability machinery this consumes).
- **Relationship to item 20:** item 20 (P95 persistence) decides *durability*; item 28 decides
  *representation for the archival tiers only* (synthesis §20(b): "P95 decides durability; this
  decides representation for the archival tiers only"). Item 28 rides item 20's durability layer.

---

## 1. Scope / goal (one paragraph)

Pilot **optical/pixel context compression** — DeepSeek-OCR-style rendering of text into a small set
of vision tokens (arXiv:2510.18234; under 10× compression, 97% decode precision; at 20×, ~60% —
explicitly lossy, degrading predictably) — for the living-memory **archival tiers only**, as a
*representation* choice for older/lower-priority retrieval content that item 20's persistence layer
stores durably. The roadmap ruling is **PURSUE** (roadmap §0), but the item is gated `NEEDS-OPERATOR-
DECISION` on **two tensions the synthesis refuses to resolve unilaterally** (synthesis §20(c)): (1)
a vision-capable model artifact is *weights*, not a Cargo dependency — whether §0's zero-dep
constraint's *spirit* extends to model-weight dependencies is a genuine scope question only the
operator can rule on; (2) the technique is **lossy**, and the plane boundary is **absolute** — under
§10/P6's plane-ranking, anything on the determinism planes (idempotency keys, signatures, gate
verdicts, any hash surface) is categorically excluded ("97% fidelity on a signature is 100%
failure"); the only legitimate candidates are the same forensic/display/archival plane §10/P6 already
assigns logger timestamps to. So this blueprint's job is to **present the two rulings with the
numbers as the evidence package**, and — *only if ruled in* — specify a pilot on a fixed local corpus
through the local GGUF path (no network dependency), with a structural test proving optically-
compressed bytes cannot reach any hash/signature/idempotency surface.

---

## 2. Verified current state — grounded

- **The research is verified (synthesis §20(a), direct paper fetch).** DeepSeek-OCR
  (arXiv:2510.18234): DeepEncoder renders a page → small vision-token set, decoded by a 3B-MoE text
  decoder; **<10× → 97% precision, 20× → ~60%**, lossy-but-predictable. Corroborated by "Text or
  Pixels? It Takes Half" (arXiv:2510.18279): ~50% fewer decoder tokens at 97–99% accuracy. The
  mechanism limit: vision tokens are not universally denser — rendering resolution/density is the
  compression knob; strictly requires a vision-capable model on the receiving end.
- **Local feasibility is confirmed (synthesis §20(a)).** The 3B decoder runs locally via GGUF
  quantizations — llama.cpp (PR #17400) and Ollama (`ollama run mike/deepseek-ocr`) — "real and
  confirmed, not lab-only." So a pilot needs **no network dependency** (the P95-C-style local-corpus
  discipline holds).
- **The consumer is item 20's archival tier.** Item 20 (P95) persists the living-memory index; its
  archival tiers (older/lower-priority content, tier-demoted via the move-not-delete rule, P95 §3.4)
  are the *only* legitimate optical-compression candidates. The determinism-plane content (idempotency
  keys, the hash-chained event log, gate verdicts) is categorically excluded.
- **The plane-ranking discipline is coded and load-bearing.** `event_log.rs` content-ids are sha3
  hashes (the determinism plane); logger/FDR timestamps are explicitly forensic-plane (items 4+29,
  "timestamps are P3-plane and must never reach any hash/signature/idempotency surface"). Item 28's
  structural test extends this exact discipline to optically-compressed bytes.
- **No optical-compression code exists** — green field. And per §0, model weights are ruled *outside*
  §0's compiled-Rust-crate scope, so this is not a `cargo tree` dependency question — it is the *new
  category* of dependency question (§3 open ruling).

---

## 3. Implementation plan — the decision package first, the pilot second (gated)

**Phase A — the `NEEDS-OPERATOR-DECISION` package (buildable now, no code):**
`docs/design/OPTICAL-COMPRESSION-DECISION-2026-07-19.md` presenting:
- **Tension 1 (model-weight dependency):** §0's line governs crates compiled into the kernel binary;
  a vision-model artifact is weights, not a crate. Present the question — does the zero-dep
  constraint's *spirit* extend to model artifacts? — with §20(a)'s numbers and the local-GGUF-no-
  network feasibility as evidence. **Do not resolve it** (synthesis §20(c): "only the operator can
  rule on").
- **Tension 2 (lossy + plane boundary):** present the absolute exclusion — determinism planes
  categorically out; only forensic/display/archival content is a candidate. This half is *not* an open
  question (the plane boundary is settled law); it is stated so the operator sees the scope is already
  fenced to the archival plane before ruling on tension 1.
- **The evidence package:** the compression/accuracy numbers (§20(a)), the energy cost in the §21 unit
  system (cycles/joules per operation — a token-only cost report is incomplete, §21), and the
  local-corpus pilot design (Phase B) so the operator rules with the full shape visible.

**Phase B — the pilot (ONLY if ruled in):**
- A pilot on a **fixed local corpus** of forensic/archival-plane retrieval content through the local
  GGUF path (no network dependency), measuring compression ratio and round-trip decode accuracy
  against the published <10×/97% baseline.
- **The structural unrepresentability test (the §1.5 standard applied to the plane boundary, not a
  convention):** a test demonstrating **optically-compressed bytes cannot reach any hash, signature,
  or idempotency surface** — the same discipline `event_log.rs` uses to keep timestamps off the hash
  plane. This is the load-bearing safety property; it makes the plane boundary a *type-level*
  guarantee, not a reviewer's promise.
- Rides item 20's durability machinery (the archival tier's persistence) — item 28 adds the
  representation codec, not a new durability layer.

**No `kernel/src/` code in Phase A.** Phase B, if reached, is scoped to the archival plane and the
local model path — model weights live outside the Cargo graph, so `cargo tree -e no-dev` is unaffected
regardless (the pilot integrates the model at a runtime seam, like the QRNG provider's opt-in network
seam, `pq/entropy.rs`).

---

## 4. Tests / proofs — 5-point hardening applicability

Phase A is a decision doc (no proofs beyond the evidence package). Phase B (if ruled in):

- **Item 5 (formal / structural — the headline):** the **plane-boundary unrepresentability test** —
  optically-compressed bytes cannot reach a hash/signature/idempotency surface (synthesis §9 item 28:
  "the §1.5 unrepresentability standard applied to the plane boundary, not a convention"). This is a
  structural/type test, not a runtime check — the strongest form.
- **Item 1 (oracle):** the round-trip decode accuracy measured against the published baseline (the
  numbers ARE the oracle — 97% at <10×); a corpus-level differential, honestly a *measured* oracle not
  an exhaustive one (the technique is inherently lossy, so byte-identity is impossible by design — the
  oracle is "decode accuracy ≥ published baseline", recorded as `measured-lossy-oracle` in the
  manifest, the honest weak form).
- **Item 2 (dudect):** **N/A** — archival compression, no secret-dependent timing.
  Record `N/A(no-secret-compare)`.
- **Item 3 (debug-differential):** **N/A** — no per-call correctness reference for a lossy transform;
  the accuracy oracle is corpus-level. Record `N/A(lossy-corpus-oracle)`.
- **Item 4 (asm):** **N/A** — no branch-free constant-time path.

**Energy accounting (synthesis §21, binding):** any pilot report includes cycles/joules per operation
alongside token counts; a token-only cost report demonstrably fails the §4 checklist review (the
energy-first telemetry rule — the `hw` field is first-class from items 4+29).

---

## 5. Acceptance criteria (falsifiable) — synthesis §9 item 28

1. **The operator ruling is recorded whichever way it goes** (the model-weight-dependency question +
   the plane boundary) — the Phase-A decision package's deliverable.
2. **If ruled in:** a pilot on a fixed local corpus through the local GGUF path (no network dependency)
   with **measured compression ratio and round-trip decode accuracy reported against the published
   <10×/97% baseline**.
3. **A structural test demonstrating optically-compressed bytes cannot reach any hash, signature, or
   idempotency surface** — the plane-boundary unrepresentability proof (type-level, not convention).
4. **The pilot report includes cycles/joules per operation** (energy-first telemetry, §21); a
   token-only report fails the checklist review.
5. **`cargo tree -e no-dev` unchanged** (model weights are outside the Cargo graph; the integration is
   a runtime seam, not a crate dependency).

---

## 6. Dependency gates

- **PURSUE, but `NEEDS-OPERATOR-DECISION` first** (roadmap §0, synthesis §20(c)) — Phase A (the
  decision package) can be built now; Phase B (the pilot) is gated on the operator ruling.
- **After item 20** (roadmap §F: "sequenced after item 20 since it consumes the same durability
  machinery") — the archival tier's persistence must exist for optical compression to store into.
- **Independent of items 9/21/27** — a representation choice for archival content, not a control-loop
  or breaker concern.

---

## 7. Open questions (operator ruling — the item IS two rulings)

1. **Does §0's zero-dependency constraint's spirit extend to model-weight dependencies?** (Synthesis
   §20(c), tension 1.) §0 governs Cargo crates compiled into the binary; a vision-model artifact is
   weights loaded at a runtime seam. This is a **genuine scope question only the operator can rule on**
   — the synthesis "declines to resolve it by stretching §0's wording in either direction." **Flagged,
   not invented.** The evidence package (§20(a) numbers + local-GGUF-no-network feasibility) is the
   input; the ruling is the operator's.
2. **The plane boundary is settled, not open** — determinism planes categorically excluded; only
   forensic/display/archival content is a candidate. Stated so the operator's tension-1 ruling is made
   with the scope already fenced. (Recorded as a *constraint*, not a second open question.)
3. **If ruled in, is a lossy archival representation acceptable for living-memory recall quality?** A
   secondary product-quality judgment: 97% decode fidelity on archival prose means some recall
   degradation on the coldest tier. Whether that tradeoff is acceptable for the dowiz living-memory
   product is an operator/product call the pilot's measured numbers inform. Named as a downstream
   ruling contingent on tension-1 being ruled in.
