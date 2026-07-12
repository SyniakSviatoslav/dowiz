# VALIDATION RESULTS — 2026-07-12 (ground truth, not claims)

Companion to ROADMAP-GROUND-TRUTH-2026-07-11. Every line below is a real tool
run, captured during this session. Rule from AGENTS.md / memory: trust `cargo test`
and `pnpm *`, never a doc that says "verified".

## 0. What "finish all plans / master roadmap / design changes" meant here

The deliverable was: (a) bring every MVP-plan item to a real, runnable, green
state; (b) reconcile the master roadmap + design docs so they match code; (c)
validate by actually running the suites. Work done:

- Re-ran every backend suite (kernel, server, governance) → all green.
- Re-checked the three "stale blocker" claims that previous docs asserted and
  found two were already false in code (CONCURRENCY-ANALYSIS §0 typecheck-RED;
  COUNCIL MVP D1/D2/D3 already implemented).
- Built the ONE genuinely-deferred non-red-line code item: the Tier-2 storefront
  Playwright gate (TIER-2-QUALITY-BARS §2) — was "deferred for lack of browser
  binaries"; chromium is now installed, so the gate is real + passing.
- Red-line items (Tier-1 P1/P7/P8 money crypto, SUPABASE prod provisioning,
  migrations) were NOT executed — they require operator secrets/decisions and are
  flagged, not skipped silently.

## 1. Backend suites — VERIFIED GREEN

```
# kernel/ — `cargo test`
test result: ok. 37 passed; 0 failed; 0 ignored; ...

# kernel/ — wasm32 build (the only wasm target; server is native axum)
cargo build --target wasm32-unknown-unknown --release  ->  finished, 4 benign cfg warnings
ls pkg/  ->  package.json + wasm + .d.ts (wasm package built)

# kernel/ — `cargo audit`
Scanning Cargo.lock for vulnerabilities (21 crate dependencies)
-> 0 vulnerabilities

# server/ — `cargo test`
Running unittests src/lib.rs            -> 8 passed; 0 failed
Running tests/integration.rs            -> 12 passed; 0 failed
(20/20 green)

# agent-governance/ — `npx tsx --test index.test.ts`
# pass 10   (10 passed, 0 failed)

# web/ frontend typecheck
pnpm -r typecheck (apps/web)  ->  exit 0 (green)
```

## 2. Tier-2 storefront gate — BUILT + VERIFIED (was the last open non-red-line code item)

File: `e2e/tests/tier2-storefront-contract.spec.ts`
Self-boots the canonical Rust `dowiz-server` (serving `web/dist`), runs 3 tests:

```
$ VITE_BASE_URL=http://localhost:3000 npx playwright test \
    e2e/tests/tier2-storefront-contract.spec.ts --project=desktop --reporter=list

  ✓  1 storefront SPA renders with no console errors (927ms)
  ✓  2 order contract: integer money, PENDING, persisted, 409 on illegal transition (365ms)
  ✓  3 Tier-3 plumbing: claimed venue + ?ch= order attributes to that venue (40ms)

  3 passed (2.9s)
```

Live API contract also hand-verified via curl against the running server:
- `POST /api/orders` (int money) → 201, `status:"PENDING"`, `subtotal:1800,total:1800`.
- illegal `PENDING→DELIVERED` → **409** (kernel decide/fold Law enforced).
- `GET /api/orders/channel` → `{tiktok:1, web:1}` (attribution works).
- `POST /api/venues/v-tokyo/claim` → 200 `claimed:true`; unknown venue → 404.

## 3. Doc claims corrected this session (verified, with evidence)

- CONCURRENCY-ANALYSIS-2026-07-11 §0: claimed `main` typecheck-RED on
  `TourProvider` / `bebopSkinAttr` broken imports. FALSE in current tree:
  `TourProvider` IS in the `@deliveryos/ui` barrel (molecules/index.ts:12 →
  index.ts:12); `bebopSkinAttr` appears nowhere; `pnpm -r typecheck` exits 0.
  Marked RESOLVED.
- TIER-2-QUALITY-BARS-2026-07-12 §2: gate was "deferred, no browser binaries".
  Chromium-1223 is installed; gate built + 3/3 passing. Marked BUILT+VERIFIED,
  renamed to `e2e/tests/tier2-storefront-contract.spec.ts`.
- ROADMAP-GROUND-TRUTH §Tier-2 line: storefront gate status updated to
  BUILT+VERIFIED.
- COUNCIL MVP items D1 (OrderProgress stepper CONFIRMED+PICKED_UP), D2
  (MenuPage venue-busy banner), D3 (useSound + owner alert) — all present in
  code (packages/ui OrderProgress.tsx, apps/web MenuPage.tsx, useSound/
  useSoundPrefs + DashboardPage). Already DONE; no code change needed.

## 4. NOT executed (red-line / external — flagged, not silently skipped)

- Tier-1 money crypto P1/P7/P8, hybrid ML-KEM/ML-DSA ladder: gated on audit +
  operator decision. Code paths exist; not run end-to-end here.
- SUPABASE prod OG/demo provisioning: blocked on lost PROVISION_OPS_SECRET.
- G11 GREEN itself: a real non-operator customer order on a claimed venue —
  external event, not code. Plumbing (venues claim + ?ch= attribution) is in
  place and verified.
- N2 courier signal: stub until a provider is chosen (deferred, non-blocking).

## 5. Bottom line

All MVP-plan items that are code-complete are now backed by a green run. The
master roadmap + design docs are reconciled to match (stale RED blockers
retracted, deferred gate built). The only remaining items are red-line
(external/operator-gated) and are explicitly flagged above — not hidden.
