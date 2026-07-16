# Principle 4 — POLARITY (Kybalion) grounded as a dowiz/openbebop architecture principle

> "Everything is dual; everything has two poles; opposites are identical in nature, different only in
> degree." Heat and cold are one phenomenon read at two ends of one thermometer, not two unrelated things.
> This pass grounds that as a concrete, testable software-architecture rule and audits the *local*
> codebase (`/root/dowiz/kernel`, `/root/dowiz/engine`, `/root/dowiz/web`, `/root/bebop-repo/bebop2`)
> for compliance and violations. No mysticism — every claim below cites real code (file:line) or a real
> architecture doc.

---

## 1. The architecture-principle statement (concrete, falsifiable)

**POLARITY (architecture form).** A binary architectural state — allow/deny, fail-closed/fail-open,
Damped/Unstable, Live/dead, durable/lost — MUST be represented as **one value or one mechanism carrying
two named poles**, never as two independently-maintained code paths that happen to encode opposite
outcomes. Three corollaries make it checkable:

1. **One decision function, two outcomes.** The "allow" answer and the "deny" answer must exit the *same*
   function through the *same* type (`Result<T, DenyReason>` / a two-arm `match`), so they cannot drift
   out of sync. Two separate `fn allow(...)` and `fn deny(...)` bodies are a violation even when they look
   correct today.
2. **Every degree is a named point on the single spectrum.** Intermediate states are not a third code
   path; they are an explicit variant on the one axis (the `Resonant` middle of `ρ`, the enumerated
   `CapError` deny reasons). An *unrepresented* intermediate ("revoked-but-not-expired" with no `Revoked`
   variant) is the classic polarity hole.
3. **A collapse of one pole into the other must be safe-directed and visible.** When two poles are forced
   to render the same surface value (an empty stream, a `0`, a `null`), the collapse must go *toward* the
   safe pole and be *typed*, so "no data" is never silently indistinguishable from "denied / failed."
   The dangerous shape is a fail-open that renders identical to a well-behaved fail-closed.

This is exactly how the canonical architecture already talks: `ARCHITECTURE.md:43` — "**Errors (S5):
fail-closed Result always**"; `AGENTS.md:191` — "**Falsifiable done-checks, not vibes** … a real command,
test name" (RED and GREEN are two poles of one test, not two tests). The principle below is the
generalization of S5 to *every* binary in the system.

---

## 2. Verification of the hypothesis instances against real code

### 2a. `DriftClass` — the textbook one-spectrum/two-poles case (CONFIRMED)

`kernel/src/spectral.rs:325-335` is the cleanest instance in the repo:

```rust
pub fn classify_drift(a: &[Vec<f64>]) -> DriftClass {
    let rho = spectral_radius(a);
    const BAND: f64 = 1e-6;
    if rho < 1.0 - BAND { DriftClass::Damped }        // pole A: contracts
    else if rho > 1.0 + BAND { DriftClass::Unstable } // pole B: diverges
    else { DriftClass::Resonant }                     // the middle degree
}
```

One continuous physical quantity — the spectral radius `ρ` = max|λ| (`spectral.rs:216`) — read against a
single reference (the unit circle) yields two poles (`Damped` ρ<1, `Unstable` ρ>1) with a well-defined
middle `Resonant` (ρ≈1) point (`spectral.rs:316-323`). This is heat-and-cold verbatim: "Damped" and
"Unstable" are not two mechanisms, they are two readings of one number. The `BAND` (1e-6) is the only
tuning, and it is symmetric about 1.0. Tests pin all three points against the same axis
(`spectral.rs:523-528`: ρ=0.5→Damped, ρ=2→Unstable, ρ=1 2-cycle→Resonant). **Verdict: exemplary polarity.**

### 2b. The decide/deny path routes through one function (CONFIRMED)

`kernel/src/order_machine.rs:123-135` — `assert_transition` is a single `Result<(), TransitionError>`:
allow exits `Ok(())`, deny exits `Err(...)` with a *named* reason (`SameStatus` / `ScaffoldDisabled` /
`Illegal`). There is no separate "allow" routine anywhere; `fold_transitions` (`order_machine.rs:140-153`)
replays through the same function. The guard uses the correct explicit-deny idiom
(`order_machine.rs:131`: `if !allowed.contains(&to) { return Err(...) }`) — an explicit deny pole, not a
silent `else {}`.

The bebop authorization gate is the same shape at higher stakes: `HybridGate::check`
(`bebop2/proto-cap/src/hybrid_gate.rs:129-208`) is ONE function returning `Result<(), CapError>`, and the
deny pole is *fully enumerated* — `error.rs` lists 14 named deny reasons (`Expired`, `NonceRejected`,
`UnknownIssuer`, `ScopeViolation`, `Revoked`, `RedLineViolation`, …), each a distinct point on the one
"why-rejected" axis. The red-line money/secrets/migrations deny is a *variant on that same axis*
(`hybrid_gate.rs:150-154` → `CapError::RedLineViolation`), not a parallel gate. `bebop2/mesh-node/src/dod.rs:58-73`
(`admit → Result<(), DodFault>`) repeats the pattern: `Ok` records the event, `Err` records nothing and
the caller drops it — the two poles are inseparable in one function. **Verdict: hypothesis confirmed; the
kernel/proto-cap trust path is single-mechanism polarity throughout.**

### 2c. M9 kill-switch / `OrganismState` — one two-pole state machine (CONFIRMED, with nuance)

`ARCHITECTURE.md:18` (M9) defines kill-switch as the only global control. In code the alive/dead axis is
`kernel/src/hydra.rs:76-78` — `enum OrganismState { Live, Locked }` — a single two-variant type, flipped by
one function `integrity_check` (`hydra.rs:180-196`) that re-derives the baseline spectrum and sets `Locked`
iff foreign code shifted `ρ` (fail-closed). There is no ambiguous third runtime state: `commit`
(`hydra.rs:220-227`) refuses while `Locked` regardless of any other flag. The nuance that *strengthens*
the finding: the drift gate exposes a deliberate second pole via `intervention: bool`
(`event_log.rs:347-369`) — default = gate ON (reject `Unstable`), `intervention == true` = ALL safeties
lifted (operator directive, `SOURCE-OF-HYDRA-2026-07-16.md:47-51`). Crucially this is **one code path with a
boolean pole selector**, and the `Locked`/tamper check is a *separate, non-bypassable* axis
(`hydra.rs:206-207`, "intervention ≠ tamper"). So the design is a clean product of two orthogonal single
mechanisms, not four divergent branches. **Verdict: confirmed — no ambiguous alive/dead state exists.**

### 2d. The `RevocationSet` "third-state" hypothesis — REFUTED against the live tree

The task carried a prior-session claim: RevocationSet is "specced M12, not built, only expiry checked," so
`revoked-but-not-expired` is an unrepresented degenerate third state. **This is stale/false for the actual
codebase.** `bebop2/proto-cap/src/revocation.rs` (245 lines) fully implements `RevocationSet` (append-only,
monotonic, `revoke_key` / `revoke_capability` / `merge` gossip), and it is *wired into the verification
path*: `hybrid_gate.rs:159-168` rejects a frame as `CapError::Revoked` when the subject key or the
capability's `revocation_hash` is in the set — "even when signature/chain/expiry are otherwise valid"
(`hybrid_gate.rs:116-118`). `error.rs` carries a distinct `Revoked` variant, separate from `Expired`. The
valid/revoked axis therefore has *two clean poles*, and revoked-but-unexpired is a first-class rejection,
not a hole. Note also that the dowiz kernel itself has **no** capability module (`ls kernel/src` shows no
`capability`/`revocation`); the entire model lives in bebop proto-cap, where it is complete. **Verdict:
hypothesis refuted — this is polarity done *right*; report it as a corrected premise, not a finding.**

### 2e. Positive collapse-direction examples (CONFIRMED safe)

Two places intentionally collapse two poles to one surface value, and both collapse *toward* the safe pole:
`wasm-host/src/lib.rs:111-123` returns an **empty** allow-set for both "explicitly denied" and "unknown
resource" — both map to the deny pole (zero ambient authority), and wasmtime then fails the link as a typed
`HostError::ScopeViolation` (`lib.rs:208-214`), so a denied import can never silently link. The engine's
`drift_from_code` (`engine/src/bridge.rs:673-678`) maps any unknown wire code via `_ => DriftClass::Unstable`
— an unrecognized frame renders as the *worst/alarming* pole, never as healthy `Damped`. The JS boundary
`decodeRet` (`web/src/lib/kernel/kernel_client.mjs:48-54`) turns the wasm multivalue into `{ok:false}` /
`{ok:true,value}` off a single `isErr` bit — one decode, two typed poles. **Verdict: correct safe-directed
collapses.**

---

## 3. Audit findings — polarity violations (severity-ranked)

### V1 — MONEY PATH — `estimate_order_total` swallows the tax error-pole → `0` — **MEDIUM**

`kernel/src/money.rs:218`:

```rust
let tax_total = apply_tax(subtotal, cfg.tax_rate, cfg.price_includes_tax).unwrap_or(0);
```

`apply_tax` returns `Result<i64, String>` whose `Err` pole is a real failure — tax overflow,
`money.rs:111`: `i64::try_from(tax).map_err(|_| "tax overflow…")`. `.unwrap_or(0)` **collapses that error
pole into the value `0`**, which is byte-identical to the legitimate `tax_rate == 0.0 → Ok(0)` case
(`money.rs:95-96`). A computation *failure* and a *tax-free order* now render the same surface number — the
exact "two opposite meanings, one surface" shape this session already flagged for the living-memory-viz
empty-stream. It is a *money* path, which is why it ranks above cosmetic issues.

The violation is sharpened by the code beside it. In the SAME struct build, delivery fee **preserves** its
unknown pole: `compute_delivery_fee` returns `Option<i64>` where `None` means "server-only / not
configured" (`money.rs:196-212`) and the caller keeps that pole explicit via `fee_known =
delivery_fee.is_some()` (`money.rs:223`). Tax gets no `tax_known`; its failure is silently absorbed. And the
*authoritative* total path does it correctly — `domain.rs:101`: `let tax = apply_tax(...)?;` **propagates**
the error pole (returns `Err` from `compute_order_total`). So one function, `apply_tax`, has two callers
with opposite polarity discipline: `domain.rs:101` (correct, `?`) vs `money.rs:218` (violation,
`unwrap_or(0)`). Severity is held at MEDIUM (not high) because `estimate_order_total` is an explicit
client-side *estimate* mirror (doc: "Mirror of money.ts `estimateOrderTotal`", server is authoritative) and
the `Err` trigger requires subtotal×rate near i64::MAX. **Fix: return `Result` or add a `tax_known` pole
mirroring `fee_known`; never `unwrap_or(0)` a money `Result`.**

### V2 — INTEGRITY / AUDIT SUBSTRATE — the durability port has **no failure pole** — **HIGH (in-path)**

`kernel/src/event_log.rs:162-166` defines the persistence trait:

```rust
pub trait EventStore {
    fn insert(&mut self, id: [u8; 32], ev: MeshEvent);   // returns () — infallible by type
    ...
}
```

`insert` returns `()`. There is *no* deny/failure pole in the type at all — the operation can only be
modeled as "success." The `FileEventStore` (the durable WORM upgrade, `hydra.rs:821-852`) then implements it
by **swallowing every IO Result** and proceeding regardless:

```rust
if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&self.path) {
    let _ = f.write_all(line.as_bytes());   // discarded
    let _ = f.flush();                      // discarded
    let _ = f.sync_all();                   // discarded (fsync!)
}
self.by_id.insert(id, ev);  // in-memory state advances EVEN IF the file failed to OPEN
self.tip = Some(id);
```

If the file fails to open, or `write_all`/`sync_all` fails (disk full, IO error), the disk write is skipped
but the in-memory map, tip, and count still advance — and the caller up the stack receives
`AppendOutcome::Committed` (`event_log.rs:272,317`). A *durability failure is indistinguishable from a
durable commit*: a fail-open masquerading as the "committed" pole, one level deeper than the money case.
This is load-bearing: the WORM log is the anti-tamper substrate — the breach-witness rows
(`SOURCE-OF-HYDRA-2026-07-16.md:108-111`, "a tampered core can never silently heal") and `boot_verify`
(replay ρ<1 after restart, G5) both assume `insert` reached disk. The polarity flaw is architectural: the
port is a **single-pole (infallible) type**, so a persistence failure has nowhere to surface. Severity HIGH
because it sits in the integrity/audit path; trigger probability is moderate (needs an IO fault). **Fix:
`fn insert(...) -> Result<(), StoreError>` and propagate; a WORM store that cannot report a lost write is
not append-only in any provable sense.**

### V3 — RENDER/COSMETIC — the `DriftClass` spectrum is defined in **two** places — **LOW**

The one three-pole spectrum from §2a is re-declared: `engine/src/bridge.rs:648-663` defines a *second*
`enum DriftClass { Damped, Resonant, Unstable }` that only a comment binds to the authority
(`bridge.rs:648`: "mirrors `dowiz-kernel::spectral::DriftClass`"). The two definitions are kept in sync by a
hand-maintained numeric wire contract (`Damped=0, Resonant=1, Unstable=2`, `bridge.rs:636`,
`drift_from_code` `bridge.rs:673-678`). This is the milder form of the anti-pattern — *one spectrum, two
independently-maintained representations* — that can drift if either enum is reordered or renamed. It is
mitigated (the flat-code contract is tested, `bridge.rs:781-783`, and the `_ => Unstable` collapse is
safe-directed per §2e), and it lives in a pure renderer, so severity is LOW. It is included because it is
the exact "two code paths for one axis" shape the principle warns against, caught early and cheaply. **Fix
(optional): share one enum across kernel/engine, or generate the engine copy from the kernel discriminants.**

### Negative result worth recording

A scan for the pure implicit-deny shape (`if authorized { … } else { /* nothing */ }`) across
`kernel/src`, `engine/src`, `web/src` found **none** — the Rust code consistently uses guard-clause
explicit-deny (`return Err(...)`). The dangerous silent-`else` is essentially absent from the kernel; the
real violations are pole-*collapses* (V1, V2), not missing deny-branches.

---

## 4. Verdict

Polarity is, for the most part, a principle this codebase already *lives*: the drift spectrum
(`classify_drift`), the transition Law (`assert_transition`), the whole proto-cap authorization gate
(`HybridGate::check` with a 14-variant enumerated deny axis, revocation fully wired), the M9/`OrganismState`
alive/dead machine, and the JS `decodeRet` boundary are all single-mechanism, two-pole designs, and the two
intentional pole-collapses (wasm deny-set, unknown drift-code) are safe-directed. The `RevocationSet`
"unrepresented third state" hypothesis is **refuted** — valid/revoked is a real second pole
(`CapError::Revoked ≠ Expired`), built and wired in bebop.

The genuine violations are pole-*collapses that hide a fail state behind a well-behaved surface value*, and
they cluster in exactly the places the principle predicts are dangerous: a **money path** (V1,
`money.rs:218`, tax `Err → 0`, MEDIUM) and the **integrity/audit substrate** (V2, `EventStore::insert → ()`
+ `hydra.rs:845-847` swallowed fsync, HIGH-in-path). Both are the same failure family this session already
named once (fail-open masquerading as fail-closed); both are fixable by giving the collapsed pole a type
(`Result` / a `*_known` flag). V3 (duplicated `DriftClass` enum, LOW) is the cheap structural version of
the same warning. Net: the architecture's polarity discipline is strong at the trust boundary and leaks at
two value-encoding seams — fix V2 first (it is the integrity floor), V1 second, V3 opportunistically.
