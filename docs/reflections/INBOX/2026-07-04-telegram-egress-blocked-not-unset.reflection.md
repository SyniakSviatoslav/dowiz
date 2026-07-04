CONTEXT:   Daily plane-maintainer firing (run-20260704T0602). SENSE step: `node scripts/plane-report.mjs
           --github-issue-on-fail` returned overall PASS (12/12 hard, 2 soft warns), but its own stdout
           ended with `[plane-report] telegram: HTTP 403`. TELEGRAM_BOT_TOKEN and PLANE_REPORT_CHAT_ID
           are both set in this cloud checkout's env — so this is not the "secret absent, skip cleanly"
           path the charter anticipated (plane-maintainer-agent.md reporting-channels §2: "the reporter
           skips this channel cleanly if unset").
DECISIONS: Did not retry, did not attempt to route around the block (curl'd api.telegram.org directly
           to confirm: `curl (56) CONNECT tunnel failed, response 403`; then checked
           `$HTTPS_PROXY/__agentproxy/status` — `recentRelayFailures` showed
           `connect_rejected … "gateway answered 403 to CONNECT (policy denial or upstream failure)"
           host: api.telegram.org:443`). Per this environment's proxy README: "403/407 … Do not retry or
           route around it — report the blocked host." Recorded telegram=blocked in today's digest
           instead of treating it as a code/config bug to fix.
WHERE:     docs/governance/plane-status-2026-07-04.md (digest); scripts/plane-report.mjs (telegram
           send path, unmodified); this reflection.
WHY:       Root cause is an environment-layer egress allowlist decision (this cloud session's org
           network policy does not permit api.telegram.org), not an application defect. The charter's
           "skips cleanly if unset" language only modeled ONE failure mode (secret absent) and left a
           gap for a second, distinct one (secret present, destination denied by the sandbox's own
           network policy) — a result-vs-expectation mismatch: expected "config problem, fixable in
           repo", got "environment policy, not fixable by this agent at all". Conflating the two would
           have tempted a wasted diagnose/heal cycle on code that isn't broken.
CONFIDENCE: high (directly reproduced via curl + the proxy's own status endpoint, not inferred)
NEXT-TIME: When a reporting channel fails, distinguish "secret unset" (skip cleanly, no digest noise)
           from "secret present but destination blocked by sandbox policy" (report the specific blocked
           host in the digest's channel-status line, do not retry, do not try alternate transports) —
           these need different digest language so a human doesn't waste time debugging a token that was
           never the problem. If this recurs across firings (see today's prediction, target
           telegram-egress), it's a candidate for plane-report.mjs to distinguish HTTP-403-from-proxy
           (network policy) vs HTTP-403-from-Telegram-API (bad token) in its own error message, since
           right now both look identical to the caller.
LINK:      docs/governance/plane-status-2026-07-04.md ; docs/governance/plane-maintainer-agent.md
           (reporting channels §2) ; [[plane-telemetry-closed-loop]]
