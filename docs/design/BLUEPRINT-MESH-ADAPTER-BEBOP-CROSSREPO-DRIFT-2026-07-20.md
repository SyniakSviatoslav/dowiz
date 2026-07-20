# BLUEPRINT — mesh-adapter / bebop-repo cross-repo type drift reconciliation (2026-07-20)

- **Date:** 2026-07-20 · **Component:** PROTOCOL (P34/P34B) · **Status:** BLUEPRINT v1 (planning
  artifact, no code changed). **⚠ OPERATOR SIGN-OFF NEEDED before implementation** — §3 presents
  two real remediation paths with different tradeoffs; this blueprint specifies both completely
  enough to execute either, but does not pick one, per explicit instruction.
- **Sources verified this session, live build attempted (not guessed):** `mesh-adapter/Cargo.toml`
  (the P34 dependency edge: `dowiz-kernel` path dep + `bebop-delivery-domain`/`bebop-proto-cap`
  path deps into `../../bebop-repo/bebop2/`); `/root/bebop-repo/bebop2/delivery-domain/Cargo.toml`
  (`dowiz-kernel = { path = "../../../dowiz/kernel", ... }` — confirms the sibling-directory
  layout convention: `dowiz/` and `bebop-repo/` must sit next to each other); `kernel/src/domain.rs`
  (`OrderItem`, live); `kernel/src/order_machine.rs` (`OrderStatus`, live); a real
  `cargo check --features kernel-rlib` run against `bebop-delivery-domain` (from
  `/root/bebop-repo/bebop2/delivery-domain`, sibling-linked to this session's worktree as the
  current `dowiz-kernel`) — the compiler errors quoted in §1 are copy-pasted from that run, not
  reconstructed from memory.

---

## 1. Problem statement — the real compiler errors

`mesh-adapter/` (this repo) wires `dowiz-kernel` as the consumer/driver of the `bebop2/` delivery
protocol via two path dependencies into the companion `bebop-repo` (`bebop-delivery-domain`,
`bebop-proto-cap`). Building `bebop-delivery-domain` with `--features kernel-rlib` (the feature
that links the real kernel rather than staying in its dependency-free default mode) against the
CURRENT kernel produces:

```
error[E0063]: missing fields `currency` and `vendor_id` in initializer of `OrderItem`
   --> bebop2/delivery-domain/src/intake.rs:270:14
    |
270 |         vec![OrderItem {
    |              ^^^^^^^^^ missing `currency` and `vendor_id`

error[E0004]: non-exhaustive patterns: `dowiz_kernel::OrderStatus::Refunding` and
              `dowiz_kernel::OrderStatus::CompensatedRefund` not covered
  --> bebop2/delivery-domain/src/intake.rs:70:11
   |
70 |     match s {
   |           ^ patterns `Refunding` and `CompensatedRefund` not covered
```

Grounded root cause: `dowiz-kernel`'s `OrderItem` (`kernel/src/domain.rs:31-46`) gained two fields
since bebop-repo's `intake.rs`/`lib.rs` were last synced — `vendor_id: VendorId` (P62 M4,
multi-vendor food-court checkout) and `currency: Currency` (same phase). `dowiz-kernel`'s
`OrderStatus` (`kernel/src/order_machine.rs:8-25`) gained two variants — `Refunding` and
`CompensatedRefund` (P07 compensation/refund states). bebop-repo's delivery-domain crate was
written against an earlier kernel shape and has not been updated. This is a real, live, confirmed
compile failure — `bebop-delivery-domain` with `kernel-rlib` **does not build today** against the
current `dowiz-kernel`.

**Full blast radius (grepped this session, not assumed):** exactly 4 `OrderItem { ... }` struct
literals are missing the two new fields —
`bebop2/delivery-domain/src/intake.rs:270`, and `bebop2/delivery-domain/src/lib.rs:454`, `:485`,
`:525` (all in `#[cfg(test)]` fixture-building code, same shape). Exactly 1 non-exhaustive match on
`OrderStatus` needs 2 new arms — `intake.rs:70`'s `from_order_status` (kernel status → wire
`DeliveryStatus`). The reverse map at `intake.rs:55` (`to_order_status`, wire → kernel) does NOT
need new arms — it only ever *produces* kernel statuses that already existed in the wire vocabulary
(`DeliveryStatus` has no refund-adjacent variant to map from), so it is unaffected. This is a small,
fully enumerated, mechanical fix — the actual code change is trivial; the hard part is that it
lives in a different git repository.

## 2. Why this is genuinely cross-repo (not solvable inside dowiz alone)

Per `CLAUDE.md`, `mesh-adapter/` "wires the kernel as the consumer/driver of the bebop2/ delivery
protocol"; the protocol crypto core lives in the companion OpenBebop repo, injected at a seam. The
2 failing files (`intake.rs`, `lib.rs`) are **source files owned by `bebop-repo`**, a separate git
repository with its own history, its own maintainers' workflow, and (per user memory) its own
`origin`/`openbebop` remote conventions. Rust's compilation model gives no way to patch another
crate's source without either (a) editing that crate's actual files (requires a change landing in
that repo), or (b) substituting a different copy of that crate entirely (a vendored fork, still
carrying the same source-level edit, just applied to a copy dowiz owns). There is no third option
that avoids touching *some* copy of `bebop-delivery-domain`'s source.

## 3. Two remediation paths — both fully specified, operator picks

### Path (a) — PR against `bebop-repo` (the canonical fix; converges the two repos)

**Exact diff, mechanical, no design ambiguity left for the implementer:**

```diff
# bebop2/delivery-domain/src/intake.rs — 4 OrderItem literals (intake.rs:270, and the 3 in lib.rs)
  vec![OrderItem {
      product_id: "sku-p13".to_string(),
      modifier_ids: vec![],
      quantity: 1,
      unit_price: 1000,
+     vendor_id: dowiz_kernel::VendorId(0),   // matches domain.rs's own documented legacy default
+     currency: dowiz_kernel::Currency::Eur,  // matches P62 M4's "Wave-0 single currency (EUR)"
  }],

# bebop2/delivery-domain/src/intake.rs:70, from_order_status — 2 new arms
  match s {
      ...
      OrderStatus::Scheduled => None,
+     // Refund lifecycle has no wire-protocol representation yet (same disposition as
+     // `Scheduled` above — not every kernel-internal state is wire-representable).
+     // Revisit once/if a mesh-side refund-notification wire message is designed.
+     OrderStatus::Refunding | OrderStatus::CompensatedRefund => None,
  }
```

**Verify exact field names/paths** (`VendorId`, `Currency::Eur`'s import path) against
`kernel/src/vendor.rs`/`kernel/src/money.rs` re-exports at implementation time — this blueprint
cites the live definitions (`kernel/src/vendor.rs:20`, `kernel/src/money.rs:29-35`) but not their
full re-export path from `bebop-delivery-domain`'s point of view, since that depends on
`dowiz_kernel`'s public module tree as seen from an external crate, not re-verified here.

**Tradeoffs:**
- ✅ Converges the two repos — no drift accumulates, no duplicate source of truth.
- ✅ The fix is genuinely tiny (6 struct-literal edits + 1 match arm) — low risk, low review burden.
- ❌ Requires a PR + merge in a repository this session/task has no write access to and no standing
  authorization to modify (per user memory: "BEBOP `origin` REMOTE ARCHIVED... Live remote =
  `openbebop`" — push destination + review process are bebop-repo's own, not dowiz's).
- ❌ Blocks on bebop-repo maintainer availability/timing, outside dowiz's control.

### Path (b) — vendored local fork inside `dowiz` (unblocks immediately, dowiz-side-only)

Add a vendored copy of just the 2 affected files under
`mesh-adapter/vendor/bebop-delivery-domain-patch/` (a full local fork of the
`bebop-delivery-domain` crate, Cargo-buildable standalone, carrying **the identical diff from
Path (a)** applied locally instead of upstream). `mesh-adapter/Cargo.toml`'s `bebop-delivery-domain`
path dependency is repointed from `../../bebop-repo/bebop2/delivery-domain` to the vendored copy.

**Explicit, load-bearing constraint if this path is chosen:** the vendored fork is a **temporary
patch, not a permanent fork**. It must carry a `VENDORED-PATCH.md` header stating (1) the exact
upstream commit it was forked from, (2) the exact diff applied (the same diff as Path (a), verbatim
— so upstreaming later is a clean apply, not a re-derivation), (3) a standing reminder that this
vendored copy re-drifts from upstream on every future bebop-repo commit exactly the same way the
original problem happened, so it is not a fix for the *recurring* class of problem, only for
*this instance* of it. **This path does not replace Path (a) — it is a stopgap that should
converge to Path (a) once the operator authorizes touching bebop-repo.**

**Tradeoffs:**
- ✅ Fully executable inside this repo, right now, no cross-repo coordination needed.
- ✅ Unblocks `mesh-adapter`/P34/P34B immediately.
- ❌ Creates a second, dowiz-owned copy of bebop-repo source — exactly the "duplicate source of
  truth" this codebase's engineering culture avoids elsewhere (no precedent for vendoring a sibling
  repo's crate found anywhere else in `docs/design/` — this would be a new pattern, not a reuse of
  an existing one).
- ❌ Every future kernel/bebop-repo evolution re-creates this exact drift, now with TWO places that
  need the same fix applied (upstream + the vendored copy), unless the vendored copy is retired the
  moment Path (a) lands.

## 4. Recommendation (non-binding — operator decides)

Given the fix is small (§1's full enumerated diff) and this repo's stated culture explicitly rejects
duplicate sources of truth, **Path (a) is the architecturally correct answer**; Path (b) should be
used only if there is a genuine near-term need to unblock `mesh-adapter` builds before a bebop-repo
PR can land, and even then only as an explicitly time-boxed stopgap (tracked with the same rigor
`docs/design/OPTICAL-COMPRESSION-DECISION-2026-07-19.md`-style decisions get in this corpus — a
named reopening trigger, not an indefinite fork). **This blueprint does not choose for the
operator** — flagged per the coordinator's explicit instruction.

## 5. Acceptance criteria (either path)

1. **RED, reproducible:** `cd bebop2/delivery-domain && cargo check --offline --features
   kernel-rlib` (from a `bebop-repo` checkout sibling to the target `dowiz` checkout/worktree)
   reproduces exactly the 2 errors quoted in §1, before the fix.
2. **GREEN:** the same command compiles clean after either path's fix lands, with the 4 struct
   literals and 1 match statement updated exactly as specified in §3(a)'s diff.
3. **No behavior change to existing tests.** `bebop2/delivery-domain`'s existing test suite
   (`lib.rs`'s `#[cfg(test)]` module, `intake.rs`'s own tests) passes unchanged — the new
   `vendor_id`/`currency` values are inert defaults for tests that don't exercise multi-vendor/
   multi-currency behavior, and `Refunding`/`CompensatedRefund` mapping to `None` matches the
   existing `Scheduled => None` precedent (no new wire-protocol surface invented).
4. **`mesh-adapter` itself builds green** against the fixed `bebop-delivery-domain`
   (`cd mesh-adapter && cargo check --offline` from a correctly sibling-linked checkout) — the
   actual DoD-1 dependency edge this repo's own `mesh-adapter/Cargo.toml` header comment names.
5. **If Path (b) is chosen:** the vendored fork's `VENDORED-PATCH.md` exists with the 3 required
   fields (§3(b)), and a follow-up tracking item is registered in `ROADMAP.md`/
   `CORE-ROADMAP-INDEX.md` for "retire vendored bebop-delivery-domain-patch once Path (a) lands
   upstream" — so the stopgap cannot silently become permanent.
