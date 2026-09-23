# dowiz

**One system for a restaurant: ordering, the dining room, delivery, the till.**
Decentralised and local-first, with one self-contained **bebop** image store per venue and a
deterministic Rust kernel that decides every order and every amount.

**Early access — launching across Albania; first venue live in Durrës.** Hosted at `dowiz.org`
with a subdomain per venue. Open source under AGPL-3.0 (`LICENSE`; trademark and DCO terms in
`TRADEMARK.md` and `DCO`).

dowiz gives a venue its own app on its own domain — menu, checkout, live courier on the map — plus
the room (tables, bookings, staff, amend, pay), a courier app that works underground, integer-exact
money and VAT, a consent-first customer record, and an owner console. Every venue's data lives in
its own append-only, content-chained store; no shared database, no SQL, no server that everything
stops without.

## Why dowiz

- **Against the aggregators.** Four of them compete in Albania — Wolt, Glovo, Bolt Food, Baboon —
  each taking a percentage of every order and owning the customer. dowiz is priced on `dowiz.org`
  at $50 a month per venue with no percentage of the bill and no delivery tariff; the venue keeps
  its domain, its brand and its customers.
- **Beside the fiscal POS, not instead of it.** Albanian fiscalisation (Law 87/2019) is
  certification-gated and 53 producers hold the certificate; dowiz does not, so today it runs the
  room, the ordering, the stock and the customers next to the venue's certified fiscal app, and
  reads that app's sales into one hub (`workers/api/src/ebills/`). Law 79/2025 makes card terminals
  mandatory for every business by 31 December 2026, so every venue is getting a second screen.
- **The market is mass, not a dozen.** About 34,000 accommodation and food businesses in Albania
  (an estimate: 14.3 % of 237,881 active legal units), and 12.5 million foreign arrivals in 2025.
  Sources and the country-by-country sequence (Albania → Kosovo → Montenegro → Serbia) are in
  `docs/design/BLUEPRINT-PLATFORM-TOP-TIER-2026-09-23.md` §6.
- **Built to be trusted.** Nobody is scored or ranked; money never touches a float; every rule that
  matters is a gate in CI, and the register is audited nightly by nine conservation laws.

## Traction, exactly

- **One live venue:** Dubin & Sushi, Durrës — `sushi-durres.dowiz.org` (the same restaurant also
  answers at `dubin-sushi.dowiz.org`). 165 dishes in three languages, delivery, pickup and dine-in,
  a courier on shift, cash payments; card payments switch on when the venue supplies its Stripe keys.
- **Waitlist open** at `dowiz.org`: "leave the venue's email and we will write when the next hub
  opens." No newsletters.
- Live platform last deployed 2026-09-22 and verified by reading it back; the conservation audit
  runs against the real venue.

## What it does today

Describes the tree at commit `53bd9573` (2026-09-23). Work in flight is under **Status**.

**Ordering and storefront** (`workers/api/public/store/`) — menu, basket, checkout, live tracking
with a map and a kernel-computed ETA in integer minutes (`workers/api/src/eta.rs`); sq / en / uk;
an installable PWA whose service worker caches the shell only. Delivery, pickup and dine-in as a
closed set of fulfilment kinds; one pricer for preview and checkout; a twelve-status order FSM where
`decide → Event`, `state = fold(events)`, and an illegal transition is an error
(`crates/dowiz-core/src/order_machine.rs`). Live updates over one WebSocket per client, polling as
the fallback (`workers/api/src/live.rs`).

**The room** (`workers/api/src/command/`, `crates/dowiz-hub/src/tables.rs`) — tables as data
(zones, seats, availability per time slot with a 90-minute dwell, double-booking refused) and
bookings from the storefront (`workers/api/src/booking.rs`). A dine-in order is placed at a table;
the sitting is a projection of its rounds, never a second record (`command/sitting.rs`). Amend in
intent form — add, remove, re-count, comp — versioned by `base_seq` so two tablets commute and a
stale edit is refused (`command/amend.rs`). Pay: one signed `Paid` event per payment, split across
as many payments as the bill takes, over-payment refused, a settled bill no longer amendable
(`command/pay.rs`). Staff as a fourth principal beside owner, courier and customer, carrying a
closed set of signed capabilities — `Advance`, `TakeOrders`, `TakePayment`, `Void`, `OpenTill` —
with presets Owner / Kitchen / Counter-Manager / Waiter (`crates/dowiz-hub/src/caps.rs`); the
console invites, re-roles and suspends staff.

**Delivery** (`workers/api/public/courier/`) — one job on screen, cash handover, voice prompts.
Taps made without signal queue in IndexedDB and replay with an `Idempotency-Key`; the server answers
a replay with the first call's result (`workers/api/public/lib/outbox.js`,
`workers/api/src/idempotency/`). Reopening the app underground still shows the job. A courier's
login is scoped to the host's venue.

**Money and tax** (`crates/dowiz-core/src/money.rs`, `crates/dowiz-core/src/tax.rs`) — integer
minor units only: currency-typed, overflow-checked `i64`/`i128`; ALL, EUR and USD with their minor
units from the kernel. VAT per line as an integer rate in parts per million (`RatePpm`), inclusive
or exclusive with the rounded quantity named, a dated rate schedule so an old receipt shows its own
rate, the tax block computed once at placement; the arithmetic is generated from the equation by
`tools/eqc-rs` and pinned by test to the hand-written law. Card payments through Stripe
PaymentIntents — card data goes browser-to-Stripe and no type in the Worker can hold a PAN
(`workers/api/src/stripe.rs`).

**Customers** (`crates/dowiz-hub/src/consent.rs`, `crates/dowiz-hub/src/forget.rs`) — consent as
an append-only log of who, when, by which channel and which wording; withdrawal is a new record; a
marketing send needs a `Consented` value only the consent fold can produce. Right to be forgotten
in place: personal fields emptied while the record keeps its id and `prev`, so the chain and last
night's witness still verify, declared by a `Forgotten` event. The customer card holds only what a
fold cannot derive; every look at a customer's contact details is itself a logged event.

**Owner console** (`workers/api/public/admin/`) — orders, menu and media, couriers, customers,
staff, stock and recipes, analytics in the venue's timezone; three languages. Integrations, each
with a "prove it" check (`workers/api/src/integrations.rs`): a venue-owned Telegram bot for the
kitchen bell, WhatsApp and Instagram through one Meta webhook, a nightly copy to a bucket the venue
owns (any S3-compatible store), and the hub as an MCP server at `/api/mcp` so an agent can do
exactly what the console can. `GET /api/owner/health` reads image gauges against their real
ceilings; backups carry a manifest with each image's SHA-256.

## Architecture

```
browsers (store / admin / courier / kit / platform — vanilla JS PWAs; vocabulary generated from the kernel)
        │  HTTPS, one venue per Host header
        ▼
Cloudflare Worker  workers/api  (Rust → wasm32)
        │  one Durable Object per venue: class HubImages, id = venue id
        ▼
Durable Object  workers/api/src/hubdo.rs — the venue's bebop images in memory, single writer,
        │  96 KiB chunks, meta written last: log · catalog · settings · posts · stock · people · consent · bookings · ledger · …
        ▼
crates/dowiz-hub    pure folds over the images: order log, tables, caps, consent, forget, stock
crates/dowiz-core   the kernel: order FSM, money, tax, PQ primitives   (kernel/ is its std facade)
crates/bebop-store  the image format, zero dependencies   ·   crates/bebop-wasm  the same reader in wasm32
bebop-lang/         the language, its self-hosting compiler, and the store format's origin
```

`tools/native-spa-server` is a second, native implementation of the owner / courier / storefront
routes for a venue running on its own machine; CI tests it. The mesh transport (`bebop2/`,
`mesh-adapter/`) is not part of the live product.

**Design invariants** (`DECISIONS.md` D0, in priority order) and what holds each today:

| Invariant | Here it means | Held by |
|---|---|---|
| decentralised | one object and one image set per venue; no shared database, no SQL | gates `no-sql`, `one-venue`, `one-image` |
| local-first | the kernel is pure (no clock, RNG, network or float on the decision path) so any node replays the same fold; offline writes queue and replay | gates `clock`, `idempotent`; CI job `offline-writes` |
| post-quantum | ML-DSA-65 and a hybrid KEM exist in the kernel; not on the live wire (see Security) | principle |
| crypto | content-chained records, a nightly witness of every log's tip, fixed-algorithm tokens | tests; no gate |
| mesh | store-and-forward transport designed in `bebop2/`; not wired into the product | principle |
| reliability over latency | a refusing FSM, append-only logs that grow rather than refuse, replays answered idempotently | gate `idempotent`; conservation laws |

Two product red lines sit beside them: money is integer (`float-money`), and trust is a signed
capability, never a score — no rating, ranking or tier of any participant (`no-scoring`; the routing
and capability enums deliberately omit `Ord`).

## bebop: the engine, and what it buys this product

bebop is the repository's own language and storage format (`bebop-lang/`). The product uses the
**format**; the language runs beside it, not inside the Worker.

**The language.** Small and integer-only, compiled straight to AArch64 machine words by
`bebop.bin`, which is itself written in bebop (`bebop-lang/bebop.bp`) and loaded by a 1.5 KB
assembly seed (`bebop-lang/seed/seed.S`); no C in the toolchain, no dependency outside the tree.
The compiler compiles itself to a byte-exact fixpoint (gen3 == gen4, re-checked on every codegen
commit — `bebop-lang/ROADMAP.md`, TG-DONE row 2). Correctness rests on oracles, not on itself: 125
golden gates in `bebop-lang/bench/vs_rust/std_golden.sh` (counted 2026-09-23), each with an
independent Python oracle under `bebop-lang/bench/oracles/`, construct-parity freezes, and a
generator-vs-oracle fuzzer judged by `bebop-lang/tools/bpref.py`.

**The format** (`bebop-lang/selfhost/prelude/store.bp`; Rust side `crates/bebop-store`):
- **Pointer-free, self-describing.** Two CRC-checked superblocks (a reader takes the newest valid
  one, or nothing); an append-only arena of objects whose header carries layout digest, length,
  payload CRC and generation; every reference is an object-relative offset. Nothing in the file is
  an address, so the same bytes are read by bebop, Rust, the wasm32 module and a Python oracle.
- **Four readers that must agree, on every push.** CI job `bebop-wasm` runs
  `crates/bebop-wasm/gate.sh`: `bebop.bin`, the native crate, the wasm32 module under Node and
  `oracle.py` fold one fixture to one root; `--prove` flips a bit and the number moves; a cut image
  must be refused by all four. The second reader found a real defect on day one — a truncated
  image read in Rust as the previous generation (`3dbccd22`). The wasm32 reader is 30,863 bytes, a
  ratchet (`crates/bebop-wasm/bytes.baseline`).
- **An event log with O(1) append.** One object per event, root relinked, no rewrite of the world
  (`crates/bebop-store/src/evlog.rs`); v2 packs eight payload bytes per cell — measured 345 → 54
  cells for a 330-byte event. Errors, messages, bookings and postings use the same shape.
- **Tables without a query planner.** Bounded sets (people, dishes, keys) live in a sorted
  key/value image where every access path is a key written on purpose and each record owns its
  index keys, so record and index change in one operation (`crates/dowiz-hub/src/table.rs`). A key
  is venue-scoped by construction — the image *is* the venue — which closed the "wrong venue by
  default row" class of defect.
- **Content-chained, witnessed, erasable.** Every order-log id commits to the id before it;
  `Hub::chain_check` classifies each record as chained, legacy, redacted or broken. The tip is
  written nightly to a different object and to the venue's off-site copy, so a truncation — which a
  chain cannot see — contradicts something (`workers/api/src/witness/`). A person can be forgotten
  in place without moving the tip.
- **Every number in an image is a claim.** Lengths, counts and offsets are bounded against the
  bytes present; a slice that does not fit is refused, never truncated; a log whose root claims more
  records than its chain delivers is `Truncated`.

**What it replaced, and what it costs.** The D1/SQL layer was removed on 2026-09-22 (`cf834d24`):
0 prepared statements, 0 bindings, 0 migration files, held by `tools/gates/no-sql.sh`. One venue
costs $5.93 a month all-in on Cloudflare (the account's Workers subscription plus the domain), a
marginal delivery about $0.0006 — measured 2026-09-20,
`docs/design/BLUEPRINT-HUB-COST-AND-ORDER-LOG-2026-09-20.md`.

**Limits, stated.** One writer per venue by design (the Durable Object); a second writer that
cannot share the object — an offline till — is the condition under which CRDTs get re-examined.
Append logs double when full; compacted key/value images (catalogue, settings, posts) have a real
ceiling of 1 MiB. Reading one order walks the chain, O(n) in the log, and the whole image is loaded
per request; the live audit reports the venue at 619 log cells per order against a budget of 120,
unresolved. `bebop.bin` is AArch64-only, so on x86 CI that reader prints NOT MEASURED and the gate
relies on the other three. No bebop code executes in the product: `bebop.bp` has no wasm backend,
and the format is the boundary (`docs/design/BLUEPRINT-BEBOP-IN-WASM-2026-09-23.md`).

## Security

- **Post-quantum primitives, precisely.** `crates/dowiz-core/src/pq/` holds a from-scratch
  ML-DSA-65 (FIPS 204) verified byte-exact against the vendored NIST ACVP vectors (`pq/kat/acvp/`,
  one `#[test]` per vector), X25519 KAT-gated against RFC 7748, a hybrid X25519 + ML-KEM-768 KEM
  that refuses a classical-only fallback, and AES-256-GCM envelopes. The ML-KEM-768 module states
  in its own header that its ACVP gate is deferred and it is **not FIPS-203-conformant**. All of it
  sits behind the kernel's off-by-default `pq` feature; the live hub's tokens are HMAC-SHA256 with
  the algorithm fixed in code (`crates/dowiz-hub/src/token.rs`, `workers/api/src/auth.rs`).
- **Capabilities, not scores.** No participant is rated, ranked or tiered;
  `tools/gates/no-scoring.sh` refuses the vocabulary and the enums are not orderable. Red-line
  capabilities deny by default (`crates/dowiz-core/src/ports/agent/scope.rs`).
- **Tenant isolation.** The venue is decided once per request from the Host header;
  `tools/gates/one-venue.sh` refuses a handler that authorises one venue and acts on another.
- **No unauthenticated write route** after the 2026-09-21 red-team pass (`docs/red-team/`); the
  post-deploy smoke test no longer places an order for that reason.

## Correctness: verified, not claimed

Every gate is a ratchet that may only fall, and each was made to fire in both directions before it
was committed. Run: `for g in tools/gates/*.sh; do sh $g; done; python3 tools/gates/unreached.py`

| Gate (`tools/gates/`) | Holds |
|---|---|
| `no-sql` | SQL and the D1 binding cannot come back |
| `clock` | the Worker reads the time in one place; handlers take the instant |
| `one-image` / `one-venue` | one image written per transaction; one venue per request |
| `vocab` / `vocabulary` | the browsers' status and currency lists are generated by `tools/gen-vocab`, never retyped |
| `idempotent` | every route a phone can replay names the server guard |
| `paths` | `CLAUDE.md`, `AGENTS.md`, `DECISIONS.md`, `CONTEXT-INDEX.md` cite only files that exist |
| `unreached` | nothing public that no shipping code calls |
| `file-size` | no serving file over 300 lines without a split (`tests.rs` exempt) |
| `float-money` / `no-scoring` / `consent` / `record` / `tap-size` / `event-kinds` | the red lines above, 44 px controls, one event-kind set in Rust and JS |

**Conservation laws** — `e2e/gates/conservation.mjs`, read-only against the live venue with an
owner token; its proof `conservation.prove.mjs` runs in CI against a stub. 1 folded status equals
served status · 2 nothing is held by an ended order · 3 an order's money is lines plus fees minus
discount · 4 every delivered order's cash is accounted to a courier · 5 image gauges read under
their ceiling · 6 nothing is quarantined and the log's claim equals served plus withheld · 7 the
nightly witness is not contradicted and is still being taken · 8 every projection rebuilds from
the log to what is served · 9 the tax block conserves money and every stamp re-derives.

CI (`.github/workflows/ci.yml`): per-crate tests for `kernel`, `engine`, `apps/courier`,
`crates/bebop-store`, `crates/dowiz-core`, `crates/dowiz-hub`, `workers/api`,
`tools/native-spa-server`, `tools/gen-vocab`; the gates above except `record` and `event-kinds`;
`cargo deny`; the conservation proof; kernel mutation and fuzz jobs; the `bebop-wasm` four-reader
gate; and `offline-writes`, which drives the courier's outbox and cold start in a real browser.
Test counts on 2026-09-22: `workers/api` 221, `dowiz-hub` 317, `dowiz-core` 3,647,
`native-spa-server` 166.

## Repository layout

| Path | What it is |
|---|---|
| `workers/api/` | the Cloudflare Worker (Rust), its Durable Object, and `public/` — the five browser surfaces |
| `crates/dowiz-core/` | the kernel: order FSM, money, tax, PQ primitives, ports |
| `crates/dowiz-hub/` | one venue's images as pure logic: order log, tables, caps, consent, forget, stock, table |
| `crates/bebop-store/`, `crates/bebop-wasm/` | the bebop image format in Rust (zero dependencies) and its wasm32 reader with the four-reader gate |
| `kernel/` | `dowiz-kernel`, the std facade over `dowiz-core`; benches, fuzz targets, examples |
| `bebop-lang/` | the bebop language: compiler, seed, standard modules, oracles, benchmarks |
| `tools/` | `gates/`, `gen-vocab/`, `native-spa-server/`, `eqc-rs/`, `platform/attach-host.sh` |
| `e2e/` | live gates, kit regression, journeys, chaos and visual suites |
| `engine/`, `apps/courier/` | a dependency-free field-UI render engine and a wgpu courier surface; tested in CI, not in the live product |
| `bebop2/`, `mesh-adapter/` | the mesh delivery protocol and its kernel adapter; designed, not deployed |
| `docs/design/` | roadmap and blueprints; `docs/adr/` decisions; `docs/red-team/` findings |
| `scripts/` | `verify-hub.sh`, `verify-kernel-engine.sh`, `build-kernel-wasm.sh`, older tooling |

## Build, test, deploy

There is **no cargo workspace**. Enter each crate: `cargo -p` or `--manifest-path` from the root
resolves the wrong graph and can pass as exit 0 (the false-green trap, `CLAUDE.md`).

```sh
bash scripts/verify-hub.sh              # the crates the live product is made of (CI's `hub` job)
cd crates/bebop-store && cargo test     # or one at a time: dowiz-core, dowiz-hub, workers/api
cd kernel && cargo test --lib

for g in tools/gates/*.sh; do sh "$g"; done; python3 tools/gates/unreached.py
sh crates/bebop-wasm/gate.sh            # needs the pinned toolchain with wasm32 (rust-toolchain.toml)
node e2e/gates/conservation.prove.mjs   # the audit's own alarm, against a stub

node --test workers/api/public/lib/*.test.mjs
cd e2e/kit-regression && npm test       # Playwright; outbox.mjs and courier-cold.mjs are the CI pair

cd workers/api && npx wrangler deploy   # config workers/api/wrangler.toml; build = worker-build --release
bash workers/api/scripts/smoke.sh https://<slug>.dowiz.org
bash tools/platform/attach-host.sh <slug>   # after POST /api/platform/hubs, or the host never resolves
```

## Status and roadmap

Start at `docs/design/ROADMAP-2026-09-22.md` — what is true with the command that reproduces each
number, what is in flight, what is decided against — then the 2026-09-23 blueprints
`BLUEPRINT-OPERATIONAL-BLIND-SPOTS-2026-09-23.md` and `BLUEPRINT-PLATFORM-TOP-TIER-2026-09-23.md`.

- **In flight, not at HEAD:** a refund route, the till as a shift with an X/Z report, table
  transfer, FX at the till, `channel` on an order as a closed set with its gate, a waste /
  write-off event with a person and a reason, and a waiter/till PWA.
- **Fiscalisation (Albania, Law 87/2019):** `workers/api/src/ebills/` maps ebills.al sales into the
  hub's envelope and is tested; the integration is **read-only** and dowiz is **not a certified
  fiscal device**. Issuing invoices is designed (`BLUEPRINT-EBILLS-INTEGRATION-2026-09-22.md`), not
  built; certification is triggered by paying venues asking for one screen, not by a date.
- **Market sequence:** Albania first, beside the fiscal POS; then Kosovo (same language, a 2026
  software-fiscalisation rule-set), Montenegro, Serbia; North Macedonia only after a printer bridge.
- **Post-quantum on the wire:** the primitives exist; ML-KEM's ACVP gate and PQ identity in the
  hub's tokens are open.
- **Stock ledger:** proven end to end, switched off on the live venue (0 recipes, 0 supplies).
- **Open findings** (`ROADMAP-2026-09-22.md` §6): log cost per order above budget; no end-to-end
  order yet placed through the new command path on the live platform.
- **Decided against, with a re-entry condition:** CRDTs for money and orders, a general effect
  system, an in-process event bus.

## Further reading

`CLAUDE.md` (build model, kernel authority), `DECISIONS.md` (red-line rulings), `MANIFESTO.md`
(the thesis); `docs/design/BLUEPRINT-POS-THE-ROOM-2026-09-22.md`,
`BLUEPRINT-TAX-PRICE-CHANNEL-2026-09-22.md`, `BLUEPRINT-CRM-CONSENT-LOYALTY-2026-09-22.md`,
`BLUEPRINT-NO-SQL-BEBOP-EVERYWHERE-2026-09-21.md`, `BLUEPRINT-STACK-AND-DEPENDENCIES-2026-09-22.md`,
`BLUEPRINT-BEBOP-LANGUAGE-DEPTH-2026-09-23.md`; `bebop-lang/README.md`.
