# GATE-ROOT INVARIANT — Item 73 (BLUEPRINT-ITEMS-73-78 §2, 2026-07-19)

**Status:** SPEC + CI-GREP level. Built against item 74's merged registry
(`docs/audits/governance/RED-LINE-REGISTRY.tsv` + `scripts/red-line-classifier.sh`
+ `scripts/red-line-monotonicity.sh`).

**Scope of THIS commit:** clauses **(ii)** (dependency-direction) and **(iii)**
(step-zero refusal) are built and enforced at spec + CI-grep level. Clause **(i)**
(root placement behind the composition root / sole minter) is **DEFERRED** to
items 64/65 — see [§7](#7-clause-i-deferred-to-items-6465-do-not-fake-a-minter).
No minter, no capability-minting code, and no Rust kernel code are introduced here.

---

## 1. The binding meta-capability negative law

> **No `pub` type, `fn`, or `macro` anywhere in the crate may construct a value
> that confers write authority over the registry, the gate code, or the
> composition root.**

This is the generalized form of the item-74 `no-courier-scoring` /
"GENERATED — do not hand-edit" discipline, lifted one plane up — from
*authorizing an agent's runtime action* to *authorizing a mutation of the gate
itself*. It is the negative-by-construction twin of seL4's "init task holds all
capabilities": the root delegates everything **except** authority over itself.

The law is **recorded, not implied** — it lives in two places:
- the registry module doc / this spec doc (human-readable binding), and
- item 74's `RED-LINE-REGISTRY.tsv` `governance-self` self-row (machine-readable twin).

---

## 2. Clause (ii) — dependency-direction check (item-45 style)

A CI leg (`scripts/gate-root-invariant.sh`) asserts, pointed at the
proposal-pipeline path-prefix, that **no module reachable from the proposal
pipeline references any registry/gate mutation-surface identifier**.

The check's **GREEN state is an EMPTY grep set**, because:

- item 74's registry is `static`/`const` data + pure classifier functions;
- it exposes **zero** `pub fn` taking `&mut self`,
- **zero** `pub static mut`,
- **zero** interior-mutability handle.

You cannot mint a capability to mutate a thing that has no mutation API. The
grep set over the registry/gate mutation surface is therefore provably empty on
a clean tree — the GREEN case — and the gate reuses item-45's exact
"core never imports X" idiom (`scripts/ai-optional-gate.sh` part B), pointed
pipeline→gate instead of core→AI.

Part (b) of the same leg asserts the proposal-pipeline modules do **not** `use`
a composition-root minter. No minter exists yet (§7), so the violated-set is
empty → GREEN; the check is written and ready to go RED the instant items 64/65
land a minter that the pipeline must never reach.

### Proof shape (RED→GREEN)
- **GREEN:** on a clean tree, the dependency-direction grep returns nothing.
- **RED:** a planted `use registry::internal_mutate;` (or any reference to the
  gate/registry mutation surface) inside a proposal-pipeline module turns the
  check RED. Removing the planted line restores GREEN.
  The gate script demonstrates this with a temp file it creates and removes
  (see `scripts/gate-root-invariant.sh` — `DEPENDENCY-RED-PATH` section).

---

## 3. Clause (iii) — step-zero classification refusal (out-of-band-only)

Changing the gate / registry / verification logic happens ONLY through the
normal human engineering process (a human editing code, review, the full CI
suite) — **never** through the proposal pipeline, even "with extra approvals".

A proposal whose diff touches a red-line path is **REFUSED at classification
time (step zero)** — a category error, not a reviewable request — by item 75's
`admit` calling item 74's `red-line-classifier.sh` **first**, *before any
verification runs and before any human is asked*. The classifier returns
`RejectionClass::Refuted{ red_line: <named cause> }`.

**No "beneficial change to the gate" exception exists** — that exception *is*
the corrigibility trap.

### D11 Q4 — core-authority rows = out-of-band-only (the HARD line)
Encoded in `RED-LINE-REGISTRY.tsv`. The `removal_authority=out-of-band-only`
rows are precisely the core-authority boundary the operator ratified
(2026-07-20):
- `kernel/src/breaker/` — circuit breaker (item 9, the HARD non-negotiable line)
- `kernel/src/order_machine.rs` — order FSM core
- `kernel/src/decision/mod.rs` — the decide half of the order/money/decide-fold
- `kernel/src/money.rs` — money core

Any touched path under an `out-of-band-only` row makes the classifier exit
**non-zero** at classification time. No AI proposal, however it entered, can
modify these.

### D11 Q5 — removal needs a DECISIONS.md D-entry marker
Removing a row is the dangerous direction (the corrigibility-adjacent attack:
quietly drop a protected path so a later proposal can touch it). Enforced by
`scripts/red-line-monotonicity.sh` (item 74): a removed row is permitted ONLY
if the diff/deletion carries a `DECISIONS.md` D-entry marker (e.g. `D12:`).
Without it → RED. Adding a row is always safe (widens protection) → GREEN.

### Proof shape (RED→GREEN)
- **GREEN (normal):** `red-line-classifier.sh --paths docs/design/foo.md` → exit 0.
- **RED (step-zero refusal):** a planted proposal diff touching a core red-line
  path (e.g. `kernel/src/money.rs`) is REFUSED at classification time with a
  typed `Refuted` cause and exit code 1, **before** any verification is entered.
  Demonstrated by `gate-root-invariant.sh` calling the classifier on a planted
  path set (see `STEP-ZERO-REFUSAL` section).

---

## 4. The registry carries NO mutation surface (clause ii's enabler)

Grep proof (also re-run by `gate-root-invariant.sh`, section `MUTATION-SURFACE`):
zero `pub fn.*&mut` / `pub static mut` / interior-mutability handle in the
registry-adjacent code (item 74's `red-line-classifier.sh` and
`red-line-monotonicity.sh`). The registry is a `.tsv` + pure-bash — there is no
runtime mutation API at all. The only way to change it is a human editing the
file through the normal CI suite (out-of-band-only / operator-ruling).

Planted `pub fn internal_mutate(&mut self)` → RED (the grep would catch it).

---

## 5. The recursion is recorded

Item 74's registry carries **§L's own gate code + the registry itself as rows**
(blueprint §2.3 step 5 / §2.5; blueprint §3.3(8)). The `governance-self` class
rows in `RED-LINE-REGISTRY.tsv`:

- `docs/audits/governance/RED-LINE-REGISTRY.tsv` — the registry ITSELF
  (self-inclusion).
- `scripts/red-line-classifier.sh` — item 75's step-zero classifier.
- `scripts/red-line-monotonicity.sh` — the monotonicity guard.

The out-of-band-only law is recorded here (this doc) **and** in the registry
module doc (the comment block at the top of `RED-LINE-REGISTRY.tsv`). The
recursion is *recorded, not implied*.

**Cross-link — item 74's self-row test:**
`scripts/red-line-classifier.sh --self-row` asserts the registry's own path is
a registered row. It is wired into `scripts/verify-item-74.sh` (step D) and
re-asserted here in `gate-root-invariant.sh` (section `SELF-ROW`).

---

## 6. Acceptance criteria (§2.5) — demonstrably met

| Criterion (blueprint §2.5) | How this commit proves it |
|---|---|
| No in-tree type/fn/macro constructs write authority over registry/gate/root (grep + compile-fail) | `MUTATION-SURFACE` grep over item-74 classifier/monotonicity scripts: zero `pub fn.*&mut` / `pub static mut` / interior-mutability. (Compile-fail leg is deferred with clause (i) to 64/65 — no type system lands here.) |
| Dependency-direction check GREEN with empty violation set + RED on planted reference | `gate-root-invariant.sh` DEPENDENCY-RED-PATH section: clean-tree grep EMPTY (GREEN), planted reference → RED, removal → GREEN. |
| Every red-line-class planted proposal refused at step zero with distinct typed cause + FDR record | `gate-root-invariant.sh` STEP-ZERO-REFUSAL section calls item-74 classifier on one planted path per class (money/auth/registry/gate/verification) → each REFUSED (exit 1) before verification. |
| Out-of-band-only law recorded in CHECKLIST.md + registry module doc; item 74 self-rows verified | This doc + registry module doc + `SELF-ROW` re-assertion in the gate script. |

---

## 7. Clause (i) deferred to items 64/65 — DO NOT fake a minter

Clause (i) requires the registry + gate-enforcement code to live at/behind
item 64's composition root — the **sole capability minter** — such that *no
capability type granting write access to the root, the registry, or the gate
code EXISTS in the type system* (unconstructible, not merely unhanded-out).

**The composition root / minter does NOT exist yet** (blueprint §1.2, §2.6).
Building clause (i)'s structural landing needs items 64 (composition root) and
65 (capability boundary), both **unbuilt**. **This commit deliberately does NOT
create a composition root, a capability minter, or any write-authority type.**
Clause (i) is recorded as a deferred obligation; its compile-fail proof
(`kernel/tests/compile_fail/gate_root_no_write_cap.rs`) is deferred to the
64/65 landing. Faking a minter here would violate the blueprint's explicit
"do not fake a minter to land clause (i) early" instruction.

---

## 8. Files in this commit (spec + scripts only; zero Rust kernel code)

- `docs/audits/governance/GATE-ROOT-INVARIANT.md` — this spec doc.
- `scripts/gate-root-invariant.sh` — CI leg (clauses ii/iii) + RED→GREEN proofs,
  pointed at item 74's registry + classifier. Does not modify item-74's files.

**Not touched:** item 74's `RED-LINE-REGISTRY.tsv`, `red-line-classifier.sh`,
`red-line-monotonicity.sh` (merged, used as-is). No kernel crate changes, no
new cargo dependency.
