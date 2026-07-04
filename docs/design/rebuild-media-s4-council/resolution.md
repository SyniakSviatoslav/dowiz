# S4-MEDIA Port — Council RESOLVE

> **Verdict: PROCEED-WITH-REVISIONS + one liftable ETHICAL-STOP (counsel).** Packet-status:
> **🟡 — NOT COUNCIL-APPROVED until the operator signs §4 (incl. the STOP lift choice).**
> Seats: architect (packet, amended by spike-evidence) · breaker (1 CRIT / 3 HIGH / 4 MED / 4 LOW) ·
> counsel (PROCEED-WITH-REVISIONS + scoped ETHICAL-STOP) · lead (this RESOLVE).
> Q3 (image stack) is **decided by machine evidence** (spike-evidence.md): pure-Rust `image` 0.25 +
> `webp` 0.3; libvips rejected (link-time system-dep = "works in Docker, broken locally" class).

## 1. Frozen revision set

- **REV-S4-1 (breaker C1 → CRIT — decode bomb cap).** Every decode call site (product, logo,
  entry-photo, brand-extractor rider) sets `image::Limits` BEFORE decode: pixel cap replicating
  sharp's default bound (268 MP) or tighter, plus an allocator cap; combined with the existing
  body-size bounds. DoD test: crafted small-file/huge-dimension PNG (e.g. 65535×65535) → clean 400,
  not OOM. The spike's benchmark path had NO limits — that path must not ship.
- **REV-S4-2 (breaker H1+M4, counsel #4 → Q2 RESOLVED).** Hand-rolled SigV4 query-presign is
  **REJECTED** (the `aws-sign-v4` crate has no presign API — "extend it" = from-scratch crypto,
  understated). Q2 resolves to **token-proxy-PUT preserving the FE contract shape**: the existing
  presign route keeps its request/response shape, but the returned URL points at a Rust
  proxy-upload endpoint authorized by a short-lived opaque token (single-use, scoped to
  key+content-type+max-bytes, TTL 300s parity). Deletes the presign-crypto class AND the
  leaked-wide-URL class; unifies all three upload paths (two already proxy). Direct-to-R2 offload
  via `aws-sdk-s3` presigner remains the recorded option IF a media-worker runtime ever stands up
  (post-OCR slice), not now. Body fully buffered (bounded by REV-S4-1 + size caps) — no streaming
  claim survives (M4).
- **REV-S4-3 (breaker H2 → parity oracle).** "Decided by spike" ≠ "parity proven". Build carries a
  golden-fixture parity suite: phone JPEG w/ EXIF, PNG, odd aspect ratios, 1×N edge, CMYK JPEG,
  16-bit PNG — goldens generated ONCE from the live sharp stack, committed as fixtures; assertions
  on exact output dimensions + quantified perceptual tolerance (channel-mean delta / dSSIM bound),
  NOT the E2E net's `\.webp$` regex (a null oracle). JPEG decode explicitly benchmarked (spike was
  PNG-only).
- **REV-S4-4 (breaker H3 → EXIF path).** The shipping decode uses `ImageReader` (orientation-capable;
  the spike's `load_from_memory_with_format` path cannot read orientation). DoD fixtures cover ALL
  8 EXIF orientation values — including mirrored 2/4/5/7 (a single rotated fixture is a test as
  narrow as the bug).
- **REV-S4-5 (packet §risk-1, inherits S3 REV-2/REV-10).** The two GUC-less media-metadata writes
  (`spa-proxy.ts:252` raw query; `themes.ts:139-143` raw connect) are sanctioned FIX-IN-PORT onto
  the `with_user` seam with in-txn membership. The unauthenticated entry-photo route seats NO user
  by design — its DB touchpoint (if any) is explicitly declared service-scope, never a borrowed
  user GUC; document in the tenancy-GUC ADR.
- **REV-S4-6 (counsel Q4 floor, breaker M3).** Entry-photo CARRY with the Q4b floor made concrete:
  magic-byte sniff, per-IP 8/min carry, a NUMERIC global cap (kill-switch env, spec'd in the build
  packet), REV-S4-1 bomb bound. **Q4c (checkout-scoped upload token) is recorded as the root fix**
  for content-hosting abuse — staged as its own operator decision (funnel-impact), not smuggled in.
- **REV-S4-7 (counsel ETHICAL-STOP — delivery-photo erasure gap).** VERIFIED: `anonymizeOrder`
  (`apps/api/src/lib/anonymizer/index.ts:237-276`) nulls text address/instructions but leaves
  `delivery_photo_key` set and purges no object (only `avatar_key` is ever deleted, :169) — a
  doorway photo survives GDPR erasure of its own order, public-by-key, indefinitely. **STOP lift
  options (operator picks one, §4):** (1) extend the avatar-purge pattern to `delivery_photo_key`
  in the OLD-stack anonymizer (~5 lines, red-line PII code — ships with its own red→green
  guardrail + ledger row), recommended; (2) delivery-window retention TTL on those objects;
  (3) recorded accepted-risk with owner + trigger. The S4 PORT must additionally carry the
  erasure-graph link: order-referenced media keys are IN the erasure path by design. Retention is
  the primary PII control for pixel-content (faces/plates/doorways) — metadata-strip does nothing
  for it (counsel #3).
- **REV-S4-8 (breaker M1/M2/L1-L4 → register).** Carried-with-register: logo fixed-key + immutable
  1yr cache (re-upload never busts — follow-up owner: FE/asset pass) · 150MB budget TOCTOU over
  client-declared bytes (real enforcement = product decision, deferred) · entry-photo key as
  unbound client-held string into S5 (record) · confirm sort_order race (carry) · SigV4 canonical
  path encoding (latent, fail-safe; covered by REV-S4-2 rejecting hand-rolled presign anyway) ·
  brand-extractor SSRF-fetched decode gets REV-S4-1 limits by construction.
- **REV-S4-9 (counsel #5 — no dead scaffolding).** Two-image split: ship the `trait ImageProcessor`
  seam only; do NOT author the media-worker Dockerfile now (unbuilt scaffolding drifts); the split
  triggers on the OCR slice or a measured CPU ceiling, each its own decision.

## 2. Seat disposition table

| Finding | Sev | Disposition |
|---|---|---|
| Breaker C1 | CRIT | ACCEPTED → REV-S4-1 |
| Breaker H1 | HIGH | ACCEPTED → REV-S4-2 (Q2b rejected outright) |
| Breaker H2 | HIGH | ACCEPTED → REV-S4-3 |
| Breaker H3 | HIGH | ACCEPTED → REV-S4-4 |
| Breaker M1 | MED | CARRY + register (REV-S4-8) |
| Breaker M2 | MED | CARRY + register (REV-S4-8) |
| Breaker M3 | MED | ACCEPTED → REV-S4-6 (numeric cap) |
| Breaker M4 | MED | ACCEPTED → folded into REV-S4-2 |
| Breaker L1–L4 | LOW | register (REV-S4-8) |
| Counsel STOP | — | REV-S4-7, lift = operator §4 |
| Counsel Q4c elevation | — | REV-S4-6 (staged root fix) |
| Counsel Q2 class-removal | — | REV-S4-2 |
| Counsel lazy-Dockerfile | — | REV-S4-9 |

## 3. Question resolutions

- **Q1 → in-process transcode, seam-only** (REV-S4-9). No second runtime now.
- **Q2 → token-proxy-PUT** preserving the presign flow contract (REV-S4-2). 🔴 operator.
- **Q3 → CLOSED by spike evidence** (pure-Rust image+webp; REV-S4-1/3/4 carry its gaps).
- **Q4 → CARRY + Q4b floor; Q4c staged** (REV-S4-6). 🔴 operator.
- **Q5 → per packet** (with_user seam; REV-S4-5).
- **Q6 (orphans) → carry the leak, register row; reaper deferred to the ops-binary slice.**
- **Q7 (content-hash integrity) → CARRY; mime magic-byte re-sniff is the gate (REV-S4-6).**

## 4. 🔴 OPERATOR SIGN-OFF REQUIRED (blocks COUNCIL-APPROVED → build)

1. **Q2** — token-proxy-PUT replaces presign (contract shape preserved; no hand-rolled SigV4).
2. **Q4** — entry-photo CARRY with the Q4b floor; Q4c checkout-scoped token staged as a separate
   product decision.
3. **ETHICAL-STOP lift (REV-S4-7)** — pick one: (1) 5-line old-stack anonymizer fix extending
   avatar-purge to `delivery_photo_key` [recommended], (2) retention TTL, (3) recorded
   accepted-risk. The S4 port carries the erasure-graph link regardless.
4. **Q1/REV-S4-9** — in-process + seam-only posture (no media-worker runtime now).

## 5. Build DoD deltas (added by this RESOLVE)

- Bomb-cap rejection test (REV-S4-1) · golden-fixture parity suite incl. JPEG/CMYK/16-bit (REV-S4-3)
  · all-8-orientations fixtures (REV-S4-4) · upload-token single-use/scope/TTL tests (REV-S4-2)
  · erasure-graph: order-media keys reachable from the erasure path (REV-S4-7) · numeric global
  cap + kill-switch env in EnvSchema (REV-S4-6, red-line: no raw env reads).
