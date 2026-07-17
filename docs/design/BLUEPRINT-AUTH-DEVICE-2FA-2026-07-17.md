# BLUEPRINT — Device-Based Authorization + TOTP 2FA for the dowiz Admin Surface

> 2026-07-17 · research/planning artifact only, no code written · branch `feat/harness-llm-backend`
> Built per AGENTS.md **Detailed Planning Protocol** (ground truth → dependencies → inline DECART →
> blueprint-grade → falsifiable checks → 2-question self-critique → consolidated single artifact).
> Trigger: operator is leaning toward device-based authorization + authenticator codes (TOTP) + 2FA
> and asked whether **Better Auth** (TypeScript) already covers it.

## §0 — Verdict in one paragraph

**Native Rust zero-dep primitive, not Better Auth.** Better Auth's `twoFactor` + `passkey` plugins
do cover everything the operator wants — but there is no longer any Node/TS service in dowiz for
them to sit in: the JS/TS product stack is deleted at HEAD **and at `origin/main`** (§1.1), and the
live runtime is a `FROM scratch` Rust static server (§1.2). Adopting Better Auth today means
*creating* a brand-new Node runtime + npm supply chain + Better-Auth-managed DB tables solely for
auth — reversing the just-completed zero-OCI/zero-Node migration, violating the hermetic direction,
and contradicting the standing rust-native-bare-metal decision ("older = adapters, but an adapter
needs an existing surface to bound; here the surface itself would be new"). Meanwhile 2/3 of the
needed machinery already exists natively: device-keypair capability certs with revocation
(bebop2 `proto-cap`) **are** device-based authorization in the strongest sense, and TOTP (RFC 6238)
is a ~300-line zero-dep primitive squarely inside the `pq_dsa.rs`/`sign.rs` from-scratch-with-KATs
convention. Better Auth's **flow design is borrowed as inspiration only** (§6): trusted-device
window, backup codes, lockout counters, ±1-period tolerance, encrypted secret at rest. The one
honestly-hard part is WebAuthn/passkeys (needs P-256 ECDSA + CBOR from scratch) — deferred to
Phase 2 with an explicit operator decision point (§5.3).

---

## §1 — Ground truth (live-verified this session; every claim from command output, not memory)

### 1.1 The JS/TS product stack is gone — at HEAD *and* at `origin/main`
- `git ls-files 'apps/*'` → **0 files**. `git ls-tree -r origin/main -- apps/` → **0 files**.
  The deletion is merged, not a feature-branch-only state (this *corrects* the tasking premise
  "main = frozen TS anchor"; memory's ROADMAP note is stale on this point).
- `git ls-files 'packages/*'` → **5 files**, all dead manifest stubs with zero `.ts` source:
  `packages/config/{package.json,tsconfig.json}`, `packages/core/package.json`,
  `packages/platform/{package.json,tsconfig.json}` (same 5 on `origin/main`). `packages/platform`
  still *declares* `jose`/`pg`/`ioredis` deps but has no source to run them.
- Total JS-family files at HEAD: **9** — wasm-bindgen glue (`kernel/pkg-web/dowiz_kernel.js`,
  `wasm/demo/pkg/*`), plus `web/serve.mjs` (a **zero-dep dev-only** static file server, per its own
  header comment), `web/src/app.mjs`, `web/src/lib/kernel/kernel_client.mjs` + its test. None is a
  backend service; none has a package dependency tree in production.
- Caveat, stated honestly: the tasking cited "AGENTIC-MESH-PROTOCOL-CONSOLIDATED.md §1 finding #2"
  for this deletion; grep of that document (`apps/`, `ls-files`, `typescript`, `delet`) found **no
  such finding text**. The git evidence above is primary and stronger; the doc citation is marked
  not-relocated rather than assumed.

### 1.2 The live runtime is Rust-static; there is NO dynamic backend — and therefore NO live login
- `deploy/native-spa-server.service` (header, verbatim): "the binary that **actually runs today**:
  the DK-04 zero-OCI static SPA server (`tools/native-spa-server/`)… Dockerfile stage 3
  `FROM scratch`, `ENTRYPOINT ["/native-spa-server",…]`" — hardened systemd unit
  (`DynamicUser`, `MemoryDenyWriteExecute`, empty capability set), serving read-only.
- `grep -rn 'axum\|hyper\|TcpListener' engine/src kernel/src */Cargo.toml` → **no dynamic HTTP
  server in the native stack**. The old Fastify `/api/auth/local/login` died with `apps/api`.
- Consequence that reframes the whole question: **dowiz today has no authentication surface at
  all** — not a weak one, none. This blueprint is not "replace auth", it is "design the first
  native auth primitive for whenever the dynamic admin surface lands."

### 1.3 Zero 2FA/TOTP/WebAuthn code exists anywhere in the Rust stack
`grep -rliE 'totp|webauthn|two.?factor|2fa|passkey' --include='*.rs' .` → 0 hits.

### 1.4 The native machinery that already covers "device-based authorization"
- `bebop2/proto-cap/src/hybrid_gate.rs`, `node_id.rs`, `revocation.rs`, `lib.rs` (`verify_chain`) —
  operator-signed **capability certificates over a device keypair** (hybrid Ed25519 ⊕ ML-DSA-65),
  with a revocation set. The crypto-safe-first pass already shipped an **operator-cert** flow
  (commit `caf560f`, memory: crypto-safe-first-pass-2026-07-14). The mesh arc reuses exactly this
  as its admission plane ("Admission = the existing `HybridGate::check`", mesh doc §1).
- Standing stance (SOVEREIGN-EVENT-EXCHANGE blueprint, memory): "trust = signed **capability**,
  NEVER reputation." Device enrollment ≡ minting a capability cert for a device pubkey. This *is*
  device-based authorization; nothing needs inventing at the trust layer.

### 1.5 The from-scratch-crypto precedent and what primitives exist/are missing
- `bebop2/core/src/pq_dsa.rs:1` — "ML-DSA-65 (FIPS 204…) implemented **from scratch, zero external
  crates**", KAT-verified against NIST ACVP vectors (1,286 lines).
- `bebop2/core/src/sign.rs:1` — "Ed25519 (RFC 8032 §5.1…), from scratch, zero-dependency",
  bit-exact §7.1 vectors, RED case asserted (1,042 lines).
- `bebop2/core/Cargo.toml` — `[dependencies] # none.` (dev-deps only, test parsing).
- `bebop2/core/src/hash.rs` — zero-dep **SHA-512**, SHA3-256, SHA3-512, KAT-green (450 lines).
- `bebop2/core/src/aead.rs:361` — `constant_time_eq` already exists.
- **Missing for TOTP**: SHA-1, HMAC, Base32, HOTP/TOTP. (RFC 6238's default and the only algorithm
  Google Authenticator-class apps reliably honor is HMAC-**SHA-1**; the `algorithm` URI parameter is
  widely ignored by consumer apps, so SHA-1 must be implemented even though SHA-256/512 variants
  exist in the RFC.) Estimated from-scratch cost against the precedent above: SHA-1 ≈ 80 lines,
  HMAC ≈ 30, Base32 ≈ 40, HOTP+TOTP ≈ 80, KAT tests ≈ 100 → **~330 lines + vectors**, all with
  published test vectors (FIPS 180-4 / RFC 2202 / RFC 4648 / RFC 4226 App. D / RFC 6238 App. B).
- **Missing for WebAuthn**: P-256 ECDSA verify (new curve — real work, order of 1–2k lines from
  scratch, comparable to the curve25519 field arithmetic already done in `sign.rs`), SHA-256
  (~100 lines), CBOR/attestation-object parsing. This is the honest big-ticket item (§5.3).

### 1.6 What Better Auth actually provides (skill `better-auth-best-practices` + better-auth.com docs)
- **`twoFactor` plugin**: TOTP (encrypted secret in DB, configurable digits/period, **accepts ±1
  period** for clock skew, otpauth URI for QR enrollment), email/phone OTP via custom `sendOTP`,
  one-time **backup codes**, **`trustDevice`** (30-day trusted-device cookie, refreshed on each
  sign-in), **account lockout** after repeated failures (`failedVerificationCount`, `lockedUntil`).
  Schema: `user.twoFactorEnabled` + a `twoFactor` table.
- **`passkey` plugin**: WebAuthn/FIDO2 "powered by SimpleWebAuthn"; `passkey` table (credentialID,
  publicKey, counter, transports, aaguid…); resident-key / userVerification knobs; conditional UI.
- **`deviceAuthorization` plugin**: this is **RFC 8628 OAuth Device Authorization Grant** — for
  smart TVs / CLIs / input-constrained devices (user-code + polling). It is **not** device-binding
  and not what a restaurant admin panel needs; naming collision only (§2).
- **Runtime reality**: Node/TS server library — `npm install better-auth`, an `auth.ts`, a DB it
  manages tables in via its CLI migrator, framework route handlers, JS client libs. There is no
  Rust/WASM build of it. Every feature above is inseparable from a Node runtime.

---

## §2 — Disambiguation: three different things called "device-based authorization"

| # | Mechanism | What binds the device | Strength | Who offers it |
|---|---|---|---|---|
| D1 | RFC 8628 device grant | Nothing — it's a *login UX* for keyboardless devices | n/a (not binding) | Better Auth `deviceAuthorization` |
| D2 | Trusted-device cookie | A bearer cookie (stealable, syncable) | Weak — cookie theft = device | Better Auth `twoFactor.trustDevice` |
| D3 | **Device-bound keypair** | Private key generated on-device, never leaves it; server holds pubkey (+ cert/counter) | Strong, phishing-resistant | WebAuthn/passkeys — and **bebop2 proto-cap capability certs (§1.4), already built** |

The operator's phrase "device-based authorization" for an owner admin panel means **D3** (with D2 as
the convenience layer on top). Better Auth's identically-named plugin is D1 — adopting it "because
the name matches" would buy the wrong thing. TOTP is orthogonal to all three: it is the *human*
proof used at enrollment of a new device and as step-up/recovery.

**TOTP vs WebAuthn as the "authenticator codes" mechanism (2026 posture):** TOTP is shared-secret,
phishable (relay/AiTM), but universal, offline, and trivially cheap to implement. WebAuthn/passkeys
are asymmetric, origin-bound, phishing-resistant — the direction NIST SP 800-63B-4 and the broader
2024–2026 industry shift push for admin/high-value access (phishing-resistant MFA; syncable
passkeys accepted at AAL2) — but require nontrivial server-side verification crypto (P-256, CBOR).
*(Standards RFC 4226/6238/8628 and the WebAuthn model are stable knowledge; the exact SP 800-63B-4
finalization details were **not** re-verified live this session — web budget exhausted — and are
marked accordingly in §8.)* For a restaurant-owner panel: few named users, phone always present,
counter tablets are shared devices, money/PII actions (payouts, GDPR) sit behind red-lines →
best-practice shape is **long-lived trusted-device sessions + step-up on sensitive operations**,
not short sessions with constant prompts.

---

## §3 — DECART (Integration Decart Rule, inline per Detailed Planning Protocol step 3)

**Choice:** auth mechanism for the future dowiz admin surface (device authorization + TOTP 2FA).

| Criterion | **A. Better Auth as bounded adapter** | **B. Native Rust zero-dep primitive** (chosen) | C. Rust crate stack (`totp-rs`, `webauthn-rs`) |
|---|---|---|---|
| Bare-metal / hermetic fit | Requires a **new** Node runtime + npm tree in a stack that just reached `FROM scratch` zero-OCI (§1.2); no existing JS surface to bound it to (§1.1) | Pure Rust in `bebop2/core` beside `pq_dsa.rs`/`sign.rs`; no new runtime, no new deps | Rust-native runtime, but breaks the `[dependencies] # none.` crypto-core convention (§1.5) |
| Falsifiable correctness | Upstream test suite (not ours); our KAT/RED discipline can't reach inside it | RFC 4226 App. D + RFC 6238 App. B + FIPS 180-4 KATs, RED cases asserted — same proof shape as pq_dsa ACVP gate | Crates are testable, but *our* gates assert their API, not their internals |
| Supply chain | npm graph + Better Auth release cadence + SimpleWebAuthn transitively | **zero** added edges | 2 crates + transitive deps in the crypto path (`deny` surface grows) |
| Reversibility | Hard: DB tables managed by its CLI migrator; a second persistence authority beside the WORM event log | Trivial: one module, caller-supplied entropy/time, deletable | Moderate: swap crate for from-scratch later |
| Fit to existing machinery | Ignores proto-cap; would duplicate device trust as cookies (D2) instead of capability certs (D3) | **Reuses** `HybridGate::check`/`verify_chain`/`RevocationSet`/operator-cert unchanged (§1.4) — only TOTP is genuinely new | Same reuse possible; only the primitive sourcing differs |
| Cost to first working TOTP | Days of code — but **weeks** of runtime re-introduction (deploy, hardening, DB adapter) | ~330 lines + KATs (§1.5); smaller than any single existing crypto module | Hours — cheapest raw code path |

- **Tiebreak (rule: tie → Rust-native wins):** not needed for A-vs-B — B wins outright on four of
  six criteria and ties the rest. B-vs-C is decided by the crypto-core convention and the KAT
  discipline: for a **~330-line** primitive with published vectors, the crate saves almost nothing
  and costs the `# none.` invariant. C is *not* rejected for WebAuthn — see §5.3.
- **Older-as-adapter clause:** nothing is purged — there is nothing left to purge (§1.1). Better
  Auth is not "kept as a bridge" because a bridge needs two banks; the Node bank no longer exists.
- **Banned-reason check:** "Better Auth is popular/battle-tested" was not used as a deciding
  criterion (social proof banned); "never roll your own crypto" as *slogan* likewise — the repo's
  own falsifiable-KAT record (ACVP-green ML-DSA, RFC-8032-exact Ed25519, and one **real** external
  forgery-class bug found *and fixed* in its own code, SSR-2020 batch-verify, commit `6541ae8`)
  is the evidence that the from-scratch-with-KATs discipline works here.
- **Probe — strongest honest argument AGAINST the chosen B (steelman, mandatory):** Authentication
  is the one domain where incumbent advice says never self-build, and Better Auth would deliver the
  *entire missing account/session/2FA/passkey product surface* — lockout, backup codes, trusted
  devices, enrollment UX, schema, SimpleWebAuthn — **in days**, whereas dowiz today cannot even
  serve a login POST (§1.2): the native path must first build a dynamic HTTP surface, then TOTP,
  and its WebAuthn half is a 1–2k-line P-256/CBOR project measured in weeks. If the admin panel
  needs real protection *soon*, B's calendar risk is genuinely higher than A's architectural cost.
  Why it still didn't win: there is no live panel to protect yet (§1.2) — the calendar pressure is
  hypothetical while the runtime-reversal cost is concrete and immediate; and the TOTP+cert core of
  B (Phase 1) is small enough that the calendar gap is days-vs-week, not days-vs-months. The gap
  only becomes real at WebAuthn — which is exactly where §5.3 keeps option C open.

**DECISION: B — native zero-dep Rust primitive** (TOTP + capability-cert device enrollment now;
WebAuthn as a Phase-2 decision point). Better Auth's **flow design** is adopted as inspiration
(§6); its **code and runtime** are not.

---

## §4 — Sequencing (derived, not assumed) and explicit dependencies

- **P1 (no prerequisites, independent):** `totp.rs` primitive in `bebop2/core` — depends only on
  existing `hash.rs` conventions; needs nothing from dowiz. Can land and be KAT-green *today*.
- **P2 (depends on P1 + existing proto-cap; NOT on any HTTP server):** device-enrollment flow as
  pure decide-path functions — "TOTP-verified enrollment mints a capability cert." Testable
  in-kernel without a network.
- **P3 (depends on a dynamic admin surface that DOES NOT EXIST YET):** wiring to real HTTP. The
  admin surface itself is owned by other arcs (native-spa-server extension, or the mesh
  `AgentBridge` admission plane which already routes through `HybridGate::check`). **This blueprint
  deliberately does not invent that surface** — naming the gap honestly per Protocol step 4 rather
  than papering it with a fictional `axum` design the hermetic direction might reject.
- **P4 (independent of P3; operator decision):** WebAuthn (§5.3).

P1 ∥ P4-decision are independent; P2 needs P1; P3 blocks on an external arc. Nothing here blocks
the mesh Wave-0 work or is blocked by it.

## §5 — The design (blueprint-grade where honest, gaps named)

### 5.1 Phase 1 — `totp.rs` (new module, `bebop2/core/src/totp.rs`, mirrored conventions)
Zero-dep, `no_std`+`alloc`, RNG-free/clock-free hot path (entropy model identical to
`pq_dsa.rs:13` — "all randomness enters ONLY through caller-supplied byte streams"; here likewise
**time enters only as a caller-supplied counter**, never `std::time`):

```rust
pub fn sha1(msg: &[u8]) -> [u8; 20];                       // FIPS 180-4, KAT: RFC 3174/NIST vectors
pub fn hmac_sha1(key: &[u8], msg: &[u8]) -> [u8; 20];      // RFC 2104, KAT: RFC 2202
pub fn hotp(secret: &[u8], counter: u64, digits: u32) -> u32;            // RFC 4226, KAT: App. D
pub fn totp(secret: &[u8], unix_time: u64, step: u64, digits: u32) -> u32; // RFC 6238, KAT: App. B
pub fn totp_verify(secret: &[u8], unix_time: u64, code: u32, skew: u8) -> bool; // ±skew periods, ct compare
pub fn base32_encode(b: &[u8]) -> alloc::string::String;   // RFC 4648, for otpauth:// URIs
```
Constant-time comparison reuses the `aead.rs:361` `constant_time_eq` shape. Secret-at-rest
encryption (Better Auth does this — §1.6) uses the existing `aead.rs` AEAD; storage itself is
event-log territory and belongs to P3's owning arc.

### 5.2 Phase 2 — device enrollment = capability cert, not a new subsystem
Flow (pure functions over existing types; exact call-site file:line is a P3-time read, honestly
deferred): device generates hybrid keypair (existing `sign.rs` + `pq_dsa.rs` keygen) → owner proves
personhood via `totp_verify` → an operator/root key signs a **capability cert** for the device
pubkey (existing operator-cert path, `caf560f` precedent) → every admin request is a `SignedFrame`
verified by `HybridGate::check`; revocation = existing `RevocationSet` (lost/stolen device = one
revocation entry, no password resets). Trusted-device window (Better Auth's 30-day `trustDevice`,
borrowed) = **cert expiry + refresh-on-use**, a field in the cert, not a cookie.

### 5.3 Phase 2b — WebAuthn/passkeys: explicit operator decision point, not silently resolved
The phishing-resistant end-state wants WebAuthn. From-scratch cost: P-256 ECDSA verify + SHA-256 +
CBOR parse (§1.5) — the one place where the from-scratch convention is genuinely expensive. Three
honest options, deliberately left open: **(i)** from-scratch P-256, same ACVP-KAT discipline,
weeks; **(ii)** `webauthn-rs` as a *bounded adapter* (decart tiebreak would flag the dep — requires
its own decart when proposed); **(iii)** defer — for a low-user-count owner panel, device
capability certs (§5.2) already deliver D3-strength phishing-resistant device binding *without*
WebAuthn, since the browser-side key can live in the wasm kernel (`kernel/pkg-web`) — WebAuthn's
marginal win is OS-keychain/biometric custody, not the trust model. **(iii) now, revisit after P3
exists** is the default this blueprint records; overriding it is one operator sentence.

### 5.4 Explicit rejections
- Better Auth as runtime/code — §3.
- `deviceAuthorization` (RFC 8628) — wrong problem (§2, D1).
- SMS/email OTP as the second factor — weakest channel; and dowiz has no native mailer today.
- Password + 2FA as the primary shape — with capability certs, the *device* is the first factor
  and TOTP the enrollment/step-up factor; no password table, nothing to breach or reset.

## §6 — What is borrowed from Better Auth (design only, no code)

1. ±1-period TOTP acceptance window (clock skew) — §1.6.
2. One-time backup codes, generated at enrollment, stored hashed, single-use.
3. Failed-verification counter + temporary lockout (`failedVerificationCount`/`lockedUntil` shape).
4. Trusted-device semantics: 30-day trust **refreshed on use** (mapped to cert expiry, §5.2).
5. otpauth:// URI + QR enrollment UX.
6. Their schema's field-level completeness (aaguid/counter/transports for passkeys) as the
   checklist for whatever P3's event-log projection stores.

## §7 — Falsifiable done-checks (Protocol step 5)

- **P1:** `cargo test -p bebop2-core totp` green with: RFC 2202 HMAC-SHA-1 all 7 cases; RFC 4226
  App. D all 10 HOTP values; RFC 6238 App. B all 6 SHA-1 rows; RED cases — wrong code rejected,
  code from ±2 periods rejected at `skew=1`, tampered secret rejected. `--no-default-features`
  (no_std) build green. `grep -c 'std::time' core/src/totp.rs` → 0.
- **P2:** in-kernel test: enrollment with bad TOTP → **no cert minted** (RED first); with good
  TOTP → cert verifies via `HybridGate::check`; revoked device's frame rejected.
- **P3:** owned by the surface arc; its gate must include: unauthenticated admin request → 401/deny
  path asserted, and a replayed `SignedFrame` rejected.
- **Non-check:** "looks integrated with Better Auth docs" — explicitly not a criterion.

## §8 — 2-question doubt audit (AGENTS.md ritual, applied to this blueprint)

**Q1 — least confident about (not rounded down):**
1. **SP 800-63B-4 specifics** cited from knowledge, not re-verified live (web-search budget was
   exhausted this session) — the phishing-resistance/AAL framing is directionally solid; exact
   clause numbers are not to be quoted downstream from this doc.
2. **The mesh-doc "§1 finding #2" citation** from the tasking was never located by grep (§1.1
   caveat); if the operator relies on that doc's wording elsewhere, it should be re-found manually.
3. **Whether any *other* live service still runs the old TS stack** (e.g. a stale Fly machine on
   dowiz.fly.dev deployed from a pre-deletion commit) was not probed — no deploy state was queried,
   only the repo. If a legacy Fly app is still serving, it changes nothing about the forward
   design but would be a live unauthenticated-or-old-auth surface worth a separate check.
4. **Consumer-app SHA-1-only behavior** ("Google Authenticator ignores the algorithm parameter") is
   knowledge-based, not re-tested in 2026 app versions; it is why §5.1 includes SHA-1 at all.
5. **The P-256 from-scratch cost estimate** (1–2k lines, weeks) is an analogy to `sign.rs`, not a
   prototyped measurement.
6. **Where exactly P3's HTTP surface will land** (native-spa-server extension vs AgentBridge plane)
   — deliberately unresolved (§4), but it means §5.2's "exact call site" is a named gap, not a spec.
7. **`packages/platform` stub deps** (`jose`, `pg`) — assumed dead because no source exists; not
   verified that no external tooling still reads those manifests.
- Bucket triage: items 1, 2, 4–7 are routine stated assumptions. **Item 3 is the "1-in-4" risk** —
  recommend the operator (or next session) run `flyctl status -a dowiz` before treating "no live
  auth surface" as globally true rather than repo-true.

**Q2 — the biggest thing I might be missing (one honest answer, no hedge):** This blueprint answers
"which auth *primitive*" rigorously, but the operator's real bottleneck is that **dowiz currently
has no dynamic admin surface at all** — the fastest route to "owners can securely log in" is not
choosing TOTP's hash function, it is deciding P3's owning arc. If the operator's underlying intent
was "get a working admin login soon," the honest reading of §3's steelman is that the calendar
argument for Better Auth gets stronger the longer P3 has no owner — and that pressure should be
answered by *scheduling P3*, not by re-litigating the primitive.

## §9 — Pointers
- Precedent modules: `/root/bebop-repo/bebop2/core/src/pq_dsa.rs`, `…/sign.rs`, `…/hash.rs`,
  `…/aead.rs` · Capability plane: `/root/bebop-repo/bebop2/proto-cap/src/{hybrid_gate,node_id,revocation}.rs`
- Live runtime: `/root/dowiz/tools/native-spa-server/`, `/root/dowiz/deploy/native-spa-server.service`
- Direction docs: `docs/design/hermetic-architecture-2026-07-16/HERMETIC-ARCHITECTURE-PRINCIPLES.md`,
  memory `rust-native-bare-metal-decision-2026-07-14.md`, AGENTS.md §Integration-Decart-Rule +
  §Detailed-Planning-Protocol
- Better Auth references (design inspiration only): better-auth.com/docs/plugins/{2fa,passkey,device-authorization}
