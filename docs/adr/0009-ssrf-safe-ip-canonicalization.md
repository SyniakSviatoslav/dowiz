# ADR-0009 — SSRF-safe IP canonicalization (mandatory fetch gate)

- Status: PROPOSED (design gate — ports red-team D1-F4 into the NEW architecture)
- Date: 2026-07-13
- Red-line: NET / APPSEC. Forward-only.
- Supersedes/relates: red-team `D1-appsec-authz.md` F4 (SSRF via IPv4-mapped IPv6), quick-win #10 (shared hook bundle so no route registers a fetch without the guard).

## Context
The legacy SSRF guard (`isPrivateIp` / `assertPublicUrl`) classified IPv4 numerically but only **string-matched** IPv6 (`::1`, `fc*`, `fd*`, `fe80*`, `::`). An IPv4-mapped IPv6 literal such as `::ffff:169.254.169.254` matched none of these, and because `net.isIP("::ffff:169.254.169.254") === 6` the guard skipped DNS resolution and trusted the literal — reaching the cloud **metadata service** and Fly 6PN internal hosts (D1-F4, HIGH, confirmed in code). Root cause: the guard trusted `isIP()===6` literals **without unwrapping the embedded IPv4**.

## Decision
ANY owner-side or node-side code that dereferences a URL/IP MUST canonicalize before classifying, via a **single shared, tested helper** — no route registers a fetch without it (the red-team's structural "shared hook bundle" fix, D1 quick-win #10):

1. **Unwrap IPv4-mapped IPv6:** `if (v.startsWith('::ffff:')) return classify(v.slice(7));`
2. **Reject** any `net.isIP()===6` value that embeds an IPv4 literal (`::ffff:` after step 1 means malformed → deny).
3. **Block** loopback, RFC1918, link-local (`169.254.0.0/16`, `fe80::/10`), and the cloud-metadata range (`169.254.169.254`).
4. **Resolve DNS, then RE-CHECK** the resolved IP against the same allowlist, and **pin** the resolved IP into the connection (undici dispatcher) to close the DNS-rebind TOCTOU.

## Alternatives considered
- **A — keep per-route `isPrivateIp` string matching (legacy):** REJECTED. Already bypassed by `::ffff:` mapping (D1-F4).
- **B — blocklist-only (deny known-bad ranges):** REJECTED. Incomplete; new ranges slip through.
- **C — canonicalize-then-allowlist + DNS-rebind pin (chosen):** closes D1-F4 and the TOCTOU at the architecture level, before the first owner/node-side fetcher is implemented in this repo.

## Consequences
- **+** SSRF class closed **before first implementation** — no route can fetch without the gate.
- **+** Red-team D1-F4 root cause removed by construction.
- **−** Every owner/node-side fetcher must route through the helper (no ad-hoc `fetch`).

## Open items / human decisions
- **None blocking.** Implement the helper when the first owner-side / node-side fetch lands; cover with unit tests for `::ffff:` mapping, metadata IP, and DNS-rebind pinning.
- **Proof (Mandatory Proof Rule):** falsifiable tests — `::ffff:169.254.169.254`, `::ffff:127.0.0.1`, and a DNS name resolving to a blocked range are all denied; a benign public IP passes and is pinned.
