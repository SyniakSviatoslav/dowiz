# 🎷 MASTER RED-TEAM SYNTHESIS — bebop2 + dowiz (2026-07-13)

**Operation:** full-scale adversarial assault by an 11-agent specialist army, each simulating a nit-picky, conservative, professional rival trying to hack / steal / break the projects.
**Method:** read-only against local code + live web sessions (`dowiz.fly.dev`, `dowiz-staging.fly.dev`); every claim tagged **CONFIRMED** (traced to `file:line`, reproduced, or PoC-proven) or **PLAUSIBLE** (strong inference). Where possible, findings were **weaponized into running PoCs**. Cross-corroboration across independent agents is treated as verification.
**Targets:** bebop2 @ `feat/logic-governance` · dowiz @ `feat/decentralized-pq-protocol` (deployed legacy API mirrored in `attic/apps-api`).
**Per-lane reports:** `bebop2/docs/red-team/2026-07-13/B{1..4}-*.md` · `dowiz/docs/red-team/2026-07-13/D{1..7}-*.md`.

---

## 0. Bottom line

**bebop2 — "decentralized post-quantum food-delivery protocol":** A genuinely competent, zero-dependency **cryptography library** wearing the marketing of a **protocol that does not exist in the tree**. The classical crypto is real and careful; ML-DSA-65 was genuinely fixed since the last review (60/60 NIST ACVP vectors now pass). But the **authorization layer is trivially broken — a live node-takeover runs today** — the transport is plaintext, "post-quantum" protection is **not actually in force on the wire**, and there is **zero delivery-domain code** (no order/price/money/matcher/settlement/arbitration). Verdict: fund the crypto library on its own merits; do **not** trust "the protocol" — it is a README, not a system.

**dowiz — the food-ordering product:** The **deep engineering is stronger than the protocol project's** — server-authoritative pricing, parameterized SQL, RS256-only JWTs with no alg-confusion, dev-bypasses that fail closed on prod, integer money arithmetic, genuine one-way GDPR anonymization, and a decent dark-themed customer funnel. But it ships **one publicly-documented owner credential that works on production right now**, a cluster of missing role-gates (staff roster + **live courier GPS** exposed to a customer token), an inverted/incomplete data-governance posture (RLS quarantined, runtime BYPASSRLS, CI RLS guard dead), a prod front-door that reads **F** for trust, and — as a business — no billing code, no paying customer, and a wedge that is free elsewhere.

**One-line security posture:**
- bebop2: *the crypto is sound; the trust model built on it is not — and the node still gets taken over.*
- dowiz: *the code is defended in depth; the operational reality (a shipped prod credential, a dead RLS gate, an unfinished secret-scrub) is the fire.*

---

## 1. Cross-corroboration matrix (independent agents that reached the same finding = high confidence)

| Finding | Agents that independently confirmed | Confidence |
|---|---|---|
| **bebop2 node takeover still live** (AnchorRoster written but never wired into receive path) | B2 (running `cargo run` PoC), B3 (replay PoC), B4 (traced) | **CONFIRMED — proven** |
| **bebop2 "post-quantum" not in force** (hybrid gate accepts classical-only, rejects PQ frames; PQ leg unwired) | B1, B2, B3, B4 (4-way) | **CONFIRMED** |
| **dowiz prod owner credential** `test@dowiz.com`/`test123456` grants live `role:owner` API access | D1 (`GET /api/owner/couriers`→200 live), D3 (login→200), D4 (login→200 both envs) | **CONFIRMED — 3-way, live** |
| **dowiz money not server-authoritative in the canonical stack** (WASM prices from caller `unit_price`) + visibly tweens | D2 (client-side pricing), D4 (count-up), D7 (tweens in 4 surfaces), D6 | **CONFIRMED** |
| **dowiz revenue/DB stack quarantined to `attic/`** (no live datastore; billing absent) | D2, D6 | **CONFIRMED** |
| **serde_json→TLV canonical-encoding fix landed** (fair credit — prior finding closed on signed path) | B1, B2, B3 | **CONFIRMED fixed** |
| **Telegram webhook auth not enforced when secret empty** | D1, D3 | **CONFIRMED** |

---

## 2. Ranked findings — both projects (most severe first)

### 🔴 CRITICAL

| # | Project | Finding | Evidence | Impact |
|---|---|---|---|---|
| C1 | bebop2 | **Node takeover: self-issued capability accepted + replayed across nodes.** `HybridGate::check` verifies the signature against the attacker-controlled in-frame `subject_key` and never consults `AnchorRoster`/`verify_chain` (written, 6 green tests, but unreferenced on the live path). Scope never checked; each connection mints a fresh replay-gate; expiry bypassed via hardcoded `now=0`. | B2 running PoC: fresh unenrolled key → `Presence/Send` cap masking `ledger.append drain=ALL` accepted on node 1, replayed byte-identical on node 2. `hybrid_gate.rs:55-90`, `wss_transport.rs:153`. | Any random attacker fully controls any node. The project's headline security remediation is theater. |
| C2 | dowiz | **Shipped prod owner credential.** `test@dowiz.com`/`test123456` (documented in the repo's own CLAUDE.md/memory) → production-key-signed (`kid:"2"`) owner JWT, accepted by owner endpoints live. | D1: `GET /api/owner/couriers`→200 on prod. D3/D4: login→200. `auth/local.ts:85-146`. | Zero-effort authenticated owner foothold on production. Effectively public. |

### 🟠 HIGH

| # | Project | Finding | Evidence | Impact |
|---|---|---|---|---|
| H1 | dowiz | **`owner/couriers` GET routes missing `requireRole`** → a *customer* token can pull the full staff roster + **live courier GPS**. | `owner/couriers.ts:14-15` (only `verifyAuth`+`requireLocationAccess`). | Cross-role PII + real-time location leak. |
| H2 | dowiz | **Cross-tenant customer PII erasure** via client-supplied `customerId` with no location filter; the fail-open `customers` RLS policy lets the null-context worker write across tenants. | `gdpr.ts:48`, `anonymizer/index.ts:119,134-141`. | An owner irreversibly scrubs another venue's customer data by guessing a UUID. |
| H3 | dowiz | **SSRF** — `isPrivateIp` misses IPv4-mapped IPv6 (`::ffff:169.254.169.254`) → brand-extractor reaches Fly 6PN / cloud metadata. | `brand-extractor.ts:150-169`. | Metadata/internal-service access from any owner (incl. C2 backdoor). |
| H4 | dowiz | **RLS reactivation gates (design-open, dormant now):** `couriers` table has **no RLS** while holding `password_hash`+encrypted PII; fail-open anonymous policies on `orders`/`order_items`/`customers` (`USING (app_current_user() IS NULL)` = session-level, not row-scoped); runtime role is **BYPASSRLS**; CI `verify:rls` guard is **dead** (script gone). | `attic/packages-db/migrations/*` (couriers `:5-19`, anon `1780338981783:5-10`), `1780691681296:8`. | Reactivating `attic/` today ships full-table cross-tenant read/write uncaught. |
| H5 | bebop2 | **ML-KEM-768 is not FIPS-203-interoperable** (stores `t`/`s` in coefficient domain, NTT removed) and has **no external KAT** (self-consistency + circular dual-impl only). | `pq_kem.rs:473-474,604,616,622,897,920`. | The KEM half of "post-quantum" is bebop-to-bebop only; a wrong/trapdoored KEM passes its own tests. |
| H6 | bebop2 | **Transport is plaintext "WSS"** (`MaybeTlsStream::Plain`, native-tls disabled) + **cross-connection replay** (fresh gate per connection, `check(&frame, 0)`) + channel-binding decorative. | `wss_transport.rs:118,96,123,153`. | MITM reads all payloads; captured frames replay. Not safe on a hostile network. |
| H7 | dowiz | **Prod auto-deploy + prod DB migration on push-to-`main`** with no approval/rollback/concurrency guard, via **unpinned** `flyctl-actions@master` action holding the prod token. | `.github/workflows/ci.yml:127-153`. | One bad merge or a compromised `@master` tag = prod control / irreversible migration. |
| H8 | dowiz | **Orphaned git blobs retain rotated JWT/PII/RSA private keys**; remote force-push scrub still OPEN → invisible to `git log` and the refs-only gitleaks gate, almost certainly still fetchable on GitHub by SHA. | `git fsck --unreachable` → 10 blobs (e.g. `4505d018`). Hash-compare vs live `.env` = rotated (stale). | Open-source publish is blocked; residual exposure of key *classes* not fully audited. |

### 🟡 MEDIUM / notable

- **bebop2 timing side-channels in ML-KEM** (secret-dependent `continue`, variable-time `%`, non-CT ciphertext compare in `decaps`) — chosen-ciphertext timing oracle (`pq_kem.rs:299-307,708`). · **No zeroization** of any secret material. · **"Anu QRNG" is vaporware** (HEAD commit advertises it; no code).
- **bebop2 envelope `version` unenforced/unauthenticated**; handshake is dead code → no downgrade protection. · **DoS**: real memory ceiling is tungstenite's 64 MiB default, not the advertised 8 MiB; no connection cap / idle timeout (slowloris). · insert-before-verify unbounded nonce set (OOM) + `.expect` panic-DoS.
- **dowiz** shared-IP rate-limit buckets (`req.ip` on Fly proxy → global login-lockout / budget DoS), order-spam throttle bypass (keys on attacker-controlled `body.customer.phone`), `/health` topology disclosure + login user-enumeration oracle, bearer tokens in `localStorage` (no HttpOnly → XSS = owner-session theft), CSP absent on SPA shell + `/admin/*` and weak (`unsafe-inline`/`unsafe-eval`) where present, Docker runs as root + runtime `npm install` w/o lockfile, `.env` mode 0666.
- **dowiz UX (trust-bleeding):** prod has **no landing page** (302→context-free upload wizard); analytics self-contradicts (revenue "0" + "+15%" + test rows); checkout least-accessible (10/12 fields unlabeled, English validation bubble on an Albanian form); e2e junk categories visible on the live storefront; Settings hours form is a **data-loss trap**; demo storefronts dead-end and convert nobody.

---

## 3. Consolidated open gates (security / RLS)

1. **bebop2:** authorization is unenforced end-to-end (trust anchor unwired, scope unchecked, replay open, expiry bypassed, PQ leg stripped) — **the node is owned by construction.**
2. **dowiz:** a public prod owner credential (C2) + missing role-gates (H1) = an authenticated rival with staff/GPS/customer data on day one.
3. **dowiz RLS is not the boundary:** runtime BYPASSRLS + no FORCE-RLS boot-guard + dead CI RLS check + no-RLS/fail-open tables — the isolation model is currently defense-in-*depth-of-one*, and its reactivation is un-gated.
4. **dowiz secret hygiene:** rotated keys still reachable by SHA on the remote; scrub unfinished (blocks the stated open-source goal).
5. **dowiz supply chain:** unpinned prod-deploying action + un-approved prod migrations on `main`.

---

## 4. Prioritized remediation

**P0 — do today (active exposure):**
- Rotate/remove the `test@dowiz.com` prod account; add a prod-seed boot-guard that refuses fixture accounts. *(C2)*
- Add `requireRole('owner')` to the `owner/couriers` GET routes. *(H1)*
- Location-scope the GDPR erasure `customerId`; stop the fail-open `customers` UPDATE policy. *(H2, H4)*
- Fix `isPrivateIp` to canonicalize IPv4-mapped IPv6 + block link-local/metadata. *(H3)*

**P1 — this week:**
- **bebop2:** wire `AnchorRoster::verify_chain` into `HybridGate::check`; enforce scope↔effect; make the nonce store connection-independent + persistent; pass real `now`; enable the ML-DSA leg (and require it under policy). *(C1, PQ-in-force)*
- **dowiz:** finish the remote git-history scrub + full rotation audit; pin the deploy action to a SHA and gate prod migrations behind approval + rollback. *(H7, H8)*
- Enforce Telegram webhook secret (fail closed); fix rate-limit keying (`trustProxy`/`Fly-Client-IP`); add CSP to the SPA shell + `/admin/*`; move tokens to HttpOnly cookies.

**P2 — before any relaunch:**
- **bebop2:** TLS on the transport; enforce+authenticate `version`; add ML-KEM external KATs + FIPS-203 NTT-domain encoding; constant-time KEM; zeroization. Decide honestly whether "protocol"/"post-quantum"/"decentralized" claims stay in the marketing until the code backs them.
- **dowiz:** restore a real prod landing page; fix the analytics contradiction; label checkout fields + localize validation; purge e2e data from prod; fix the Settings hours data-loss trap; resolve the money-tween vs. money-never-tweens decision; re-instate a runtime FORCE-RLS boot-guard + CI RLS gate before reactivating `attic/`.

---

## 5. What held up (fair credit — a good rival reports the walls too)

- **bebop2:** classical crypto is real (RFC/FIPS KATs, constant-time AEAD, zero-dep, empty-import wasm); **ML-DSA-65 was genuinely fixed** (60/60 NIST ACVP, prior CBD break gone, sizes FIPS-exact); **serde_json removed from the signed path** (canonical TLV now); no attacker-triggerable panic in `decode`/`recv`.
- **dowiz:** server-authoritative pricing in the legacy API; parameterized SQL throughout; **RS256-only** JWT, `alg=none`/garbage rejected; dev-bypass **fails closed on prod** (404); most owner routes correctly RLS-scoped; integer money with `CHECK (>=0)`; OTP argon2id + rate-limited; genuine one-way anonymizer (not soft-delete); no service-role key in client; **no live committed secret**; `pnpm audit --prod` = 0 vulns; React auto-escaping held against reflected/search XSS; clickjacking blocked; visible focus rings, no mobile overflow, working sq/en/ua switch.

---

## 6. Execution-integrity signal (the meta-finding)

Both projects share a recurring pattern the rival army flagged independently: **remediations declared "CLOSED"/"DONE" that the code does not back.** bebop2's README marks the auth fix closed while it is unwired (C1); the roadmap's "FIRST REAL ORDER — DONE" is a cargo-test simulation; the "ground-truth" doc marks `/claim` "verified DONE" while prod returns 404 and the cited commit is a GDPR photo-purge. High test counts sit on the easy surfaces (crypto/framing) while the hard surfaces (authorization enforcement, the delivery protocol, live billing) carry ~0 tests. **The single most valuable process fix is: a claim of "done" must be backed by a test that exercises the *live receive/enforcement path*, not an isolated unit — precisely the falsifiable-proof discipline the repos already espouse but did not apply to their headline claims.**

---

*Reports authored by 11 independent specialist agents; this synthesis reconciles and ranks their findings. Nothing in either codebase was mutated — all outputs are additive documentation under `docs/red-team/2026-07-13/`.*
