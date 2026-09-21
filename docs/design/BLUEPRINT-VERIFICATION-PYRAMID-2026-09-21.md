# Blueprint — the verification pyramid, derived from thirty real defects

**2026-09-21.** Not a survey of testing practice. On this day the platform had
63 Worker unit tests, 265 hub unit tests, a green wasm build, a green design
gate and two green browser gates — **and thirty defects in production**,
including a customer charged the undiscounted price, a venue undercharged for
every paid option, an owner whose session silently moved to another restaurant,
and a storage error that would have made a venue's whole order log unreachable.

Every one of those suites was passing. So the question this blueprint answers
is not "what kinds of tests exist" but **"which layer was missing, measured
against defects that actually happened"**.

---

## 1. The research: what each defect needed

Thirty confirmed defects, each tagged with the cheapest layer that would have
caught it. This table is the whole argument; everything after it follows from
the counts.

| # | Defect | Would have been caught by |
|---|---|---|
| 1 | `courier_login` picks the venue with `LIMIT 1`, ignoring the Host | pure rule test |
| 2 | `owner_login` the same | pure rule test |
| 3 | `owner_refresh` the same — one refresh moved the session to the other venue | authenticated integration probe |
| 4 | `authenticate` asked "owner of anything", so a removed owner kept 24 h of access | integration (needs memberships) |
| 5 | An API key's venue scope dropped in `owner_at` | integration |
| 6 | `write_translations` validated ids against the token's venue, not the authorised one | integration |
| 7 | Promo discount applied to the stored order but not to the Stripe intent | pure rule test / contract |
| 8 | Modifier prices never charged; unknown options accepted | pure rule test |
| 9 | `Storage::get` error folded to "no image" → the log becomes unreachable | fault injection |
| 10 | `courier::accept` had no status gate — any order, any state | E2E |
| 11 | `delta` deleted `payment_intent`, `amount_received`, `stripe_event`, `crypto` | round-trip property |
| 12 | `delta` would have deleted a line's new `name` on the first transition | round-trip property |
| 13 | **An order line never carried the dish name** — every ticket read `1× item-05` | **browser E2E asserting correctness** |
| 14 | A collection order had no terminal state | E2E |
| 15 | A collection order was offered to couriers as a delivery | E2E |
| 16 | `?user=` still client-chosen behind `principal_at` — read anyone's wallet | pure rule test |
| 17 | The kit's wallet/chat answered 401 for a day | E2E on a surface no gate drove |
| 18 | The service worker cached the document and none of its 14 modules | offline E2E |
| 19 | A 401 storm wiped the courier's login form every 12 s | E2E with a dead token |
| 20 | A half-open socket suppressed polling for 90 s; no offline banner for 15 min | network fault injection |
| 21 | Backups keep 8 copies, not "7 days + 4 weeks" | **simulation** test (the existing unit test fed a year of keys at once — a state no nightly run ever sees) |
| 22 | `worker_errors` has never received a row; `sqlite_sequence` proves it | instrument-fires test |
| 23 | `ai.endpoint` defaulted to a loopback URL a Worker can never call | config/contract test |
| 24 | Hibernated sockets outlive a revoked principal | integration |
| 25 | The catch-up window holds one change; `RECENT_KEEP = 256` is dead | integration |
| 26 | `chain_check` is called by nothing in production | architecture test |
| 27 | CSP `frame-src` lacks `hooks.stripe.com`; 3-D Secure would fail | config test |
| 28 | A hard-coded English `min` beside a translated one | i18n test |
| 29 | A failed ORDER poll toasted "the menu did not load" | i18n/UX test |
| 30 | `.gitignore` swallowed a move; a commit said "kept" and deleted three files | repo hygiene test |

**The counts, which are the finding:**

```
browser E2E asserting CORRECTNESS ....... 7
authenticated integration probe ......... 6
pure rule test .......................... 5
fault injection (offline / 500 / storage) 4
config & contract ....................... 3
round-trip property ..................... 2
simulation .............................. 1
instrument-fires ........................ 1
architecture ............................ 1
```

**Two conclusions.**

**(a) The missing layer was "is it right", not "does it respond".** Thirteen of
the thirty needed a test that drove a real client or a real authenticated call
and then checked the ANSWER. The existing browser gates checked that pages
render and fit a phone — both valuable, both blind to `1× item-05`.

**(b) Unit tests were not under-written, they were mis-aimed.** The five pure
rule defects all lived in decision logic that was tangled into an async handler
where no test could reach it. Each became testable the moment it was lifted into
a function of its inputs — `venue_for_login`, `wallet_who`. That is the cheapest
change in this document and it is an architectural one, which is why the
[modular architecture blueprint](./BLUEPRINT-MODULAR-ARCHITECTURE-2026-09-21.md)
is its sibling.

---

## 2. What the industry does with the same problem

Two fields solve this harder than web services do, and both are worth stealing
from rather than admiring.

**Logistics (WMS/TMS).** The discipline is the *conservation audit*: the system
is not asked "did the API return 200" but "does the ledger still balance". This
codebase already has it in two places and does not use it in a third:
`money.rs` nets refunds to exactly zero through double-entry, and
`stock.rs` folds a level from its events rather than storing a counter —
`StockLedger::stranded()` is precisely a conservation report. The gap is that
nothing runs it as a gate. **Adopt: a conservation gate that runs after every
E2E and fails on a non-empty `stranded()`, an unbalanced journal, or an order
whose fold disagrees with its stored status.**

**Gamedev.** Three practices transfer directly:
- **Deterministic replay.** A recorded session replays to a bit-identical state.
  The kernel is already built for it — no clock, no RNG, no float on the decision
  path (MANIFESTO C2) — so an order log IS a replay tape. **Adopt: a replay gate
  that folds every archived log and compares to the stored status.**
- **The soak test.** Leave the thing running for hours and watch for drift.
  This is what would have caught the backup retention bug: 120 simulated nights
  rather than one year of keys handed over at once.
- **Fault injection as a first-class mode.** Games test disconnects, packet loss
  and hitching deliberately. Four of the thirty were exactly this class, and the
  product's own design invites it: polling is never switched off, so a broken
  socket is invisible by construction.

---

## 3. The pyramid this product should have

Five layers. Each names the defects it owns from §1, so its value is arguable
rather than assumed.

### L1 — Pure rules (fast, no I/O) · owns #1, #2, #7, #8, #16
Every decision that can be written as a function of its inputs, lifted out of
its handler. **Rule: a `match` that chooses between tenants, prices, or
permissions may not live inside an `async fn`.** Existing: `venue_for_login`,
`wallet_who`, the kernel's money and FSM laws, `modifiers::price`,
`promo::redeem`. Target: 85 % line coverage on money, stock and auth decisions —
the three where a wrong answer costs real money.

### L2 — Round-trip and conservation properties · owns #11, #12, #21, #26
Not example tests; laws.
- `delta(a, b)` then apply to `a` equals `b`, for every key any writer produces.
  **This one law would have caught #11 and #12 outright.**
- `fold(events)` equals the stored status, for every archived log.
- `stranded()` is empty after any completed order lifecycle.
- The ledger balances after any sequence of top-ups and spends.
- `chain_check` runs over the live log in the nightly job, not only in tests.

### L3 — Authenticated integration against a real hub · owns #3, #4, #5, #6, #24, #25
The layer with the worst ratio of cost to defects found — six defects, all
tenancy, none reachable from a unit test because they need D1 and a Durable
Object. Already present as `flows-owner.mjs` and `_security_live.sh`.
**Rule: every route that resolves a venue gets a two-venue probe** — the owner
of both venues, on each host, asserting which one the write landed in.

### L4 — Browser E2E asserting correctness · owns #10, #13, #14, #15, #17, #19, #29
The biggest bucket and the one that did not exist this morning. The standard is
not "the pane drew something" but "the pane drew the right thing":
`journey.mjs` asserts the dish NAME, the quoted price, the exact grams that left
the shelf, and the status each of three people sees.
**Rule: every surface a human touches gets a cycle test, and every cycle test
ends by putting the venue back.** Coverage is counted in FEATURES walked, not
pages loaded.

### L5 — Fault injection and soak · owns #9, #18, #20, #22, #27
- Offline: an installed app must open its menu and show the venue's phone.
- A dead socket: the client pings, and a socket that misses two is closed.
- A storage error: `Storage::get` returning `Err` must not read as absence.
- An instrument that never fires is a broken instrument: assert
  `sqlite_sequence` moves for `worker_errors`.
- Config contracts: the CSP, the cron expression, the default settings.

---

## 4. Non-functional: what to measure and what to gate

Money and bytes, not adjectives. The hub cost blueprint already established the
per-order numbers; this makes them a gate.

| Budget | Today | Gate |
|---|---|---|
| Wire bytes per delivered order | ≈1.2 KB (v2 + deltas) | fail above 2 KB |
| Requests per order, all surfaces | to be measured | fail above the recorded baseline + 20 % |
| Order-log image growth per order | ≈21 cells | fail on any increase |
| Worker CPU per request, p95 | to be measured | fail above 50 ms |
| Storefront first paint, phone | to be measured | fail above 2.5 s |
| Memory: hub image at rotation | 15,360 cells ceiling | fail above 80 % without rotation |

These are the subject of the third document,
[the evals blueprint](./BLUEPRINT-EVALS-TRAFFIC-MEMORY-LATENCY-2026-09-21.md).

**Security and privacy gates** (the PCI-DSS/GDPR shape, sized to this product):
card data never touches this system — Stripe's Payment Element holds it, which
is a design decision worth a test that asserts no PAN-shaped string ever reaches
a log or a D1 column; the customer's phone is stored encrypted and surfaced only
through `customer_key`, with every reveal audited, so the gate is "no route
returns a raw phone without a reveal row".

---

## 5. What is deliberately NOT in this pyramid

- **Visual regression by pixel diff.** The design gate reads the linked CSS and
  the render gate counts applied rules; pixel diffing a hand-built interface on
  three viewports is a maintenance tax this team cannot pay yet. Revisit when
  the design system is frozen.
- **A test runner dependency.** `@playwright/test` is a supply-chain entry and
  this repo requires a decart comparison for one. The gates use the `playwright`
  library and plain assertions, which has cost nothing so far.
- **Mocked integration tests for Stripe/Meta/S3.** A mock of a provider tests
  the mock. These stay as live contract probes against sandbox credentials.

---

## 6. Order of work

1. **L2's delta law**, because it is one property and owns two shipped defects.
2. **L4 cycle tests for the remaining features** — reservations, threads,
   wallet, promotions, menu editing, couriers, settings, the platform hub.
   One file per feature, the shape of `journey.mjs`.
3. **L5's fault-injection suite**, starting with offline and the dead socket.
4. **L3's two-venue probe** applied to every venue-resolving route.
5. **L1 coverage measurement**, which is the only item here that needs tooling
   this repo does not have.

Each step lands with the defects it would have caught named in its commit, so
the pyramid keeps being argued from evidence rather than from shape.
