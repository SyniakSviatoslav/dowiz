# Ping-Pong / Shadow-Copy State-Transition Idiom — Propagation Scan

> RESEARCH-ONLY. No code written, no branches touched. Recreated 2026-07-19 after
> the original (same filename/date) was accidentally deleted; re-investigated from
> scratch against the live working tree. Every claim is cited `file:line`.
>
> **Trigger (operator, 2026-07-18):** "field_frame.rs::step вже робить ping-pong
> u/u_prev через swap, ping-pong логіку можна поширити буде багато де — і варто
> поширити через застосування shadow/simulated копій." → sweep the codebase for
> other sites already using (or that could benefit from) the same
> *compute-into-a-secondary-buffer-then-atomically-promote-it* idiom, and decide
> whether it is worth extracting into a shared abstraction.

---

## 0. The reference implementation (confirmed)

`engine/src/field_frame.rs` — `FieldFrame::step()` (the wave-simulation integrator).
The state advance is **not** a single swap; it is a **three-buffer rotation via two
swaps**, `field_frame.rs:220-221`:

```rust
// u_prev, u, next_scratch already hold: old u_prev, old u, freshly-computed U_next
std::mem::swap(&mut self.u_prev, &mut self.u);          // u_prev <- old u;  u <- old u_prev
std::mem::swap(&mut self.u, &mut self.next_scratch);    // u <- next;  next_scratch <- old u_prev
```

The struct holds `u`, `u_prev`, plus two pre-allocated scratch buffers
(`lap_scratch`, `next_scratch`) allocated **once** in `new()` (`field_frame.rs:159-183`)
so `step()` performs **zero heap allocation** thereafter (`field_frame.rs:171-183`,
`204`). `U_next` is computed into `next_scratch` (`field_frame.rs:215`), then the two
swaps rotate the roles:

- `u_prev` ← old `u` (needed for the backward-difference `U̇ = (U − U_prev)/dt`),
- `u` ← the freshly computed `U_next`,
- `next_scratch` ← the now-dead old `u_prev` (recycled as next step's scratch).

Key structural properties that define **"the idiom"** for this scan:
1. **compute-into-secondary** — the new state is built in a buffer that is *not* the
   live one (readers keep seeing consistent old state during compute);
2. **atomic promote** — a pointer/handle flip (`mem::swap`), not an element copy;
3. **buffer reuse** — the retired buffer is recycled, so steady-state allocation is
   zero.

Proven bit-identical to the pre-rework arithmetic by `allocfree_step_byte_identical`
(`field_frame.rs:459-493`). This is the yardstick every other candidate is measured
against below.

---

## 1. Confirmed SAME-idiom sites (atomic-swap promotion)

| # | Site | Buffers | Promotion | Verdict |
|---|------|---------|-----------|---------|
| 0 | `engine/src/field_frame.rs:220-221` | 3 (u/u_prev/next_scratch) | double `mem::swap` | **reference** |
| 1 | `kernel/src/csr.rs:355` | 2 (pi/next) | `mem::swap` | **exact 2-buffer ping-pong** |
| 2 | `bebop2 core/src/chebyshev.rs:162-163` | 3 (t_prev/t_cur/t_next) | double `mem::swap` | **exact — structural twin of #0** |
| 3 | `bebop2 core/src/field.rs:261, :880` | 2 (u/unext) | `mem::swap` | **exact 2-buffer ping-pong** |

### 1.1 `kernel/src/csr.rs:355` — personalized-PageRank Jacobi power iteration
`Csr::personalized_pagerank` (`csr.rs:330-366`). Two owned vectors `pi` and `next`
(`csr.rs:346-347`); each iteration writes the whole vector into `next` via
`spmv` + restart blend (`csr.rs:351-353`), then `std::mem::swap(&mut pi, &mut next)`
(`csr.rs:355`). This is the **cleanest textbook instance**: a pure 2-buffer
Jacobi ping-pong. The doc-comment even names *why* it is double-buffered
("the WHOLE vector is updated from the previous iterate … no in-place Gauss-Seidel
mixing", `csr.rs:348-349`) — i.e. correctness *requires* the shadow buffer, it is
not just an allocation optimization. **Genuine, same idiom.** ✅

### 1.2 `bebop2 core/src/chebyshev.rs:162-163` — Chebyshev polynomial recurrence
The three-term Chebyshev recurrence `Tₖ = 2·(…)·T_{k-1} − T_{k-2}`
(`chebyshev.rs:154-164`) computes `t_next` from `t_cur`/`t_prev`, then rotates all
three with **two swaps**:
```rust
core::mem::swap(&mut t_prev, &mut t_cur);
core::mem::swap(&mut t_cur, &mut t_next);
```
This is the **structural twin of `field_frame::step`** — same 3-buffer, same
double-swap rotation, same "keep k-2 and k-1, produce k, recycle" shape. The only
difference is the *math* being iterated (Chebyshev matrix polynomial vs. semi-implicit
wave PDE). **Genuine, same idiom — and the strongest evidence the pattern recurs
independently.** ✅

### 1.3 `bebop2 core/src/field.rs:261` (and `:880`) — wave-field integrator
`core::mem::swap(&mut u, &mut unext)` (`field.rs:261`, again at `:880`). This is the
bebop2 sibling of the engine's `field_frame` — a 2-buffer wave/diffusion field step:
compute `unext` from `u` (+ an activity mask), then swap. `u` is returned via
`u.clone()` at the end (`field.rs:265`). **Genuine, same idiom** (2-buffer form). ✅

> Note on locus: per project convention (MEMORY: "bebop/wire-native-core files →
> `/root/bebop-repo` NOT `/root/dowiz`") the live bebop2 **core** crate is at
> `/root/bebop-repo/bebop2/core/`; the in-repo `/root/dowiz/bebop2/` holds only
> `delivery-domain` (no `core`, zero `.rs` swap sites). The operator explicitly named
> bebop2 as a sweep target, so sites #2/#3 are counted.

---

## 2. Same *family*, different promotion mechanism (NOT a bare swap)

These are double-buffered iterations, but the promotion is a **fused copy** (it reads
the whole secondary buffer to produce the primary), so `mem::swap` is structurally
unavailable. Honest classification: *cousins that share the "compute-into-secondary"
half but not the "atomic pointer flip" half.*

### 2.1 `kernel/src/spectral.rs` `topk_symmetric` — deflated power method
Working buffers `x`, `ax`, `tmp` (`spectral.rs:281-283`). Each iteration does
`a.spmv(&x, &mut ax)` (compute into secondary, `spectral.rs:317`), deflates, then
**normalizes back into `x`**: `x[i] = ax[i] / nr` (`spectral.rs:337-339`). It cannot
be a swap because the norm `nr` must be computed from *all* of `ax` before the
write-back. Same double-buffer *intent*, promotion mechanism differs (copy-with-
normalize). The sibling arena power method at `spectral.rs:449-486` is the same shape.
**Related, not identical.** ◑

### 2.2 Functional-advance variants (compute a fresh *owned* next, move to current)
Same conceptual "advance state = f(state)", but a **fresh `Vec` is allocated each step**
and moved into place — no buffer reuse, no swap:

- `kernel/src/noether.rs` — invariant / Lyapunov drift checkers: `let x_next =
  update(&x); … x = x_next;` at `noether.rs:36-45`, `60-67`, `96-105`.
- `engine/src/field_energy.rs:357-367` — Lyapunov energy-descent checker: `let x_next
  = broken(&x); … x = x_next;`.

These are the *degenerate* form of the idiom (allocate-per-step instead of
ping-pong). They are generic verification harnesses, not hot render/compute loops, so
the allocation is deliberate and fine. **Same intent, not the buffer-reuse idiom.** ◑

---

## 3. Checked and honestly REJECTED (superficially similar, structurally different)

The operator's candidate list included several sites that turn out **not** to be this
idiom. Recording them so the negative result is explicit and not re-litigated.

### 3.1 `kernel/src/event_log.rs:319` — durable-insert-then-`set_tip`
`append()` (`event_log.rs:302-321`): bind `prev` to the current tip, `insert(id, ev)?`,
then `set_tip(id)` (`event_log.rs:318-319`). The `?` short-circuits **before** the tip
advance, so a rejected durability barrier leaves the tip untouched (`event_log.rs:316-317`).
This shares the **atomic-promote + fail-closed** property, but it is a **log-structured
moving-head pointer**, not an A/B double-buffer: there is no second reusable buffer, no
swap, and the old state (`prev`) is *retained in the chain*, not recycled as scratch.
**Cousin in the "commit-then-promote" family; NOT the ping-pong idiom.** ✗

### 3.2 `kernel/src/capability_cert.rs` `RotationState::Overlapping` — SSH-style overlap rotation
`capability_cert.rs:538-572`. During the migration window **both** old and new suites
are simultaneously accepted (`accepts()`, `capability_cert.rs:564-568`), until
`overlap_until`; then only the new. This is a **make-before-break / temporal-overlap
handover** — which is the *opposite* of an atomic instantaneous flip: the whole point
is that two states are valid *at once* for a grace window (`OVERLAP_WINDOW_TICKS`,
`capability_cert.rs:759-760`) so no verifier strands. Same broad "state transition"
umbrella, but structurally distinct from compute-into-shadow-then-atomic-swap.
**NOT the idiom** (and the "overlap-rotation" naming elsewhere refers to *this*, not to
ping-pong). ✗

### 3.3 `engine/src/bridge.rs` `VertexBridge` — staging mirror
`scratch` (live SoA vertex buffer) + `staging` (host mirror), `bridge.rs:69-95`.
`upload_once()` does `staging.clear(); staging.extend_from_slice(view)`
(`bridge.rs:164-165`) — a **one-directional mirror copy** modelling the GPU upload.
`staging` never swaps back and never becomes the source `scratch` reads from next
frame. Double *buffer* by name, but not double-*buffered iteration*. **NOT the idiom.** ✗

### 3.4 P92 mesh fast-path — session-key rotation (bonus context)
`docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md`.
Session rotation is a **full re-handshake minting a fresh `epoch` + fresh ephemeral
ML-KEM** on expiry (`§4.5`, blueprint L411-418), i.e. bounded-lifetime + rebuild — not
an in-place buffer flip. The blueprint **explicitly rejects** the Signal double-ratchet
/ continuous-rekey pattern as over-engineering (L137: "NOT taken … Over-engineering …
Bounded-lifetime session + full re-handshake on expiry is sufficient and far simpler").
Revocation is a merge/recheck, not a swap. **NOT the idiom** — recorded because the
operator asked. ✗

---

## 4. Honest tally

- **Exact idiom (atomic-swap promotion, buffer reuse):** **4 sites** across **5 code
  locations** — `field_frame.rs` (ref), `csr.rs` PageRank, `chebyshev.rs`, `field.rs`
  (×2). They split into two shapes:
  - **2-buffer ping-pong:** csr PageRank (`pi↔next`), bebop2 field (`u↔unext`).
  - **3-buffer double-swap rotation:** field_frame (`u_prev/u/next_scratch`),
    chebyshev (`t_prev/t_cur/t_next`).
- **Same family, non-swap promotion:** **1** (spectral power method) + **2**
  functional-advance harnesses (noether, field_energy).
- **Checked & rejected:** **4** (event_log tip, capability_cert overlap, bridge mirror,
  P92 rotation).

So the operator's instinct is **confirmed and stronger than one site**: the exact
compute-into-shadow-then-swap idiom recurs at **4 genuinely independent places, in 3
different crates (engine, kernel, bebop2 core), authored around different math** — that
is real convergent evidence, not force-fit.

---

## 5. Should it be extracted into a shared abstraction? — verdict: **NO (leave as convention)**

Weighed honestly against the project's ponytail/YAGNI default ("the best code is the
code never written") **and** the standing performance-priority directive (which would
license a real change *if it compounded*):

**Arguments for a `PingPong<T>` / `DoubleBuffer<T>` type**
- 4 independent sites is past the "rule of three".
- The 3-buffer double-swap (field_frame, chebyshev) is genuinely easy to mis-order
  (which buffer becomes the next scratch); a named `rotate(next)` could encode that
  invariant once.

**Arguments against (decisive)**
1. **`std::mem::swap` is already the minimum-viable, zero-cost, totally legible
   primitive.** A `PingPong<T>` wrapper *adds* lines + a `.rotate()`/`.swap()`
   indirection to replace a one-line stdlib call — the exact over-abstraction ponytail
   forbids. Net line count would go **up**, not down.
2. **No single shape fits.** Two sites are 2-buffer, two are 3-buffer, and the two
   3-buffer sites rotate for *different reasons* (field_frame keeps `u_prev` for a
   backward difference **and** recycles it as scratch; chebyshev holds `T_{k-2},
   T_{k-1}`). A generic type covering both collapses into a config-heavy `Ring<N, T>`
   whose call sites are *less* readable than the explicit swaps they replace.
3. **Crate-boundary cost is real.** The sites live in `engine`, `kernel`, and
   `bebop-repo/bebop2/core` — three crates with **no shared low-level dependency edge**
   for this. A shared type forces a new common-crate dependency across the
   kernel/engine/bebop boundary purely to host a 2-line helper. That architectural
   coupling is far more expensive than the duplication it removes.
4. **Every site is already individually tested** (field_frame byte-identity KAT
   `field_frame.rs:459-493`; csr/spectral/chebyshev each have their own iteration
   tests), so there is no correctness debt an abstraction would pay down.

**Recommendation (cheap, non-code-mandating):** keep the idiom as an **explicit,
greppable convention**, which the codebase already half-follows:
- the `_prev` / `_next` / `_scratch` buffer-naming (`field_frame.rs:161-165`,
  `csr.rs:346-347`, `chebyshev.rs:142-153`),
- a short `// ping-pong rotation` / `// double-buffer swap` marker comment at each swap
  so the pattern is discoverable by `grep -rn "mem::swap"`.

That gives the recognizability benefit of an abstraction at zero abstraction cost. If a
future site count crosses ~6–8 **within a single crate** (removing objection #3), revisit
a *crate-local, 2-buffer-only* `PingPong<T>` — never a cross-crate generic ring. Until
then: **implicit convention wins.**

---

## Appendix — files read (fresh, this pass)

- `engine/src/field_frame.rs` (full), `engine/src/bridge.rs:60-189`,
  `engine/src/field_energy.rs` (grep)
- `kernel/src/csr.rs:300-374`, `kernel/src/spectral.rs:264-363` (+ grep),
  `kernel/src/event_log.rs:299-338`, `kernel/src/capability_cert.rs:538-577` (+ grep),
  `kernel/src/noether.rs` (grep)
- `/root/bebop-repo/bebop2/core/src/chebyshev.rs:138-166`,
  `/root/bebop-repo/bebop2/core/src/field.rs:248-265`
- `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md` (grep)
- repo-wide grep: `mem::swap|mem::replace|mem::take|ping-pong|double-buffer|shadow|_prev|_next|scratch|staging|rotat|overlap`
