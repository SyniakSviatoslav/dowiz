# BLUEPRINT P39 — App-shell: installability canon + capability-auth wiring + offer math (2026-07-18)

> **Planning document — writes no product code.** Written against the 20-point contract in
> `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map §9). Component:
> **DELIVERY**. This is P39 exactly as scoped by
> `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §10.5.3: three legs that share one
> phase because each is "the remaining product-surface piece once P37/P38 exist" — (A) the
> **installability gap** (the one §10.5.3 item with no prior unit ID and no canon ruling),
> (B) the DELIVERY-side wiring of **P23** device-auth/TOTP onto P37's live routes (P23 keeps
> its number; P39 hosts the wiring), and (C) **P20 DM-1** kernel discount math (P20 keeps its
> number; DM-1 is its unblocked entry point, hosted here). Structural template:
> `BLUEPRINT-P-A-kernel-primitives.md` (numbering mirrored); sibling precedent:
> `BLUEPRINT-P37-order-http-surface.md`.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

Working tree on `main`, 2026-07-18. All fresh reads; one roadmap-claim drift found and named.

| Claim | Fresh `file:line` (this pass) | Status |
|---|---|---|
| The old stack HAD a real service worker: cache-first shell (`dowiz-shell-v` versioned cache), explicit `/api/`+`/ws/` network passthrough, `UPDATE_CACHE_VERSION` message channel | `git show 069fcbe2a:apps/api/public/sw.js` (read this pass — minified but fully legible); quarantined to attic `fce5738b0` (2026-07-12), purged `f9ab28ff1` (2026-07-15) | **VERIFIED — the PWA precedent is real, and it carries a prior-defect biomarker** (CLAUDE.md critical-biomarkers list: `apps/api/public/sw.js — prior defect`) — the new SW design must treat SW-cache staleness as a first-class adversarial case, not a footnote |
| Roadmap claim "a Tauri desktop installer (`apps/bootstrap-installer`)" — **NOT locatable in git history**: `git log --all --diff-filter=A -- 'apps/bootstrap-installer/**'` → 0 files; `-S "bootstrap-installer"` hits only the two roadmap-doc commits themselves | greps this pass; likely dropped by the clean-history rewrite (`069fcbe2a` "clean-history snapshot… secrets dropped") | **DRIFT NOTED** — the desktop-installer precedent is roadmap-asserted, not source-verifiable. §1's DECART treats the PWA precedent as evidence-backed and the Tauri one as testimony-only |
| New stack has ZERO installability work: grep `serviceWorker\|manifest` over `web/` → 0 files | grep this pass | VERIFIED — P39 leg A starts from nothing, per §10.5.3 ("installability 0% + genuinely undecided") |
| `web/src/app.mjs` — console-only, binds 24/24 `_js` exports incl. `place_order_js`/`apply_event_js` (the F12 local-decide order path) and `estimate_order_total_js`; money asserted `Number.isSafeInteger` at the boundary | `web/src/app.mjs:17-19,55-61,86-95` | VERIFIED — the surface an installed shell must launch INTO; the local-decide path already exists to be reached in airplane mode |
| `estimate_order_total_js`'s config shape already carries `min_order_value`, `delivery_fee_flat`, `tax_rate`, `price_includes_tax` — but **no discount field** | `web/src/app.mjs:98-105` | VERIFIED — DM-1's wasm-estimate extension point |
| `compute_order_total(subtotal, tax_rate, price_includes_tax, fee) -> Result<i64,String>` — checked_add throughout, non-negative assert; module doc: "**No discounts in this scope**"; its own comment names the old oracle law it restricts: "total = subtotal + deliveryFee + taxTotal **− discountTotal** (orders.ts:565)" | `kernel/src/domain.rs:129-145` (fn), `:11` (no-discounts doc), `:123-124` (oracle mirror comment) | **VERIFIED — DM-1's exact target: the discount slot the oracle had and the kernel deliberately dropped is re-added as law, not bolted beside it** |
| `place_order_priced` re-derives every unit price from the trusted `PriceCatalog`, ignores caller prices, sets `price_trusted = true`, fail-closed on unknown product | `kernel/src/domain.rs:198-232` | VERIFIED — discount math must compose AFTER catalog-authoritative pricing, never weaken it |
| P20 confirmed 0% built: `kernel/src/offer.rs` absent (ls this pass); `OfferKind`/`OfferRedeemed`/`PromotionType` → 0 grep hits in `kernel/src/` | ls + grep this pass | VERIFIED — re-confirms §10.5.3's P20 row from source |
| P37's routes and cap seam are now BLUEPRINTED (supersedes §10.5.3's "no dedicated blueprint" note): `ROUTE_ORDER_PLACE/READ/ADVANCE`, `CAP_HEADER = "x-dowiz-cap"`, `CapVerifier` trait binding `(route ‖ body digest ‖ epoch)`, middleware-before-handler admission via witness type | `BLUEPRINT-P37-order-http-surface.md` §2 (route consts, `CapVerifier`), §3.3 (W37-3) | VERIFIED — leg B wires ONTO this, never beside it |
| Kernel capability machinery in-repo: `verify_chain<V: SignatureVerifier>`, append-only `RevocationSet`, `DOMAIN_DELEGATION` signing domain, deterministic `RefSigner` for tests | `kernel/src/ports/agent/cap.rs:480` (verify_chain), `:406` (RevocationSet), `:33` (DOMAIN_DELEGATION), `:102` (RefSigner) | VERIFIED — leg B's substrate; bebop2 `HybridGate` swap-in stays the P34-boundary seam per P37 §4.4 |
| Auth canon D3: device-bound keypair = capability cert (PRIMARY), TOTP = enrollment/step-up factor ONLY; sequencing P1 (`totp.rs`, zero deps, landable today) → P2 (enrollment mints cert, no HTTP needed) → P3 (HTTP wiring, blocked on a dynamic surface — **P37 is that surface now**) | `docs/design/BLUEPRINT-AUTH-DEVICE-2FA-2026-07-17.md:113` (D3 row), `:176-188` (§4 sequencing), `:200-207` (§5.1 totp.rs API), `:212-219` (§5.2 enrollment flow), `:251-258` (§7 done-checks) | VERIFIED — P39 leg B executes P1+P2+P3 against that blueprint verbatim; zero re-design here |
| Zero TOTP/2FA code exists anywhere in the Rust stack | auth blueprint §1.3 (`grep -rliE 'totp\|webauthn\|two.?factor'` → 0), re-run this pass over `kernel/ engine/ web/` → 0 | VERIFIED — P23-P1 baseline RED |
| `totp.rs`'s home is `bebop2/core` (`/root/bebop-repo`), NOT this repo — cross-repo rule is standing | auth blueprint §5.1; memory `cross-branch-todo-map-2026-07-10` ("bebop/wire-native-core files → /root/bebop-repo NOT /root/dowiz") | VERIFIED — §10's T-tasks name the repo per file |
| Offline canon F12: solo island runs the full flow; network is never the required order path | `docs/design/ARCHITECTURE.md:75`; P37 §3.5 (offline-parity test `kernel.offline-order.test.mjs`) | VERIFIED — the SW must not introduce what F12 forbids; DoD-1's airplane-mode gate is this canon, installed |
| Money red-lines standing: integer minor units only, `checked_*` everywhere, money-never-tween (`TweenGuard`) landed and binding | `kernel/src/domain.rs:129-145`; `engine/src/money_guard.rs:50` (per P38 §0, re-confirmed present) | VERIFIED — DM-1 operates entirely inside these laws |

Ground truth is non-discussible; everything below builds on this table only.

---

## 1. Scope — what P39 owns vs deliberately does NOT (§10.5.3 anti-scope, sharpened)

**P39 owns (build items §3):**

| Item | Leg | Content |
|---|---|---|
| W39-1 | A | Installability **canon decision** as a committed ADR (the §10.5.3 first deliverable: decision BEFORE code) — the DECART is in this section; the ADR records it |
| W39-2 | A | The chosen shell: web manifest + service worker over the existing `web/` + `native-spa-server` stack, F12-honoring (cache-only shell, never a network gate) |
| W39-3 | B | P23-P1: `totp.rs` in `bebop2/core` — RFC 6238 primitive, KAT-green, zero deps (auth blueprint §5.1 executed verbatim) |
| W39-4 | B | P23-P2: enrollment decide-path — TOTP-verified enrollment mints a device capability cert (pure functions over existing cap machinery; no HTTP) |
| W39-5 | B | P23-P3: step-up wiring onto P37's live routes — a `StepUpPolicy` layer over the already-admitted (`CapVerifier`) request, applied to at least one real sensitive route |
| W39-6 | C | P20 DM-1: `kernel/src/offer.rs` + `compute_order_total` gains the discount slot back as kernel law; property tests pin the money invariants |
| W39-7 | A | Installed-offline proof: install + airplane-mode launch reaches the local-decide order path (the §10.5.3 DoD-1 falsifier, made a named test) |

**P39 explicitly does NOT own (each with its owner):**

- **NOT a password+TOTP primary login** — capability certs are primary auth per canon (D3);
  TOTP is enrollment/step-up ONLY (§10.5.3 anti-scope verbatim; auth blueprint §5.4 rejections
  apply unchanged: no password table exists to protect).
- **NOT WebAuthn/passkeys** — auth blueprint §5.3's recorded default is defer-and-revisit
  (option iii); overriding it is one operator sentence, not this phase's initiative.
- **NOT DM-2..DM-8** — offer *redemption pipeline*, demo generation, publishing, OG assets are
  P20's own scope (publication gated behind P18 public-flip). DM-1 is math only.
- **NOT P22 social posting** — ECOSYSTEM-adjacent, stays P22 (§10.5.3 verbatim).
- **NOT rendering** — the shell installs and launches the surface P38 renders; zero pipeline,
  token, or DOM-visible work here (P38's zero-visible-DOM gates apply to anything W39-2 adds).
- ~~NOT a Tauri/native wrapper build~~ — **SUPERSEDED, see §1.1's RESOLVED block below.**
  Operator ruling 2026-07-18: Tauri native wrapper IS Wave-0, not deferred.
- **NOT the admin/owner surface** — P48's lane; W39-5 wires the step-up *mechanism* and proves
  it on P37's route set; P48 consumes the same layer for its sensitive operations later.
- **NOT push notifications** — the old SW's push half (VAPID etc.) is P43/P49 territory; the
  W39-2 service worker ships with NO push handler (adding one here would front-run P43 DoD-2).

### 1.1 The installability DECART (leg A's first deliverable, decided honestly)

| Criterion | **(a) PWA: manifest + SW on the existing stack** (chosen) | (b) Native wrapper (Tauri-class) | (c) Both now | (d) Reject installability |
|---|---|---|---|---|
| New dependencies | **Zero** — a JSON manifest + one vanilla-JS SW file served by the existing `native-spa-server`; no toolchain change | Large Rust dep tree + system webview + per-OS packaging/signing pipeline | (a)+(b) costs summed | zero |
| Fit to the users who install | Courier + owner phones (Android-dominant per the budget-device canon, memory `gaussian-splatting-address-picker-arc-2026-07-16`): home-screen install + offline launch is exactly the PWA feature set | Desktop-first; the courier does not carry a desktop | — | The courier/owner daily-driver need is real (the old stack built BOTH shells; the SW is source-verified §0) |
| F12 compatibility | SW is cache-only shell; local-decide wasm path already works serverless (P37 §3.5) — install strengthens F12 | Also fine | fine | Browser tab still works, but no offline *launch* entry |
| Evidence base | Old-stack SW source-verified (§0) | Roadmap testimony only — not locatable in history (§0 drift) | — | — |
| Reversibility | Delete 2 files + 1 route entry | Delete a packaging pipeline | worst | trivial |
| OS-keychain / biometric custody | Not available — device key lives in wasm/origin storage (auth blueprint §5.3 explicitly accepts this: "the browser-side key can live in the wasm kernel… WebAuthn's marginal win is OS-keychain custody, not the trust model") | Available | — | — |

**VERDICT (as originally written by this pass — now SUPERSEDED, kept for the record,
not deleted): (a) PWA-first.** Wins on every criterion that has evidence behind it. **(b)
is deferred, not rejected**, with two named triggers that reopen it: (1) a real requirement
for OS-keychain/biometric key custody (the §5.3 revisit), or (2) an operator-stated
desktop/app-store distribution need. **(c) rejected Wave-0** — two shells to maintain
before one is proven contradicts reuse-first. **(d) rejected on evidence** — the old stack
built installability twice; the need is demonstrated, not hypothetical. Falsifiability of
the verdict: it flips if the first-client device fleet turns out not to support PWA install
(checkable the day devices are known).

**RESOLVED (2026-07-18, operator ruling — reopens trigger (2), see original verdict above):
(b) Tauri native wrapper, Wave-0, not deferred.** This pass's own DECART row "Fit to the
users who install" assumed Tauri is desktop-only ("the courier does not carry a desktop") —
that assumption is **stale**: Tauri 2.0 went stable 2024-10-02, current line 2.11.x
(2.11.5, 2026-07-01), and its stable "Mobile Update" targets **Android + iOS from the same
Rust codebase**, with native-API plugins out of the box for **NFC, biometric auth, barcode
reading, and deep links** — not merely "desktop with a webview." This directly reopens
reopen-trigger (1) as well (OS-keychain/biometric custody, §1.1's own row 5), not only
trigger (2): Tauri's mobile build gets biometric custody the PWA path explicitly cannot
(§1.1 row 5, "Not available"). Two concrete load-bearing connections to already-designed
work, not a generic "native is nicer" preference:
- **P52 §3.4b's NFC PoD** (added same day) already designs a courier-phone NFC tap against
  `tools/nfc-pod-codec`; Tauri's `tauri-plugin-nfc` gives that a real native API instead of
  depending on browser Web NFC's inconsistent availability.
- **`BLUEPRINT-AUTH-DEVICE-2FA-2026-07-17.md` §5.3's WebAuthn-revisit trigger** — "a real
  requirement for OS-keychain/biometric key custody" — is satisfied BY THIS RULING; §5.3's
  own default ("(iii) now, revisit after P3 exists") should be re-read as reopened, not
  independently re-litigated here — cross-reference only, that file's own edit is out of
  this blueprint's scope.
**Honest residual risk, not glossed over:** Tauri's own 2026 documentation notes mobile
support is newer than the desktop path and "works best for straightforward apps like forms,
content readers, and productivity tools" — this product's WebGPU field-render surface (P38a)
is NOT that shape, so W39-1's build items must include a falsifiable check that Tauri's
webview on Android/iOS actually hosts `wgpu`'s WebGPU backend at acceptable frame time
(§3 below, new adversarial case) — this is the genuine open engineering risk the ruling
accepts, not a hidden one. PWA is NOT rejected — it demotes to a documented fallback/second
shell (§1.1's own (c) reasoning reverses: with (b) now Wave-0 and carrying the real
mobile-native win, a *thin* PWA manifest+SW costs little extra and keeps the F12 no-install
browser-tab path alive for a device Tauri's mobile build doesn't yet cover — build both,
Tauri primary/PWA secondary, not "two full shells").

### 1.2 Shell refinement (2026-07-18, SECOND operator ruling — payment-path §4-A — refines §1.1's Tauri-Wave-0 block on the desktop-webview point ONLY)

> Append-only correction per repo convention. §1.1's RESOLVED block (Tauri native wrapper =
> Wave-0) STANDS as written for mobile and as the packaging/installer mechanism. Nothing above
> is deleted. What this block refines is the **desktop webview-hosting-the-UI assumption** that
> §1.1's "honest residual risk" paragraph carried (lines ~116–126: "Tauri's webview on
> Android/iOS actually hosts `wgpu`'s WebGPU backend"). A second, later operator ruling on the
> **same day** — the payment-surface decision — made the *desktop half* of that assumption
> obsolete. Recording it here rather than silently reinterpreting §1.1.

**The ruling that produces this** (`SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` §4-A, now CLOSED):
the operator chose **Path C — hosted redirect for card capture** on **web and desktop** (redirect
to the payment provider's own verified domain over a signed, short-TTL, single-use session;
the **server-side webhook is the source of truth** — NOT a client-side "success redirect"),
**explicitly rejecting Path A** (scoped provider-iframe overlay). Per that synthesis's own
dependency table (§2, X3): Path A was the *only* PCI-compliant card-capture surface that required
a **live webview/DOM host on desktop** (X3 desktop row). With Path A rejected, that forcing
function is gone.

**Consequence — desktop targets `winit` + `wgpu` + AccessKit, with NO embedded webview** (X3
desktop row verbatim: "choosing C frees desktop to be `winit`+`wgpu`+AccessKit; choosing A forces
a webview host"):
- The desktop card-capture step happens through the **system's own browser / an OS-level URL
  handler** (the hosted redirect) — **not** an embedded webview. Payment was the last remaining
  reason a desktop build would need a DOM host; it no longer does.
- Every other desktop surface is already zero-DOM full-`wgpu` (§16.30, *ad fontes* §16.42) with a
  **native AccessKit** accessibility tree — production-ready on winit desktop (synthesis X1:
  "Native (winit desktop, Android): AccessKit crate directly … zero hand-rolling"). No other
  desktop DOM requirement exists.
- **SUPERSEDED for desktop:** §1.1's residual-risk clause — "Tauri's webview … hosts `wgpu`'s
  WebGPU backend at acceptable frame time" — is **moot on desktop**, because the desktop UI is not
  hosted in a webview at all. That risk survives **only** for the *mobile* webview question, which
  P63 measures (below). Any §1.1 reading that assumed a Tauri desktop webview would host the
  payment UI or any other DOM content is marked superseded here.
- **Preserved (be precise):** §1.1's **Tauri-as-packaging/installer** role is NOT superseded by
  this block. If a packager (Tauri or otherwise) is used purely to produce/sign a **desktop
  installer around the `winit`+`wgpu` binary**, that is orthogonal to webview-hosting and is
  untouched here. What is superseded is specifically "a Tauri **webview** hosts the desktop UI /
  desktop payment DOM." Zero desktop DOM content — payment or otherwise — lives in an embedded
  webview.

**Mobile (Tauri) — shell unchanged, distinct card path:**
- Tauri remains the **mobile packaging/shell mechanism**; §1.1's mobile-native rationale
  (NFC/biometric plugins, §1.1's P52 §3.4b NFC-PoD + §5.3 biometric-custody links) stands
  unchanged. Mobile **keeps its native Tauri webview/shell**.
- Mobile card-capture default is **Path B — native provider SDK sheet** (zero DOM, the cleanest
  §16.30 fit — X3 mobile row), with **Path C (hosted redirect) as the mobile fallback**.
- Path B rests on a **named, still-unresolved engineering unknown**: the **native-SDK-over-GPU-
  surface bridge** (the Tauri-plugin-over-`wgpu`-surface question — synthesis R2 risk #3, listed
  as an engineering unknown in synthesis §4-E). This block does **NOT** resolve it — it is cited
  as an open unknown, exactly as the synthesis leaves it. Until P63 reports, Path B is the
  directional default, not a confirmed-feasible one.

**This is a *directional* ruling; `BLUEPRINT-P63` supplies the *evidence*:** this decision is now a
named **input to P63 (shell & platform spike)**. Per the synthesis build sequence (§3.2, "P63 is
first among equals"), P63 produces the **measured verdict** — desktop `winit`+`wgpu` vs any
alternative, **mobile raw-`wgpu`-surface vs WebGPU-in-webview**, and the native-SDK-over-GPU
bridge feasibility — on real budget hardware (frame time, battery) that **confirms or refines**
this ruling with benchmarks. This file records the operator's **directional** shell ruling only;
P63 records the **evidence**. If P63's measurements contradict the directional ruling (a desktop
`winit`+`wgpu` path unviable on a target OS, or the mobile Path-B bridge infeasible), P63's
evidence governs and a further dated block refines this one. Cross-refs:
`SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` §2 (X3 coupling table), §4-A (the CLOSED ruling), §5
(the **P39-rev** canon-diff row and the **P63** blueprint row that name this exact intake).

**Installability / PWA / service-worker consequence (see the note appended at §3.2):** the
manifest + service worker (leg A / W39-2) are a **web-browser-path artifact only** — a service
worker requires a browser/DOM context, which the desktop `winit`+`wgpu` shell does **not** have.
On desktop there is no webview ⇒ no service worker, no PWA install: **desktop installability is
the native binary/installer, not the PWA path.** The manifest+SW stay live for the browser-tab /
web path (§1.1's demoted-to-secondary PWA shell) and for installed-PWA push on iOS-Safari web
customers (X10) — unaffected by this desktop refinement.

---

## 2. Predefined types & constants (standard item 4 — named BEFORE implementation)

```rust
// ── leg A: web/manifest.webmanifest + web/sw.mjs (NEW files, served statically) ──
// Manifest: name/short_name/start_url="/"/display="standalone"/icons (from the
// brand asset set, dowiz-brand canon). No JS. Served by native-spa-server's
// existing ServeDir — zero server change for the manifest itself.
//
// Service worker constants (in web/sw.mjs — vanilla JS, no toolchain):
//   const SHELL_CACHE_PREFIX = "dowiz-shell-v";   // SAME naming as the old SW (proven pattern)
//   const SHELL_CACHE_VERSION = "2";               // v1 was the old stack's; never reuse
//   const PRECACHE = [ "/", "/index.html", app.js bundle, kernel wasm, fonts ];
//   // NETWORK PASSTHROUGH (never cached, never intercepted): /api/*, /healthz —
//   // the SW must be INERT for P37's dynamic routes. The old SW's /api/+/ws/
//   // passthrough is the precedent; the prior-defect biomarker is why §3.2's
//   // adversarial set exists.
//
// ── leg A: the ADR file ─────────────────────────────────────────────────────
// docs/design/adr/ADR-P39-installability-canon.md — records §1.1's verdict,
// the two reopen-triggers, and the falsifier. Committed BEFORE W39-2 code.

// ── leg B: bebop2/core/src/totp.rs (NEW — /root/bebop-repo, cross-repo rule) ──
// API verbatim from BLUEPRINT-AUTH-DEVICE-2FA §5.1 (not re-designed here):
//   sha1 / hmac_sha1 / hotp / totp / totp_verify(±skew, ct-compare) / base32_encode
// KAT sources pinned there: RFC 2202 · RFC 4226 App. D · RFC 6238 App. B.
// Time enters ONLY as a caller-supplied counter (no std::time) — pq_dsa.rs:13
// entropy-model discipline.

// ── leg B: kernel/src/ports/agent/enroll.rs (NEW module beside cap.rs) ──────
/// P23-P2: pure decide-path. TOTP-verified enrollment mints a device capability
/// cert over the EXISTING chain machinery (cap.rs verify_chain/:480,
/// DOMAIN_DELEGATION/:33, RevocationSet/:406). No HTTP, no clock, no RNG —
/// caller supplies time counter and entropy, same law as totp.rs.
pub struct EnrollmentRequest {
    pub device_pubkey_hybrid: Vec<u8>,  // Ed25519 ⊕ ML-DSA-65 public halves
    pub totp_code: u32,
    pub unix_time: u64,                 // caller-supplied (counter discipline)
}
pub enum EnrollError { BadTotp, RevokedIssuer, ExpiredWindow, Malformed }
/// Ok = a capability cert signed by the issuer key, scope-limited, with expiry.
/// Err = NO cert minted, no state touched (RED-first test: bad TOTP ⇒ nothing).
pub fn enroll_device(/* issuer signer, roster, request */) -> Result<Vec<u8>, EnrollError>;
pub const TRUST_WINDOW_SECS: u64 = 30 * 24 * 3600; // Better-Auth-borrowed 30d, refreshed on use
                                                    // = cert expiry field, NOT a cookie (§5.2 canon)

// ── leg B: tools/native-spa-server/src/stepup.rs (NEW module, layered on api.rs) ──
/// P23-P3: step-up runs AFTER CapVerifier admission (P37 §3.3), never instead.
/// Policy is data, not code branches: routes listed here demand a fresh TOTP
/// proof bound to the SAME frame digest the cap check already bound.
pub struct StepUpPolicy { pub routes: &'static [&'static str] }  // e.g. [ROUTE_ENROLL, ROUTE_REVOKE]
pub const STEPUP_HEADER: &str = "x-dowiz-stepup";   // TOTP code + counter, signed into the frame
pub const STEPUP_MAX_AGE_SECS: u64 = 90;            // ≤ 3 TOTP periods; stale proof ⇒ 403
pub enum StepUpReject { Missing, BadCode, Stale, Replayed }      // all map to 403, fail-closed

// ── leg C: kernel/src/offer.rs (NEW module) + domain.rs extension ───────────
/// DM-1: discount math as kernel law. Integer-only; percent = basis points;
/// rounding rule pinned (floor) so every node computes the same i64.
pub enum OfferKind { PercentBps(u32), FixedMinor(i64) }   // 1000 bps = 10%
pub struct Offer { pub kind: OfferKind, pub min_subtotal: i64 }  // gate, not clamp
pub const MAX_PERCENT_BPS: u32 = 10_000;   // 100% ceiling; > ⇒ Err, never saturate
/// discount amount for a subtotal. Fail-closed: negative/oversized inputs ⇒ Err.
/// Law: 0 ≤ discount ≤ subtotal (a discount NEVER makes money flow backward).
pub fn discount_amount(subtotal: i64, offer: &Offer) -> Result<i64, String>;

/// domain.rs — the ONE authority extended (not forked):
/// compute_order_total(subtotal, tax_rate, price_includes_tax, fee,
///                     discount: Option<i64>) -> Result<i64, String>
/// Law (restores the oracle mirror at domain.rs:123-124 as kernel math):
///   effective = subtotal − discount   (checked_sub; discount > subtotal ⇒ Err)
///   total     = effective + tax(effective) + fee   (checked_add throughout)
/// discount = None ≡ Some(0) — byte-identical to today's outputs (the migration
/// regression pin, §3.6). Callers (recompute_total, wasm estimate) thread the
/// new slot; the wasm estimate config gains an optional `discount` field.
```

Rejected alternatives (DECART one-liners): **a parallel `compute_order_total_discounted`** —
rejected: two total authorities is the exact fork `place_order_priced` exists to prevent; one
signature, extended. **f64 percent discounts** — rejected: float money violates the standing
integer law; basis points + pinned floor rounding is deterministic. **SW-side route caching for
`/api/`** — rejected: an offline-served stale order state masquerading as live is the
prior-defect class the old sw.js biomarker warns about; the SW is inert for dynamic routes,
offline ordering goes through the wasm local-decide path ONLY (F12). **TOTP as primary auth**
— rejected per canon (D3); repeated because it is the most likely drive-by regression.

---

## 3. Build items — spec → RED test → code, each with adversarial cases (items 3, 5)

### 3.1 W39-1 — the ADR (decision before code)

Commit `docs/design/adr/ADR-P39-installability-canon.md` carrying §1.1's table, verdict,
reopen-triggers, falsifier. **RED:** the file does not exist (verified §0 — zero installability
artifacts). **GREEN:** file committed; W39-2's PR links it. **Adversarial (process):** any
W39-2 code merged before the ADR exists = not-done per §5 (the §10.5.3 ordering is binding:
"recorded before code").

### 3.2 W39-2 — manifest + service worker, F12-honoring

`web/manifest.webmanifest` + `web/sw.mjs` per §2; registration is one guarded line in the web
entry (feature-detected, failure-silent — an unsupported browser gets today's exact behavior).
The SW: precache the shell asset list at install, serve cache-first for precached statics,
**pass through untouched** anything under `/api/` or `/healthz`, version-bump invalidation via
`SHELL_CACHE_VERSION` (the old SW's proven mechanism, minus its push/message surface).

- **RED:** `web/tests/installability.spec.mjs` (Playwright) — asserts manifest served with
  correct MIME + SW registered + shell assets served from cache on second load. Fails today
  (no files). **GREEN** after W39-2.
- **Adversarial (the biomarker's lesson, three arms):** (i) *stale-shell*: bump
  `SHELL_CACHE_VERSION`, reload ⇒ old cache deleted, new shell served (assert cache-storage
  keys); (ii) *API-inertness*: with SW active, a `POST /api/order` reaches the server
  byte-identical and is NEVER answered from cache — asserted by a server-side nonce echo;
  (iii) *SW-absent parity*: the full web test suite passes with the SW unregistered — the SW
  is an accelerant, never a dependency (same "additive-only" proof shape as P38's `e21`).

> **Note (2026-07-18, §1.2 cross-ref — no-webview-on-desktop consequence):** W39-2's manifest+SW
> are **web-browser-path only**. Following §1.2's second operator ruling (desktop =
> `winit`+`wgpu`+AccessKit, **no embedded webview**), the desktop shell has no DOM/browser
> context, therefore **no service worker and no PWA install** — desktop installability is the
> native binary/installer, not this SW. Everything in §3.2/§3.7 applies to the **browser-tab /
> web** path (and installed-PWA push on iOS-Safari web customers, X10); it is inert-by-absence on
> desktop, not merely inert-by-passthrough.

### 3.3 W39-3 — P23-P1 `totp.rs` (bebop-repo)

Execute auth blueprint §5.1/§7-P1 verbatim in `/root/bebop-repo/bebop2/core/src/totp.rs`.
**RED:** the module is absent (grep 0, §0). **GREEN:** `cargo test -p bebop2-core totp` green
with the full KAT set (RFC 2202 all 7 · RFC 4226 App. D all 10 · RFC 6238 App. B all 6 SHA-1
rows) + RED arms (wrong code; ±2 periods at skew=1; tampered secret) + `no_std` build +
`grep -c 'std::time'` → 0. **Adversarial:** non-constant-time compare is the classic
implementation bug — the verify path must route through the existing `constant_time_eq` shape
(`aead.rs:361` precedent, cited in the auth blueprint), asserted by review-grep in CI.

### 3.4 W39-4 — P23-P2 enrollment decide-path (kernel)

`kernel/src/ports/agent/enroll.rs` per §2: `enroll_device` verifies the TOTP against the
issuer-held secret (P1's `totp_verify`, mirrored or vendored per the cross-repo seam —
the kernel gets the ~80-line HOTP core under the same KAT set, one authority per repo
boundary documented in the module header), then signs a delegation-scoped cert with
`DOMAIN_DELEGATION` (`cap.rs:33`) using the existing signer trait; expiry =
`TRUST_WINDOW_SECS`, refresh-on-use = re-mint on a valid authenticated request past half-life
(cert field, not cookie — §5.2 canon). **RED-first (auth §7-P2 verbatim):** enrollment with a
bad TOTP mints NO cert and touches NO state; with a good TOTP the minted cert passes
`verify_chain` (`cap.rs:480`); a revoked device's frame is rejected via `RevocationSet`
(`:406`). **Adversarial:** replayed enrollment request (same TOTP counter window) ⇒ second
mint refused (counter monotonic per issuer); an enrollment attempt whose issuer key is itself
revoked ⇒ `RevokedIssuer` (the chain is checked from the root down, not assumed).

### 3.5 W39-5 — P23-P3 step-up onto P37's routes

`stepup.rs` per §2 layers AFTER P37's `CapVerifier` middleware (the `Admitted` witness is a
precondition — step-up without prior cap admission is unrepresentable). Wave-0 policy set: the
enrollment/revocation route class (the P48 roster operations will join the same list in P48's
lane). The TOTP proof binds `(route ‖ body digest ‖ counter)` — the same binding discipline as
the cap frame, so a step-up minted for one action cannot authorize another.

RED tests (each written first):

```text
s1_stepup_required_route_without_proof_403   — cap-valid frame, no STEPUP_HEADER → 403, zero writes
s2_stepup_bad_code_403                        — wrong TOTP → 403, zero writes
s3_stepup_stale_403                           — proof older than STEPUP_MAX_AGE_SECS → 403
s4_stepup_replayed_403                        — same proof re-sent → 403 (counter ring)
s5_stepup_valid_admitted                      — positive control (deny ≠ broken, P37 r10c pattern)
s6_stepup_never_primary                       — a request with ONLY a TOTP proof and no cap frame
                                                → 401 at the CAP layer; step-up alone opens nothing
```

**Adversarial (canon guard):** `s6` is the anti-scope made mechanical — TOTP can never become
a primary credential by accident, because the middleware ordering makes it unreachable.

### 3.6 W39-6 — DM-1 offer math (kernel)

`kernel/src/offer.rs` per §2 + the `compute_order_total` signature extension. Steps: (1) pin
today's behavior — capture fixtures of `compute_order_total` outputs over the existing test
corpus; (2) extend the signature with `discount: Option<i64>`, thread through
`recompute_total` (`domain.rs:110-118`) and the wasm estimate path; (3) assert `None` ≡
`Some(0)` ≡ pre-change fixtures **byte-identical** (extraction-must-be-motion discipline, P37
§3.1's pattern); (4) land `discount_amount` + property tests; (5) delete the "No discounts in
this scope" doc line (`domain.rs:11`) in the same commit — the roadmap DoD-3 falsifier.

Property tests (proptest over i64 ranges + bps ranges): `total ≥ 0` always;
`discount ≤ subtotal` or Err (never clamp); `PercentBps(10_000)` ⇒ effective = 0 exactly;
`PercentBps(bps)` monotone non-decreasing in bps; floor rounding pinned by a golden vector
(e.g. 999 minor units at 3333 bps ⇒ 332, not 333); overflow inputs ⇒ Err not panic;
`min_subtotal` unmet ⇒ Err (a gate, not silent zero). **Adversarial:** negative
`FixedMinor` ⇒ Err (a "discount" that adds money is the money-red-line inversion — tested
unreachable); `discount` on a `price_trusted = false` order composes identically (discount
math is orthogonal to price provenance — asserted so nobody "fixes" untrusted prices via
offers); tax computed on the DISCOUNTED base, pinned by golden vector against the oracle law
(`domain.rs:123-124`).

### 3.7 W39-7 — installed-offline proof (the §10.5.3 DoD-1 falsifier)

Playwright spec `web/tests/installed-offline.spec.mjs`: (1) load the surface once (SW installs,
shell precaches); (2) go offline (context network disabled) AND stop the server process;
(3) reload from the SW cache — the shell launches; (4) drive `place_order_js` +
`apply_event_js` through the golden sequence and assert the final order JSON equals P37 §3.5's
offline fixture byte-for-byte (one fixture, three referees: wire, serverless node, installed
shell). **Adversarial:** while offline, a `fetch('/api/order')` attempt from the page fails
fast with a network error — it is NOT served a cached fake success (the SW inertness proof
under the exact condition where faking would be tempting).

---

## 4. Cross-cutting design obligations (items 6, 8, 9, 11–16)

### 4.1 Hazard-safety as math (item 6)

Reachability arguments, not prose: **"offline user sees stale state as live"** is unreachable
for dynamic data because the SW has no cache entry for `/api/*` to serve (inertness is
structural — the fetch handler's passthrough list — and adversarially tested §3.2/§3.7).
**"TOTP becomes primary auth"** is unreachable because the step-up layer consumes P37's
`Admitted` witness as a typed precondition (§3.5 s6). **"A discount mints money"** is
unreachable: `discount_amount` is total over its domain with `0 ≤ d ≤ subtotal` enforced by
checked arithmetic and Err arms — the negative-flow state has no representation, and the
property corpus falsifies the claim continuously. **"Enrollment without proof"** is
unreachable: `enroll_device`'s only Ok path traverses `totp_verify` (RED-first test pins the
no-cert-on-failure behavior). **"Shell update bricks the app"**: versioned cache + the
SW-absent-parity test (§3.2 iii) means the worst SW failure degrades to today's non-installed
behavior — degrade, never crash.

### 4.2 Schemas for scaling (item 8)

Stated axes: **precache manifest size** (shell + wasm ≈ low MB today; break point = wasm
growth past mobile-quota comfort → split kernel wasm into lazy-loaded modules, named not
taken); **TOTP verify rate** (microseconds each, no axis at venue scale); **offer evaluation**
(O(1) per order line, no axis); **step-up counter ring** (per-issuer u64 monotonic — no
growth). Honest non-axis: this phase adds no data plane that grows with orders.

### 4.3 Isolation / bulkhead (item 11) + error-propagation gates (item 14)

The SW is process-isolated by the platform; its failure mode is bounded by §3.2-iii's parity
test (suite green with SW unregistered). Step-up is a layer scoped to the policy route list —
non-listed routes' behavior is byte-unchanged (P37's r-suite re-run green is the regression
proof). Named CI gates per bug class: fixture byte-pin (silent total drift), s6 (auth-canon
inversion), API-inertness nonce test (cache-poisoning), KAT sets (crypto regression),
`std::time` grep (clock leak into the pure core).

### 4.4 Mesh awareness (item 12)

Entirely **node-local**. The manifest/SW are static assets on P37's server; enrollment and
step-up ride P37's existing request path (no new wire events); offer math is in-process kernel
law. Zero new transport payloads. The one cross-repo seam: `totp.rs` lands in bebop2/core and
the kernel's enroll module mirrors the ~80-line HOTP core under the same KATs (both sides
pinned to identical RFC vectors — divergence is a KAT failure, not a review question).

### 4.5 Rollback / self-healing vocabulary (item 13, used precisely)

**Self-Termination leg claimed:** typed `EnrollError`/`StepUpReject`/offer `Err` arms;
witness-ordered middleware; fail-closed everything. **Self-Healing leg claimed narrowly:** the
versioned SW cache is genuine regenerative redundancy for the SHELL only (a corrupt cache is
deleted and refetched); claimed for static assets, never for state. **Snapshot-Re-entry: NOT
claimed.** Mechanical rollback: delete manifest+sw+registration line (today's tree, §3.2-iii
proves equivalence); delete `offer.rs` + revert the one signature (fixtures prove
output-identity at `None`); delete `enroll.rs`/`stepup.rs` (additive modules).

### 4.6 Living memory (item 15) + tensor/spectral (item 16) + Linux discipline (item 9)

Item 15: the SW cache is a demote-never-delete tier for shell assets (old versions garbage-
collected only after a new version is proven serving — the activate-handler order the old SW
already used); certs carry temporal validity (expiry/refresh) rather than flat permanence.
Item 16: honestly N/A — no closed-form math beyond integer discount arithmetic; eqc-rs not
applicable, stated not decorated. Item 9 verdicts: **ALREADY-EQUIVALENT** — one total
authority extended in place ("one implementation of one concept"); **REINFORCES** — mechanism
(step-up layer) separated from policy (the route list is data); **GAP** honestly named — no
real-device install matrix exists; Playwright's install emulation is the Wave-0 proof, real
Android install verification waits for the first-client device fleet (recorded, not hidden).

---

## 5. DoD — falsifiable, RED→GREEN, extending §10.5.3's three items (item 2)

| §10.5.3 DoD | Named test(s) (RED today → GREEN at close) | Permanent regression (item 17) |
|---|---|---|
| 1. Installability canon decision + (if accepted) install + airplane-mode launch reaches local-decide | ADR committed (§3.1); `installability.spec.mjs` (§3.2); `installed-offline.spec.mjs` (§3.7 — the airplane-mode falsifier) | both Playwright specs |
| 2. P23-P1 landed; P23-P3 wired onto P37's routes, step-up only | totp KAT suite (§3.3); enroll RED-first set (§3.4); `s1..s6` (§3.5 — s6 IS the "never primary" falsifier) | KATs + s1..s6 |
| 3. DM-1: kernel discount math; "No discounts in this scope" gone; property-pinned money invariants | offer property corpus + golden vectors (§3.6); `grep "No discounts in this scope" kernel/src/domain.rs` → 0 (the roadmap's own falsifier, made a CI grep) | property corpus + fixture byte-pin + the grep gate |
| (new, SW safety) | API-inertness nonce test + stale-shell + SW-absent parity (§3.2) | all three |

Ledger rows (`docs/regressions/REGRESSION-LEDGER.md`, ratchet rule): DM-1 fixture byte-pin,
s6 never-primary, API-inertness. **Not-done clauses:** W39-2 code before the ADR = not done;
any route added to the step-up policy without a listed justification = not done; a float
anywhere in the discount path = not done regardless of green totals; a push handler in the SW
= not done (P43's lane).

---

## 6. Benchmark plan (item 10) — small and honest

This phase has one hot path worth measuring and several deliberately unmeasured cold paths
(install is a once-per-device event; enrollment is once-per-device; benching them would be
decorative). Measured: **`compute_order_total` with the discount slot** — criterion bench
`kernel/benches` gains `order_total_with_discount` beside the existing money benches,
RED-commit-first so `bench_track` auto-seeds (P-A §6 discipline); budget: within 10% of the
pre-change `compute_order_total` baseline (a checked_sub and one branch — regression past that
means someone added allocation). Shell numbers recorded once, not gated: cold-vs-SW-cached
load time printed by `installability.spec.mjs` into its report (evidence for the ADR's benefit
claim, appended to `BENCH_HISTORY.md`-style notes; a CI gate on network timing would flake).

---

## 7. Links to docs & memory (item 7)

Depends on / cites: `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §10.5.3 (scope
authority; P39 DoD extended §5) · `CORE-ROADMAP-STANDARD-2026-07-17.md` (contract) ·
`BLUEPRINT-AUTH-DEVICE-2FA-2026-07-17.md` (legs B's ENTIRE design authority — §5.1/§5.2/§7
executed, not re-derived; its §5.3 WebAuthn deferral honored) ·
`BLUEPRINT-P37-order-http-surface.md` (route/cap substrate; §3.3 middleware, §3.5 offline
fixture reused) · `BLUEPRINT-P38-webgpu-render-engine.md` (the surface being installed;
zero-visible-DOM gates inherited) ·
`docs/design/DEMO-MARKETING-PIPELINE-REFACTOR-2026-07-17.md` (P20's home; DM-1 hosted here,
DM-2+ stays there) · `docs/design/ARCHITECTURE.md:75` (F12) ·
`docs/regressions/REGRESSION-LEDGER.md`. Memory: `rust-native-bare-metal-decision-2026-07-14`
(DECART discipline §1.1) · `test-integrity-rules-2026-06-27` (money red-lines — §3.6 operates
inside them) · `cross-branch-todo-map-2026-07-10` (bebop files → bebop-repo, §3.3) ·
`never-bypass-human-gates-2026-06-29` (WebAuthn stays operator-gated) ·
`dowiz-brand-voice-canon-2026-07-07` (manifest icons/name). Supersedes: nothing — executes
three existing designs under one phase.

---

## 8. Hermetic principles honored (item 20 — load-bearing only)

- **P1 MENTALISM** (spec is source): the ADR precedes the shell; §2's types precede every
  implementation; the KAT vectors ARE the totp spec.
- **P2 CORRESPONDENCE** (one concept, one authority): one total function extended (never a
  discounted twin); one cap admission path gaining one step-up layer; one offline-order
  fixture refereeing three launch contexts (§3.7).
- **P6 CAUSE-AND-EFFECT** (determinism as law): integer bps + pinned floor rounding; fixture
  byte-pins; counter-based TOTP with caller-supplied time (no ambient clock anywhere).
- **P7 GENDER** (paired verification): the SW is refereed by the SW-absent parity run; the
  discount math is refereed by pre-change fixtures AND the old oracle law; every deny arm has
  its positive-control twin (s5, r10c pattern).

(Other principles not load-bearing here; not claimed decoratively.)

---

## 9. Standard-compliance map (all 20 points, checkable)

| §2 item | Where satisfied |
|---|---|
| 1 ground truth | §0 (fresh cites; the bootstrap-installer drift found and named) |
| 2 DoD | §5 (extends §10.5.3's 3 items with named tests + grep falsifiers) |
| 3 spec/event-driven TDD | §2 spec-first; §3 RED-first per item; §3.4 asserts on the no-cert event outcome, not just return values |
| 4 predefined types/consts | §2 |
| 5 adversarial/breaking tests | §3.2 (stale-shell, cache-poisoning, SW-absent), §3.3 (ct-compare), §3.4 (replay, revoked issuer), §3.5 (s1-s6), §3.6 (negative offer, overflow, rounding), §3.7 (offline fetch honesty) |
| 6 hazard-safety as math | §4.1 (five unreachable-state arguments) |
| 7 links docs/memory | §7 |
| 8 scaling axes | §4.2 (incl. one honest non-axis) |
| 9 Linux discipline | §4.6 (verdicts incl. an honest GAP: no real-device matrix) |
| 10 benchmarks+telemetry | §6 (one real bench gated; cold paths honestly unmeasured) |
| 11 isolation/bulkhead | §4.3 |
| 12 mesh awareness | §4.4 (node-local, stated; cross-repo seam named) |
| 13 rollback/self-heal vocabulary | §4.5 (two legs claimed precisely, one refused) |
| 14 error-propagation gates | §4.3 (five named gates) |
| 15 living memory | §4.6 |
| 16 tensor/spectral | §4.6 (honestly N/A, stated) |
| 17 regression ledger | §5 (three rows named) |
| 18 agent-executable instructions | §10 |
| 19 reuse-first | §1.1 (DECART with rejected alternatives), §2 (rejected alternatives), header (three existing designs executed, zero re-derived) |
| 20 Hermetic citations | §8 |

---

## 10. Clear instructions for other agentic workers (item 18 — zero session context assumed)

Three independent lanes after T1; **T2 targets `/root/bebop-repo`, everything else
`/root/dowiz`**. Lane A (shell): T1→T3→T4. Lane B (auth): T2→T5→T6 (T6 needs P37's routes
merged). Lane C (offers): T7 alone. Nothing here waits on O18a/GPU.

1. **T1 (W39-1).** Write `docs/design/adr/ADR-P39-installability-canon.md` carrying §1.1's
   table, verdict (PWA-first), the two reopen-triggers, and the falsifier. Commit BEFORE any
   other P39 code. Acceptance: file exists, linked from this blueprint's §1.1 by a follow-up
   one-line edit.
2. **T2 (W39-3, bebop-repo).** Create `/root/bebop-repo/bebop2/core/src/totp.rs` per
   `BLUEPRINT-AUTH-DEVICE-2FA-2026-07-17.md` §5.1 (API verbatim) with the full §7-P1 KAT set
   RED-first. Acceptance: `cargo test -p bebop2-core totp` green; `no_std` build green;
   `grep -c 'std::time' core/src/totp.rs` → 0. Push to `openbebop` remote (memory: bebop
   `origin` is archived).
3. **T3 (W39-2).** Add `web/manifest.webmanifest` + `web/sw.mjs` per §2 + the guarded
   registration line; write `web/tests/installability.spec.mjs` RED-first with the three §3.2
   adversarial arms. Acceptance: spec green; full existing web suite green WITH the SW
   unregistered (parity arm).
4. **T4 (W39-7).** Write `web/tests/installed-offline.spec.mjs` per §3.7, reusing P37 §3.5's
   offline fixture (do NOT fork the fixture). Acceptance: green with the server process
   stopped; the offline `/api/` fetch-fails-fast arm asserted.
5. **T5 (W39-4).** Create `kernel/src/ports/agent/enroll.rs` per §2 (RED-first: bad-TOTP ⇒
   no cert). Use `RefSigner` (`cap.rs:102`) in tests; document the bebop2 HybridGate swap
   seam in the module header (P37 §4.4 pattern). Acceptance:
   `cargo test -p dowiz-kernel enroll` green incl. replay + revoked-issuer arms.
6. **T6 (W39-5).** Create `tools/native-spa-server/src/stepup.rs` per §2, layered after
   P37's cap middleware; add `s1..s6` per §3.5 RED-first. Acceptance: all six green; P37's
   existing r-suite re-runs green byte-identical for non-policy routes.
7. **T7 (W39-6).** Capture `compute_order_total` fixtures FIRST; create `kernel/src/offer.rs`
   per §2; extend the signature + thread callers + wasm estimate config; property corpus +
   golden vectors per §3.6; delete the `domain.rs:11` "No discounts in this scope" line in
   the same commit; add the CI grep gate + the criterion bench (RED-commit-first).
   Acceptance: `cargo test -p dowiz-kernel offer` + domain suite green; `None`-discount
   fixtures byte-identical; ledger rows added.
8. **T8 (close-out).** Run everything: kernel + server + web suites; verify every §5 row;
   add the three REGRESSION-LEDGER rows. Do NOT mark P39 done if the ADR postdates any shell
   code, if s6 was weakened, or if any float touched the discount path.

**Forbidden in this phase (for the zero-context reader):** no password login, no WebAuthn, no
push handlers in the SW, no DM-2+ pipeline work, no P22 posting, no Tauri build, no caching of
`/api/` responses, no float money, no TOTP-as-primary anywhere.
