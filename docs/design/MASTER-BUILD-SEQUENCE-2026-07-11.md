# Master Build Sequence — dowiz + bebop — 2026-07-11

> The single dependency-ordered map that ties together every blueprint produced this session, so the
> parallel code work has one sequence instead of ~30 scattered docs. Organized by GATE, not by topic:
> what can be built now at zero pivot-risk, what waits behind the prod-merge vehicle, what waits behind
> the first real order (`G11 GREEN`), and what waits behind the crypto audit. Nothing here is code —
> it is the order in which the code (built in parallel) should land. Every item traces to a blueprint
> that carries its own phased plan + Verified-by-Math RED cases.

## Source blueprints (all in-repo, this session)
- **dowiz current-state gaps:** `docs/design/gap-blueprints-2026-07-11/` G01–G13 + `MASTER-EXECUTION-PLAN.md` (Waves 0–4).
- **Local-first hub:** `docs/design/local-first-hub-2026-07-11/` — `SYNTHESIS.md`, `A`–`D`, `01`/`02` (concurrent session), `03`/`04` anonymity, `05` protocol-tech-completion, and the 6 layers: `layer-channel-adapters`, `layer-notifications-push`, `layer-sync-crdt`, `layer-reliability-observability`, `layer-migration-drop-cutover`.
- **Design:** `docs/design/dowiz-brand/INTERFACE-DIRECTION-2026-07-11.md` + `DESIGN-COMPLETION-BLUEPRINT-2026-07-11.md`.
- **GTM / EV:** `docs/research/2026-07-11-MAX-EV-SYNTHESIS.md` (+ 4 lenses), `-gtm-pipeline-completion.md`, `-launch-without-lawyer-albania.md`, `-relay-hetzner-tailscale-mesh.md`.
- **Assessments:** `docs/design/bebop-field-sim-2026-07-11/SYNTHESIS.md` (field-sim: parked), `docs/research/2026-07-11-honest-assessment-bebop-dowiz.md`.

## Live status (probed this session — may move; parallel session is editing code)
- `fix/prod-blockers-P2` @ `09dcbe05` **contains** the `/claim` SPA_ROUTES fix + G03 checkout enum/receiver fix + tests — **committed, NOT deployed**; `/claim` still 404s on prod AND staging. (It sits on paleo lineage → re-apply its diffs to the curated main-lineage PR, don't merge wholesale.)
- Staging OG card = 652,906 B, over Meta's ~600 KB / reliable-≤300 KB link-preview cap → WhatsApp likely shows a bare link. Needs recompress.
- 5 of 6 offer packets link to unbuilt demos (blocked on lost `PROVISION_OPS_SECRET`).
- Prod worker stopped since 07-03. Degrade-storm trap #15 still armed. `sw.js` served worker has no push handler. bebop-repo HEAD moved again (`84d5adc`); the crypto WIP is committed; wasm32 builds; 388/388 (now higher) tests green — but node/wire/storage/settlement still 0 lines; crypto still non-FIPS/unaudited.

---

## TIER 0 — NOW, zero pivot-risk (wanted under every future; parallel session executing)
No gate. Small, reversible, and they make v1 more stable for the live first client immediately.

| Item | Blueprint | VbM RED |
|---|---|---|
| Deploy the committed `/claim` + G03 fixes (they exist, undeployed) | GTM P1, G03 | `/claim` resolves; `kind=phone/signal/simplex` → 201; bad kind → 400 |
| Remove the 3 money-tween sites (`ClientLayout:245`, `Dashboard:451`, `Analytics:265`) | Design P1 | money never animates; a count-up assertion fails |
| P1 token-flip (bebop skin on admin/courier/404; spectral tokens) | Design P1 | contrast-audit AA; ramp-on-text = RED |
| Degrade-storm ratchet: boot-grace + real alert + restart-regression test | Reliability RG-1, G12 | flags reset on restart → test RED |
| `sw.js` push handler (served worker is push-deaf) | Notifications N0 | a sent push renders; silent-drop detected |
| gitleaks install + CI hard-fail; land the 3 gate diffs + P7 amendment; close stale GH #9 | G02 P1, G13 | canary secret fails scan; P7 RED on resurrected gate |
| OG card recompress <300 KB | GTM | content-length probe + WhatsApp paste-test |
| Restart prod worker; rotate `PROVISION_OPS_SECRET`; check worktree Supabase-cred rotation | G11, G12 | worker health 200; secret rotated |
| **Protocol library lanes** (no product dependency): W wire (iroh+WSS+framing), A capabilities, H crypto ladder start (Wycheproof) — distinct crate names (`bebop-wire`/`bebop-cap`, NOT `bebop`) | Protocol 05 (W/A/H) | tampered frame rejected; PQ-only-sig rejected |
| **Sync/CRDT fence gates** (CI: `sync-crdt` banned from domain/settle/dispatch dep-graph) | Sync Y0 | a price field in a MenuDelta → compile/CI RED |
| Channel: prod attribution READER ("Orders by channel" card) + QR+`?ch=` stamps | Channel P1 | `other`-spike detects a broken QR |

## TIER 1 — the prod vehicle (the curated main-lineage merge)
Gate: a rewrite-aware curated PR onto `origin/main` (G01/G02). CI auto-deploys prod on merge.

- **GDPR trio** (`5ded9f19`/`58caf4f4`/`d6b3473e`) **+ the AnonymizerService storage-DI fix** (else photo-purge no-ops) + webhook `secret_token` preflight. (G01)
- Re-applied `/claim` + G03 diffs (from `fix/prod-blockers-P2`, curated onto main lineage — not wholesale).
- Prod OG cards + prod demo provisioning (rides this vehicle). (GTM Wave 2)
- Then (scheduled, after this lands): the **remote-history scrub** + branch prune (G02 Wave 3), Actions disabled during the window, mirror-bundle first, push-loop paused.

## TIER 2 — quality-first stabilization → the "stable enough to send" bars
Gate: two concrete, mechanically-checkable checklists must go green before remote vendor outreach. This is the operator's quality-first line, made falsifiable (not a feeling).
- **Design "stable enough to send"** = the 13-item checklist incl. the storefront zero-diff Playwright gate (Design §5.7). ~7–10 sessions.
- **GTM "genuinely working to send"** = the 8-point per-venue gate (data-not-shell resolve, claim e2e, <300 KB unfurl + paste-test, channel render, freshness, ≤60s order-loop alert, honest-inert shadow, human checklist). Build the 5 missing demos (needs the rotated secret).
- Courier out-of-app signal (Notifications N1/N2) — so a real order actually reaches a courier.
→ **Then: first remote sends** (warm WhatsApp → referral → owner's Facebook page; no auto-send; venue-level data only).

## TIER 3 — validation (the hinge of the whole program)
`G11 GREEN` = one real order from a non-operator customer on a claimed venue. This single event gates everything below. Pre-committed RED: 0 claims after the defined contact set → stop/pivot (worth ~€1,800 in information). In-person walk-in comes when the operator can; remote outreach is the current path.

## TIER 4 — local-first substrate (gated on `G11 GREEN` + reliability gates)
Only after a validated order does substrate-rewriting earn its cost. Strangler, per-surface, reversible.
- **Protocol production:** R node runtime (`dowiz-node` single-writer sequencer = the kernel::decide-bypass fix made structural), X settlement/dispute (COD obligation ledger, counter-signed custody hand-offs, NO courier scoring — CI-enforced). (Protocol 05 R/X)
- **Migration ladder:** P2 SQLite read-replica dual-run (fold-parity byte-compare) → P3 menu/presence device-authoritative (reverse-projector keeps the pilot storefront unaware) → P4 money single-writer on vendor node (Postgres → alarmed read-only mirror = reversibility). (Migration-drop)
- **Channel registry** (`sales_channels` per-token, `/c/<token>` redirects) + **sync-crdt menu lane** (structure CRDT, price always vendor-signed). (Channel P3, Sync Y3)
- **Reliability gate LD0–LD11** must return GO before any venue cutover. (Reliability RG-4+)
- Each rung: entry precondition + VbM RED + reversible. P4 additionally gated on the crypto tier (below).

## TIER 5 — earn-it / future (each behind its own named gate)
- **Money-bearing bebop2 crypto** → gated on the **crypto audit ladder**: Wycheproof → FIPS re-derivation (ML-KEM NTT-domain, ML-DSA uniform-A + 48-byte challenge) → ACVP differential → constant-time (fix Ed25519 secret-branch + KyberSlash-class) → external audit (NLnet/NGI Zero path). Until then: **hybrid-only, audited classical half guards value.** (Protocol H, G09)
- **Messenger transport** (Telegram/WhatsApp/IG intake) → gated on the **G7 demand survey** + a red-line council; WhatsApp per-message pricing since 2025-07-01, inbound ≈€0. (Channel P4)
- **Astro/Svelte port** → gated on the arbiter doc confirming the rebuild + the FE-0.1 budget signature. (Design P5, G05)
- **Anonymous channel tier** (.onion vendor-node mirror + no-phone contact) → gated on the vendor node existing (Tier 4). (Anonymity 04, Channel P5)
- **Multi-venue mesh** (iroh-gossip, P5 decommission of Supabase/Node/Fly) → gated on all above soaking. (Migration P5)
- **P5 decommission:** signed pg_dump cold archive + staging drill with Postgres OFF through the reliability gate.

## TIER X — PARKED (dated, with re-entry criteria)
- **bebop as a protocol** (beyond the library lanes in Tier 0/4): capture-and-protect only (commit/push/one demo/memory bootstrap ≈ 1 hour). Not funded as a protocol until a product carries it. (Assessment §1)
- **Field-sim** for delivery: parked. Salvage heat-kernel/FFT/VSA(pow2) as libraries; fix the sign bug first if revived; the one survivor is a dev-tooling regression-radius, not couriers. (Field-sim SYNTHESIS)
- **Sovereign-core rebuild cutover** (G04): mothball default unless the full gate sheet is banked; keep only the kernel-honesty slice (= Protocol R). Renumber migrations 085→087–091 before any settlement work.
- **06-27 security trio + B3 RLS flip:** ledgered with review-by dates (G10); B3 NOBYPASSRLS flip last, red-line.

---

## The one-line spine
**Tier 0 makes v1 stable now → Tier 1 ships the prod truth (GDPR + fixes) → Tier 2 hits the two quality bars → Tier 3 is the first real order → and only then does Tier 4 rewrite the substrate.** Everything the project has built is real and mostly good; the entire edifice still pivots on Tier 3. Build downward from the order, not upward from the protocol.

*Assembled 2026-07-11 from this session's ~30 read-only blueprints. No code changed here. Gates,
not vibes: every tier boundary is a falsifiable condition, not a calendar date.*
