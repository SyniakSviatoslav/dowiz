# Operator TODO — prod cutover (the human-only actions)

You approved the full prod flip + waived councils (probe-and-approve). Here is exactly what
must come from your side — the things I structurally cannot do or that carry irreversible /
credential / legal risk only you can accept. Everything else I'm executing autonomously.

Ordered by when it blocks.

## 1. 🔴 BLOCKS ANY PROD PUSH — Supabase credential rotation
`secrets-exposure-incident`: prod Supabase **superuser** creds are in git history (12 files /
4 commits). This does NOT block a prod *flag-flip* using Fly secrets (prod Node already runs on
those creds), BUT it blocks:
  - **merging `fix/audit-remediation` → `main`** if that pushes the branch/history to a remote
    that is (or will be) public (the ADR-020 open-source goal). Scrub history OR keep the merge
    on a private remote until scrubbed.
  - the open-source flip entirely.
**Action:** rotate the prod Supabase DB password + any exposed service keys, update the Fly
secrets on `dowiz` (prod) + `dowiz-staging`, then `git filter-repo`/BFG the 4 commits before any
public push. I cannot rotate your live DB creds or rewrite shared history.

## 2. 🔴 HARD MONEY GATE — the 085 settlement watermark (date-sensitive)
`packages/db/migrations/1790000000085` bakes a literal watermark `2026-07-10 00:00:00+00`.
Today is 2026-07-05. Applying 085 to prod is SAFE only while the watermark is **≥ the real prod
apply date** (a later watermark just defers pre-watermark rows to your backfill; an EARLIER one
DOUBLE-PAYS couriers). So: apply 085 to prod on or before 2026-07-10 with the literal untouched;
if prod apply slips past 2026-07-10, **bump all three literals in 085 first**. I will NOT apply
085 to prod without your go on the date.

## 3. Prod DB credentials for the prod Rust app
To stand up `dowiz-rust` (prod) I need the prod Supabase `DATABASE_URL_OPERATIONAL` /
`_SESSION` / `_MIGRATIONS`, prod JWT keypair (matching prod Node), R2, VAPID, Telegram, and
`PROVISION_OPS_SECRET`. I can read these from prod Node's Fly secrets IF you confirm I may
(`flyctl ssh console -a dowiz ... printenv`). **Confirm: may I read prod `dowiz` secrets to seed
`dowiz-rust`?** (Y/N). If N, set them on `dowiz-rust` yourself once I create the app.

## 4. Irreversible per-surface prod flips — explicit go
You said "approve full prod flip" — I'll take that as the standing go for S1–S4, S6, S7, S10.
For the two irreversible-money/PII surfaces I want ONE more explicit confirm because a bad flip
charges or erases real customers:
  - **S5 (orders/money)** — a prod flip routes real checkout/settlement to Rust. Confirm: flip S5 prod? (Y/N)
  - **S9 (GDPR erasure)** — a prod flip routes real Art.17 erasures to the Rust engine. Confirm: flip S9 prod? (Y/N)
(Both are flag-reversible in seconds, but the money/erasure they touch in the window is not.)

## 5. Phase-D decommission owner + date (REV-C10 HARD GATE)
The strangler's honest close needs a named human owner + a date to remove Node after the soak.
I cannot self-assign this. **Fill:** owner = ______, decommission date = ______.

## 6. S8 telegram webhook — set the secret + confirm the path-vs-header parity call
S8 is held on Node: the Rust webhook validates the header secret, Node validates the URL-path
secret. Confirm which is canonical (I'll align Rust to match), and set `TELEGRAM_BOT_SECRET` on
`dowiz-rust` (prod + staging). Until then S8's webhook stays on Node (its crons run dark, DB-safe).

## 7. Sign-offs already banked (no action unless you want to revisit)
S2 auth amendment §3, courier-deliver conflict-bool race (flagged, low-frequency), prod merges
of the 3 live fixes (#74 photo-purge, #75 telegram fail-closed, #76 GDPR fan-out — already
staging-validated, held for your prod-merge).

---

### What I'm doing autonomously in parallel (no action from you)
- Probe-and-approve porting the remaining ~58 staging routes surface by surface (council waived).
- Building + deploying the prod-flip mechanism to the boundary: prod Rust app creation is gated on
  #3; prod migrations on #2/#3; the flip on #4.
- Running the live-PG suite as each port lands (the ratchet), regenerating the keep-gate, updating docs.

I'll flip prod the moment #1 (or its private-remote workaround), #2 (date), #3 (creds), and the
#4 confirms are in. Reply inline (e.g. "3: Y, 4: Y both, 5: me / 2026-07-20") and I'll execute.
