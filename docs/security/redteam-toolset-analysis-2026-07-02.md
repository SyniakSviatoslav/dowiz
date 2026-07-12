# Red-team & tooling analysis for dowiz (2026-07-02)

Authorized self-red-team of the operator's OWN dowiz assets to harden app + infra + DB.
Analyzed against dowiz's real stack and its ethics charter. Verdicts apply the 12-rule tooling
grammar. Sources cited inline in the four source lanes; this is the synthesis of record.

## The governing frame (decides most verdicts)
dowiz owns **code and data, NOT infrastructure**. The network/host/edge layer belongs to Fly.io
(edge, Firecracker VMs) and Supabase (Postgres, their VPC); R2 is Cloudflare's. Therefore any tool
whose target is a **network/host/wireless layer** points at a THIRD PARTY (Fly/Supabase) — off-limits
by their AUP + law, regardless of intent. What the operator CAN legitimately red-team is the
**application layer** over HTTPS at their own hostnames: authz/RLS enforcement, injection defenses,
JWT handling, WS-token exposure, auth rate-limiting. Keep all testing on **staging**, modest volume,
to avoid tripping Fly/Supabase abuse detection.

## Tier 1 — ADOPT now (app-layer, own assets, maps to known dowiz weak spots)
1. **Autorize** (Burp ext, free on Community) — records an owner-A session, replays every request as
   owner-B/unauth, diffs responses. This is THE tool for dowiz's recurring **cross-tenant/IDOR** class
   (owner-revocation, ADR-0013). Autorize finds the app-layer authz gap; the RLS FORCE test proves the
   DB backstop. **Start here.**
2. **JWT Editor** (Burp ext, Apache-2.0, Community) — attacks the RS256 invariants: alg-confusion
   (re-sign as HS256 on the public key), `alg:none`, tampered tenant/role claims, and
   expired/post-revocation owner tokens (ADR-0004 24h TTL + per-request status='active').
3. **SQLmap** (GPL-2.0) — the one network/offensive tool that fits: proves injection immunity AND
   tenant/RLS non-bypass on your own API (menu search/filter, order-by-id, owner analytics,
   menu-import). Authenticated session via `--cookie/--headers`, staging only, modest `--risk`.
4. **crt.sh + theHarvester + SecLists** — asset-discovery core. crt.sh: CT-log diff of `%.dowiz.*`
   to catch forgotten staging/preview subdomains (best value-to-effort, one curl). theHarvester:
   domain-scoped subdomain/email/host enum. SecLists: inert wordlist fuel for fuzzing your own
   staging API + DNS brute + secret-pattern repo scan.
5. **Kali Linux** — ADOPT as a **disposable VM/container workstation** (`kalilinux/kali-rolling`),
   NOT 40 installs on the dev box (keeps offensive tooling isolated from build/deploy env).

## Tier 2 — PILOT / one-shot
- **Param Miner** (Apache-2.0, Community) — hidden param/header + cache-poisoning recon on the API +
  the public SPA-proxy/`/s/:slug` path (prior pool-starvation/caching history there).
- **Hackvertor** (free BApp) — encoding-chain multiplier for menu-import parser / Zod-boundary fuzzing.
- **John the Ripper** (GPL-2.0) — one-shot OFFLINE audit: run against sample dev/staging **argon2**
  hashes of known-weak passwords; it should NOT recover them → certifies the argon2 cost params.
  Never touch prod hashes (PII red-line).
- **SpiderFoot** (MIT, but OSS stagnant since v4.0/2022) — dark, scoped, periodic wide sweep (buckets,
  leaked keys, breach hits); person/social modules DISABLED; targets restricted to owned assets.
- **RSSHub** (MIT) — fills the plane-maintainer SCOUT hole (upstream dep releases / advisories).
  Start with ZERO-dep GitHub `.releases.atom` polling wired into SCOUT; self-host RSSHub on Fly only
  once the watchlist needs ≥~5 feed-less/non-GitHub sources.
- **Bambdas** (LGPL-3.0) — table-filter Bambdas triage on Community; full authz scan-checks need Pro.

## Tier 3 — PARK-with-trigger
- **Wireshark** (GPL-2.0) — only for WS-`?token=`-in-URL evidence / TLS checks, and browser
  devtools / Playwright network traces usually beat it. Trigger = need a raw-capture artifact.
- **THC-Hydra** (AGPL-3.0) — only to validate auth rate-limiting/lockout on OTP/login; prefer a
  scripted E2E rate-limit assertion (stays in-harness). Staging only, low volume.
- **Suricata** (GPL-2.0) — network IDS with nowhere to sit on managed Fly. Trigger = **dowiz
  self-hosts infra/network it controls** (the operator's "improve infra later" note). Then it's the
  leading OSS IDS pick.
- **ELK / Elastic** (tri-licensed SSPL/ELv2/AGPLv3 since 2024) — heavyweight for a small SaaS; a
  managed log sink (Fly logs → hosted SIEM, or Grafana Loki) beats standing up Elasticsearch. Trigger
  = centralized cross-service log search + alerting outgrows current telemetry. It's the app-log/SIEM
  answer, NOT the network-IDS answer.
- **Recon-ng** (GPL-3.0) — heavyweight/drifting; redundant with theHarvester + crt.sh for this surface.
- **EnIGMA / SWE-agent self-red-team** (MIT) — right idea (authorized self-pentest loop), wrong
  maturity (CTF-shaped, pinned to 0.7, own LLM budget, red-line-adjacent autonomy). Trigger = a
  dedicated gated pentest lane + maintained 1.0 + accepted cost line.

## SKIP for this project
- **Nmap / RustScan** — port surface is Fly's, not yours; use testssl.sh/SSL Labs for the one TLS
  question worth asking.
- **Metasploit** — host-CVE exploitation has no target; the only "hosts" are third-party managed infra.
- **Aircrack-ng** — no wireless surface in a cloud SaaS.
- **Snort** — dominated by Suricata; no reason to run both if you ever self-host.
- **Parrot OS** — redundant with Kali (pick-one-lane).
- **HelixDB** (AGPL-3.0, pre-1.0) — solution-seeking-problem; Postgres+pgvector already cover the
  memory-graph + RAG needs; a 2nd datastore fractures the single-store + RLS discipline.
- **SWE-agent as a coding agent** — duplicates the existing Claude Code harness, adds an LLM bill,
  sits outside the guardrails/ledger (don't-conflict-utilize).

## 🔴 Ethics line (charter-binding)
**Maigret is SKIP for the platform** — it is people-profiling (username→dossier across 3000+ sites).
Under dowiz's charter (no surveillance-for-harm, PII protection, data-sovereignty) it may ONLY be run
by the operator against the operator's OWN handles as a personal leak-check — NEVER against a
customer/courier/prospect, never integrated into the app, never touching platform data. The asset
tools carry latent profiling capability too (theHarvester email harvest, SpiderFoot person modules):
keep them scoped to `*.dowiz.*` owned assets with person-modules disabled.

## Suggested first engagement (all free, staging-only, one afternoon)
1. crt.sh diff `%.dowiz.*` → enumerate the real subdomain attack surface.
2. Kali container as the workstation; load Burp Community + Autorize + JWT Editor.
3. Autorize: owner-A vs owner-B replay across `/admin/*` + API → hunt the cross-tenant class.
4. JWT Editor: alg-confusion / none-alg / revoked-owner-token against staging.
5. SQLmap: authenticated, against menu-search/order-lookup/import endpoints → prove injection+RLS.
6. JtR one-shot: argon2 param certification on sample staging hashes.
Each finding → a red→green guardrail + REGRESSION-LEDGER row, same discipline as every other fix.
