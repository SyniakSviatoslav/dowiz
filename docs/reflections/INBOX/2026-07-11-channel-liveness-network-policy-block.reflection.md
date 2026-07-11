---
date: 2026-07-11
slug: channel-liveness-network-policy-block
surface: plane-maintainer / governance / cloud-env network policy
qualifies: "recurrent failure (3x) -> promoted to a new plane-guard soft check"
---

# Reflection: HEAL + Telegram channels silently dark on the cloud maintainer env — promoted to a deterministic check

## CONTEXT
Daily plane-maintainer run (`run-20260711T0603`), SENSE step. `pnpm verify:all --ci` and
`node scripts/plane-guard.mjs --staging` both reported 12/12 hard PASS with only the two
already-known soft warns (prediction backlog, inbox backlog). No hard fails to diagnose.

While preparing the HEAL step I found `node_modules` was missing (fresh checkout — ran
`pnpm install`, fine) and `flyctl` was not installed. `STAGING_DATABASE_URL`,
`FLY_API_TOKEN`, `TELEGRAM_BOT_TOKEN`, `PLANE_REPORT_CHAT_ID` were all present in env — so by
the charter's literal text ("if a required secret ... is absent ... do not attempt the
deploy") nothing said stop. But direct probes (`curl https://api.fly.io`,
`curl https://api.telegram.org/bot.../sendMessage`) both failed with `CONNECT tunnel failed,
response 403`, and `$HTTPS_PROXY/__agentproxy/status` confirmed it structurally:
`recentRelayFailures: [{kind: "connect_rejected", detail: "gateway answered 403 to CONNECT
(policy denial or upstream failure)", host: "api.telegram.org:443"}]`. Same for `api.fly.io`.

Checking `plane-telemetry.mjs inbox --json` showed this was NOT new: `run-20260707T0603`
recorded `telegram=failed:chunk_send` and `run-20260710T0603` recorded `flyctl absent; install
blocked by network policy (403 on fly.io/install.sh)`. Three runs, two independently-discovered
symptoms, one root cause, and until now zero deterministic artifact naming it — each run's
operator had to re-derive "oh, the network is blocked" from scratch via ad hoc curl.

## DECISIONS
- Did NOT attempt `flyctl install` / a deploy workaround — the charter's escalate-don't-route-
  around instruction for missing deploy capability applies in spirit even though the literal
  trigger text says "secret absent" (the secret IS present; the network path is not). Treated
  "required capability unreachable" as the same class as "required secret absent."
- Added `channel-liveness` to `scripts/plane-guard.mjs`, gated on `--staging` (matches the
  existing P8 live-drift-probe convention: live network checks are opt-in, never in default/CI
  runs). Soft-only — never blocks, only makes the degradation visible every `--staging` run
  instead of requiring a human/agent to rediscover it by hand.
- First implementation treated "fetch didn't throw" as reachable — WRONG. Verified empirically
  that Node's `fetch` (undici) surfaces the proxy's policy-denied CONNECT as a normal
  (non-throwing) HTTP 403 response, not an exception — so "didn't throw" alone under-reported
  the exact failure this check exists to catch. Caught this by testing directly with `node -e
  "fetch(...)"` before trusting the check, not by assuming the first version was correct.
  Fixed: treat a bare-403 response as unreachable (same fingerprint on two unrelated domains =
  proxy-level policy denial, not each service's own behavior).
- Proved red→green empirically (`git stash` the fix → 0 `channel-liveness` rows in `--staging`
  output, matching the silent failure mode of the prior 3 runs; restore → 2 named `⚠️` rows) —
  not just narrated it. Added ledger row #57.
- Committed on a feature branch (`plane-maintainer/channel-liveness-guard-20260711`), no
  staging deploy attempted (this is a governance-plane script, not part of the deployed Fly
  app — `pnpm verify:all --ci` full-green is this change's actual proof surface).

## WHERE
- `scripts/plane-guard.mjs` (+channel-liveness, `--staging`-gated, 2 soft checks)
- `docs/regressions/REGRESSION-LEDGER.md` (#57)
- `docs/governance/plane-status-2026-07-11.md` (today's digest, first one ever actually
  committed — no prior `plane-status-*.md` exists in git history despite the charter dating
  to 2026-07-02 and telemetry recording runs back to 07-02)

## WHY-causal
Root cause of the *silence*, not the network block itself (the network policy is presumably an
intentional environment-security default the operator set for this cloud checkout — not a bug
to fix, a boundary to respect and make visible). The causal chain: `plane-guard.mjs --staging`
already had exactly one live-network check (P8's DB drift probe) and it was scoped ONLY to the
migration head comparison — nobody extended the "live probe under --staging" pattern to the
OTHER two live-network dependencies the HEAL/REPORT steps actually need (fly.io, Telegram).
So each of those two channels had a single ad hoc, uncoordinated failure path (a `try/catch`
inside `plane-report.mjs`'s Telegram sender; a bare "command not found" for flyctl) with no
shared, named, cross-run-comparable signal. Three runs rediscovered the same fact three
different ways before anyone (any run) turned it into a check. This is the same shape as the
`*-liveness` family already in the file (telemetry/prediction/inbox/scout/health) — "a
producer that goes silent needs a consumer that says so," pattern #12 — it just hadn't been
applied to network egress yet.

## CONFIDENCE
High that the proxy CONNECT-403 is the correct read of the RED condition (fixture-level
red→green proof, not inference) and that this cloud env's fly.io/Telegram block is a policy
default rather than an outage (identical 403 fingerprint on two unrelated domains, at the
gateway CONNECT layer, per `__agentproxy/status`). Medium on whether the operator WANTS this
env to reach fly.io/Telegram at all — that is a deliberate call for them, not something this
run should second-guess or route around. Flagging via the digest + escalation, not assuming.

## NEXT-TIME
- When a charter says "if secret X absent, escalate," read it as "if capability X is
  unavailable, escalate" — a present secret with an unreachable network path is the same
  failure class, not a loophole.
- Before trusting a new reachability/liveness probe's polarity, empirically test the ACTUAL
  failure condition against it (don't assume "throws vs doesn't throw" maps cleanly onto
  "reachable vs blocked" for a proxied environment) — this cost one throwaway iteration here
  but would have shipped a check that silently says "reachable" while blocked, which is worse
  than no check (false green > no signal).
- `docs/governance/plane-status-*.md` had never actually landed a commit despite the charter
  being 9 days old — the REPORT step's "always commit the digest" half of pattern #11 was
  itself silently unexercised. Worth a librarian/ratchet look at whether other charter steps
  are similarly "documented but never actually completed end-to-end" (result-vs-expectation
  doubt trigger, one level up).

## LINK
- [[plane-maintainer-agent-2026-07-02]]
- [[memory-corpus-meta-patterns-2026-07-02]] (pattern #12 — silent producer/consumer gap;
  H3 — silence made visible)
- [[plane-telemetry-closed-loop-2026-07-02]]
