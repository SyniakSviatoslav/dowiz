# dowiz self-red-team runbook (operational)

> Repeatable procedure for the operator's **authorized self-red-team** of dowiz.
> Turns [`redteam-toolset-analysis-2026-07-02.md`](./redteam-toolset-analysis-2026-07-02.md)
> (the tool-selection analysis of record) into an ordered, checkable engagement.
> Read the analysis first — this runbook assumes its verdicts and the 12-rule tooling grammar.

---

## 1. Authorization + scope

**Authorization.** The operator owns dowiz's code and data and hereby authorizes red-teaming of
those assets only. No third party is targeted.

**In scope (own assets, application layer over HTTPS at own hostnames):**
- authz / RLS enforcement (cross-tenant, IDOR)
- injection defenses (SQL, parser/Zod boundaries)
- JWT handling (RS256 invariants — [ADR-0004](../adr/0004-owner-token-revocation.md),
  [ADR-0013](../adr/0013-courier-realtime-authz.md), [ADR-0003](../adr/0003-dev-login-fail-closed.md))
- WS-token exposure (`?token=` in URL history)
- auth rate-limiting / lockout

**Out of scope (NOT yours — third-party infra, off-limits by AUP + law):**
- Fly.io edge, Firecracker VMs, host/port surface, TLS termination
- Supabase Postgres host + their VPC/network
- Cloudflare R2 edge
- any network / host / wireless-layer scanning (Nmap, Metasploit, Aircrack — SKIP, see analysis)

**Operating constraints:**
- **Staging-first.** Run everything against `https://dowiz-staging.fly.dev`. Prod is never a
  red-team target.
- **Modest volume.** Keep request rate low (low `--risk`/`--level`, small threadcounts) so you
  do not trip Fly/Supabase abuse detection — that reads as attacking *their* platform.
- **App layer only.** If a technique's target is the network/host/edge, it points at a third
  party — stop.

**Ethics fence (charter-binding).** No person-profiling. **Maigret and person/social modules stay
OFF** — they turn a username into a cross-site dossier and violate the charter's
no-surveillance-for-harm / PII / data-sovereignty rules. Maigret may only ever be run by the
operator against the operator's OWN handles as a personal leak-check — never against a
customer/courier/prospect, never wired into the app, never touching platform data. Asset tools with
latent profiling capability (theHarvester email harvest, SpiderFoot person modules) run
**scoped to `*.dowiz.*` with person-modules disabled**.

---

## 2. The workstation — disposable Kali

Kali is a **throwaway container/VM spun up per engagement and destroyed after** — never 40 tool
installs on the dev box. This keeps offensive tooling isolated from the build/deploy environment
(no supply-chain bleed into the box that ships prod).

**Docker (fastest):**
```bash
# ephemeral shell; --rm destroys it on exit
docker run --rm -it kalilinux/kali-rolling /bin/bash
# inside: pull the metapackage subset you need (not kali-linux-everything)
apt update && apt install -y kali-tools-web sqlmap john seclists curl jq
```
- Burp Community is GUI — run it either via `kali-linux-headless` + a VNC/X forward, or (simpler)
  run **Burp Community on the host desktop** and keep only the CLI tools (SQLmap, JtR, curl,
  theHarvester) in the container. Either way the container is disposable.
- Persist nothing sensitive in the image; mount a scratch dir for findings:
  `-v "$PWD/redteam-out:/out"`.

**VM alternative:** official Kali VM image (VirtualBox/UTM), snapshot before the engagement, revert
after. Same disposability principle.

**Teardown:** `exit` (with `--rm`) or delete the VM snapshot. Nothing offensive survives on the
build host.

---

## 3. The first engagement (one afternoon, all free, staging)

Ordered. Each step lists the **question it answers**, the **dowiz surface**, and the **pass
criterion**. Any failure → §5 finding discipline.

### (a) crt.sh subdomain enumeration
- **Question:** what hostnames under `dowiz.*` actually exist — any forgotten staging/preview/dark
  subdomain that widens the attack surface?
- **Surface:** Certificate Transparency logs for `%.dowiz.*` (CT catches certs even for unlinked
  hosts).
- **How:** run `scripts/asset-surface-scan.mjs` (the codified crt.sh diff — see gap note if not yet
  present; interim one-liner: `curl -s 'https://crt.sh/?q=%25.dowiz.fly.dev&output=json' | jq -r '.[].name_value' | sort -u`).
- **Pass:** every returned host is a known, intended surface. Any unexpected host → inventory it,
  decide keep/kill, and (if it's a live app surface) run steps (b)-(d) against it too.

### (b) Burp Community + Autorize — cross-tenant / IDOR
- **Question:** can owner-B read or mutate owner-A's data through the app layer? (dowiz's recurring
  cross-tenant class — the same class ADR-0013 hardened for courier WS.)
- **Surface:** `/admin/*` UI flows + the API behind them (orders, menu, analytics, courier-invites,
  gdpr, menu-import). Seed two owners on staging (e.g. the `test@dowiz.com` fixture + a second
  seeded owner).
- **How:** proxy an owner-A session through Burp with **Autorize** loaded; supply owner-B's cookie
  as the low-priv identity. Browse every admin surface as A; Autorize replays each request as B (and
  unauth) and diffs. Green diff = properly denied; same-content = leak.
- **Pass:** every A request replayed as B returns 401/403/404 or B-scoped-empty — **never A's data**.
  Autorize shows no "Authorization bypassed!" rows. Autorize proves the app-layer gate; the RLS
  FORCE test (DB-owner track, §6) proves the DB backstop independently.

### (c) JWT Editor — RS256 invariant attacks
- **Question:** does the API accept a forged, downgraded, or revoked token?
- **Surface:** any owner/courier-authenticated endpoint; the RS256 verification path
  ([ADR-0004](../adr/0004-owner-token-revocation.md) 24h TTL + per-request `status='active'`).
- **How (each must be REJECTED):**
  1. **alg-confusion** — re-sign a valid token as HS256 using the RS256 **public** key as the HMAC
     secret.
  2. **alg:none** — strip the signature, set `alg:none`.
  3. **claim tamper** — flip `tenant`/`role`/`sub` to another owner.
  4. **revoked/expired owner token** — a token past its 24h TTL, and a token whose owner was
     revoked (`status != 'active'`) — must fail per-request even if not yet expired.
- **Pass:** all four return 401. None reaches business logic.

### (d) SQLmap — injection immunity + RLS non-bypass (authenticated)
- **Question:** is any parameterized input injectable, and if a probe *did* reach SQL, would RLS
  still contain it to the tenant?
- **Surface (staging, authenticated):** menu search/filter, order-lookup-by-id, owner analytics,
  and the `menu-import` path — the query-bearing endpoints.
- **How:** capture an authenticated request (Burp → save request file), then:
  ```bash
  sqlmap -r req.txt --cookie="<staging session>" --batch \
         --risk=1 --level=2 --threads=2   # modest volume — staging only
  ```
- **Pass:** SQLmap reports **no injectable parameter**. If it ever does surface data, that data must
  still be tenant-scoped (RLS non-bypass) — a cross-tenant read here is a 🔴 finding → council.

### (e) John the Ripper — one-shot argon2 param certification
- **Question:** are the argon2 cost parameters strong enough that known-weak passwords resist
  offline cracking?
- **Surface:** **sample dev/staging argon2 hashes only** of deliberately-weak test passwords.
  **NEVER prod hashes — that is the PII red-line.**
- **How:** feed the sample hashes + a small weak-password wordlist to JtR (offline, on the Kali
  container).
- **Pass:** JtR does **not** recover the passwords in a reasonable one-shot window → argon2
  cost/memory params certified. If it cracks them fast → raise the argon2 params (auth red-line →
  council before change).

---

## 4. Pilot tools — when to reach for each

| Tool | Reach for it when… |
|------|--------------------|
| **Param Miner** (Burp, Community) | hunting hidden params/headers or **cache-poisoning** on the API + SPA-proxy / `/s/:slug` path (that path has pool-starvation/caching history). |
| **Hackvertor** (Burp BApp) | fuzzing the **menu-import parser / Zod boundary** with encoding chains (multi-layer encode to slip past a naive validator). |
| **SpiderFoot** (dark, scoped) | a periodic wide sweep for leaked keys / exposed buckets / breach hits — **targets restricted to owned `*.dowiz.*`, person/social modules DISABLED**. Dark by default. |
| **Bambdas** (Burp, Community) | table-filter triage of proxy history (spotting anomalies fast). Full authz scan-check Bambdas need Burp Pro — Community gets filtering only. |
| **RSSHub / scout-feeds** | filling the plane-maintainer SCOUT hole (upstream dep releases / advisories). Start with **zero-dep GitHub `.releases.atom` polling**; only self-host RSSHub on Fly once the watchlist needs ≥~5 feed-less/non-GitHub sources. |

---

## 5. Finding discipline

A red-team finding is **not a special class** — it feeds the exact same self-improvement loop as every
other fix. For every confirmed finding:

1. **Red→green guardrail.** Write the deterministic artifact that fails on the vulnerable code and
   passes on the fix: a regression test, a `tools/eslint-plugin-local` rule, or a
   `scripts/plane-guard.mjs` check. Prove it **red before, green after**. A finding is not "done"
   without it. Never weaken an existing gate; never cheat green.
2. **Ledger row.** Add a row to
   [`docs/regressions/REGRESSION-LEDGER.md`](../regressions/REGRESSION-LEDGER.md) — the finding, the
   guardrail, red→green proof.
3. **Council before the fix (if serious).** If the finding touches **auth / RLS / money / PII**
   (🔴 red-line), convene the **Triadic Council** (Architect + Breaker + Counsel) to harden the fix
   plan (ADR + threat-model + counsel-opinion) **before** touching code. Reversible/cosmetic
   findings skip straight to fix + guardrail.
4. **Ship discipline.** Fix → commit (feature branch) → staging deploy → Playwright/proof, per the
   standing ship loop.

Red-team results = ordinary fixes with an offensive origin. Same ledger, same gates, same council.

---

## 6. Infra / DB later

**Self-host trigger.** Suricata (network IDS) and ELK/Loki (SIEM) have **nowhere to sit on managed
Fly** — their target is a network/host layer dowiz does not own today. The day dowiz **leaves managed
Fly for infra it controls** (the operator's "improve infra later" note):
- **re-open Suricata** — leading OSS network IDS pick, once there is a network you own to watch.
- **reassess ELK vs Grafana Loki** for centralized cross-service log search + alerting — trigger is
  when telemetry needs outgrow the current setup. This is the app-log/SIEM answer, **not** the
  network-IDS answer; a managed log sink likely beats standing up Elasticsearch for a small SaaS.

**DB hardening is a different track.** The DB-side items already staged from the security sweep —
SECURITY-DEFINER `search_path` pinning, `NOBYPASSRLS`, RLS `WITH CHECK` — belong to the
**DB-owner council track**, not this runbook. This runbook red-teams the app layer; those are
proactive DB invariants owned separately.

---

*Cross-references: [redteam-toolset-analysis-2026-07-02.md](./redteam-toolset-analysis-2026-07-02.md) ·
[ADR-0003](../adr/0003-dev-login-fail-closed.md) ·
[ADR-0004](../adr/0004-owner-token-revocation.md) ·
[ADR-0013](../adr/0013-courier-realtime-authz.md) ·
[REGRESSION-LEDGER.md](../regressions/REGRESSION-LEDGER.md)*
