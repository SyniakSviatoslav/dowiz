# Blueprint — modular service architecture in one repository

**2026-09-21.** A plan to restructure dowiz into services with contracts,
inside the single repository it already has, and to put a hard ceiling on file
size so that the decisions inside a file can be tested.

This is written from a measurement, not a preference. The Worker is 18,591
lines across 30 files; **twenty of them are over 300 lines** and `extra.rs`
alone is 3,666. That one file holds courier invites, customer reveals, supplies,
stock movements, promotions, posts, branding, api keys, health and eight other
things. Nothing in it can be unit-tested, because every decision inside it is
reachable only through an `async fn` that needs a D1 handle and a Durable
Object.

The verification blueprint's central finding was that five shipped defects were
pure decisions tangled into handlers. **That is an architecture problem wearing
a testing problem's clothes, and this document is the other half of the fix.**

---

## 1. What "microservice" means here, and what it must not mean

dowiz runs on one Cloudflare Worker and one Durable Object per venue. Splitting
it into separately deployed processes would buy nothing and cost the thing this
product is actually good at: a placement that reserves stock, appends to the log
and broadcasts is **one CAS write inside one object**. Distributing that turns a
compare-and-swap into a saga.

So: **service boundaries, not service processes.** Each service is a Rust module
tree with a published interface, a schema it owns, and no permission to reach
into another's internals. The deployment stays one Worker; the compiler enforces
the boundaries. This is the shape that Amazon's own internal guidance settled on
after a decade of over-splitting — a boundary is worth a network hop only when
it is worth an independent failure domain, and none of these are yet.

Two things DO earn a process boundary later, and the plan keeps them separable:
- **the hub object** (already one, per venue, and the only stateful thing);
- **long-running or bursty work** — media processing, nightly export, AI —
  which belongs in a queue consumer rather than a request.

---

## 2. The services

Eight, each owning its data and its vocabulary. The dependency arrows only ever
point downwards; a cycle is a compile error once `platform/` exists.

```
                    ┌──────────────────────────────────────┐
                    │  edge/          routing, CORS, CSP    │
                    └──────────────┬───────────────────────┘
                                   │
   ┌───────────┬───────────┬───────┴──────┬───────────┬──────────────┐
   │ identity/ │ catalogue/│  ordering/   │ courier/  │ engagement/  │
   │ auth,     │ dishes,   │  placement,  │ shifts,   │ threads,     │
   │ sessions, │ recipes,  │  lifecycle,  │ assign-   │ reservations,│
   │ tenancy   │ stock     │  money       │ ment, gps │ posts        │
   └─────┬─────┴─────┬─────┴──────┬───────┴─────┬─────┴──────┬───────┘
         │           │            │             │            │
   ┌─────┴───────────┴────────────┴─────────────┴────────────┴───────┐
   │  platform/   the middleware: contracts, schema, errors, ports   │
   └─────────────────────────────┬───────────────────────────────────┘
                                 │
   ┌─────────────────────────────┴───────────────────────────────────┐
   │  kernel (dowiz-core) · hub (dowiz-hub) · store (bebop-store)    │
   │  the laws: money, FSM, stock, evlog. No I/O, no clock, no RNG.  │
   └─────────────────────────────────────────────────────────────────┘
```

| Service | Owns | Today's code |
|---|---|---|
| `platform/` | contracts, schema registry, error taxonomy, ports, telemetry | new |
| `identity/` | login, refresh, tokens, principals, **tenancy resolution** | `auth.rs`, `accounts.rs`, parts of `owner.rs` |
| `catalogue/` | categories, dishes, modifiers, recipes, supplies, stock ledger | `catalog_edit.rs`, `recipe.rs`, stock parts of `extra.rs` |
| `ordering/` | placement, pricing, promo, lifecycle, payment records | `storefront.rs`, order parts of `owner.rs`, `stripe.rs`, `wallet.rs` |
| `courier/` | roster, invites, shifts, assignment, positions, earnings | `courier.rs`, courier parts of `extra.rs` |
| `engagement/` | threads, inbox, reservations, posts, feedback, waitlist | `social.rs`, `booking.rs`, `channels.rs`, `waitlist.rs` |
| `venue/` | settings, features, branding, hours, zones, activation, health | settings parts of `extra.rs`, `integrations.rs` |
| `operations/` | backup, restore, rotation, cron, error log, analytics | `cloud.rs`, `errlog.rs`, analytics parts |

**`extra.rs` does not appear.** It is dismantled into five of the eight, and
that dismantling is the single largest item of work in this plan.

---

## 3. The middleware: one way for a service to be reached

Every service exposes exactly one kind of entry point and never a
`worker::Request`. This is the unified contract the whole plan rests on.

```rust
// platform/src/contract.rs

/// Everything a service is allowed to know about a call.
pub struct Ctx<'a> {
    pub principal: Principal,   // already authenticated and venue-checked
    pub venue: VenueId,         // resolved ONCE, by the edge, never re-guessed
    pub now_ms: i64,            // injected: no service reads a clock
    pub ports: &'a Ports,       // db, hub, object store, http, clock, rng
    pub trace: TraceId,
}

/// The one shape every handler has.
#[async_trait]
pub trait Handler {
    type In: Schema + DeserializeOwned;
    type Out: Schema + Serialize;
    const NAME: &'static str;           // "ordering.place_order"
    const AUTH: AuthRequirement;        // Owner | Courier | Customer | Public
    async fn run(ctx: &Ctx<'_>, input: Self::In) -> Result<Self::Out, Fault>;
}
```

**Why this shape is the point, not ceremony.** Six of today's thirty defects
were a venue resolved differently in two places in the same handler, or a
permission checked against one venue while the write landed in another. `Ctx`
makes that unrepresentable: the venue is resolved once, by the edge, and a
handler that wants a different one has to say so out loud.

Four more were decisions tangled into `async fn`s. A `Handler` is a function
from `In` to `Out` over injected ports, so its decision is testable with a fake
`Ports` and no network at all.

**The error taxonomy is closed**, because the status codes this product returns
are a security surface — a 403 where a 404 belongs confirms a venue exists:

```rust
pub enum Fault {
    NotFound,                       // 404 — also the cross-tenant answer
    Unauthenticated,                // 401
    Forbidden { why: &'static str },// 403 — only when the caller may know
    Invalid { field: &'static str, why: String },  // 400
    Refused { rule: &'static str, detail: String },// 409 — a kernel/ledger law
    Conflict,                       // 409 — CAS lost, caller may retry
    Unavailable { place: &'static str, source: String }, // 503, always loud
}
```

`Unavailable` is the one that fixes the instrument problem: constructing it
writes the `worker_errors` row, so a failure cannot reach a client without also
reaching the table. Today `worker_errors` has never received a row because
`loud!` must be called by hand and the paths that actually fail use `?`.

---

## 4. The schema standard

One definition per message, in Rust, generating everything else. Not OpenAPI
hand-written beside the code — that drifts, and this repo has already lost a
week to a doc that described a different world than the tree.

```rust
#[derive(Schema, Serialize, Deserialize)]
#[schema(name = "ordering.PlaceOrder", version = 3)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PlaceOrder {
    pub items: Vec<Line>,
    pub contact: Contact,
    pub fulfilment: Fulfilment,
    #[schema(enum_of = PaymentKind)]
    pub payment: PaymentKind,
    #[schema(optional, note = "promo code, uppercased by the server")]
    pub promo: Option<String>,
}
```

The derive emits, in one build step:
- the JSON Schema, written to `contracts/` and **committed**, so a schema change
  is a visible diff and a reviewable event;
- a TypeScript type for the clients, so the storefront and the console cannot
  drift from the server (`i.name || i.product_id` would not have compiled
  against a `Line` that had no `name`);
- a compatibility check in CI: a field may be added, never removed or retyped,
  without a version bump.

**Rules that are load-bearing rather than tidy:**
- `deny_unknown_fields` everywhere. Today a promo request with `location_id`
  is a 400 and that is correct; the rule must be universal so it is predictable.
- Money is `Money { minor: i64, currency: Currency }` in every schema. Never a
  bare integer, never a float. The four independent money formatters this
  product once had were possible because money was an `i64` with a convention.
- Identifiers are newtypes: `VenueId`, `OrderId`, `CourierId`, `ProductId`. The
  slug/id confusion that made `dubin-sushi` and `dubin-durres` interchangeable
  becomes a compile error.
- Every schema carries a version, and the hub's stored envelopes carry the
  version they were written with.

---

## 5. The file-size rule, and how to obey it without scattering

**Hard ceiling: 300 lines, target 150.** Enforced by a gate, with a ratchet that
may only go down — the same mechanism `bebop-lang/tools/arch_check.py` already
uses for `.bp`.

The rule is a means, not an end. A 3,666-line file is bad because nothing in it
can be reached in isolation; 30 files of 120 lines that all reach into each
other are worse. So the ceiling comes with a shape:

```
services/ordering/
├── mod.rs              (60)  the service's public surface, and nothing else
├── contract.rs        (140)  In/Out schemas — the only file clients read
├── place/
│   ├── mod.rs          (90)  the handler: orchestration only
│   ├── pricing.rs     (170)  PURE: lines → priced lines. no I/O.
│   ├── promo.rs       (120)  PURE: subtotal + code → discount
│   ├── reserve.rs     (110)  stock reservation, over the hub port
│   └── tests.rs       (220)  pricing and promo, exhaustively
├── lifecycle/
│   ├── mod.rs          (80)  the transition handler
│   ├── transitions.rs (130)  PURE: action + status → next, or a Fault
│   ├── settle.rs      (100)  what a transition does to the shelf
│   └── tests.rs       (240)
└── payment/
    ├── intent.rs      (120)  PURE: order → chargeable amount
    ├── webhook.rs     (160)
    └── tests.rs       (180)
```

**The split is along the testability seam, not alphabetically.** Each directory
has at most one file that touches a port; everything else is a function of its
arguments. `pricing.rs` and `promo.rs` in that tree are exactly the two files
that would have caught the undercharged modifiers and the undiscounted Stripe
intent.

**Tests live beside the code** (`tests.rs` in each directory) and do not count
against the ceiling. A directory with no `tests.rs` fails the gate.

---

## 6. 100 % coverage for worker, hub and storefront

The operator's requirement, and it is achievable only because of §5 — a handler
that needs a network cannot be covered; a pure function always can.

**What 100 % means here, precisely, because the number is worthless otherwise:**

| Surface | Measured | Instrument |
|---|---|---|
| Worker services | **line + branch** over `services/**`, excluding `mod.rs` orchestration shims | `cargo llvm-cov`, host target |
| Hub crates | line + branch over `crates/dowiz-hub/src/**` | `cargo llvm-cov` |
| Storefront & clients | **statement + branch** over `public/store/**`, `public/admin/**`, `public/courier/**`, `public/lib/**` | `c8` over a headless run of the cycle tests |

**Three rules that stop the number from lying** — this codebase has a documented
history of instruments that measure nothing, and a coverage percentage is the
easiest of them to fake:

1. **A line executed is not a line tested.** Coverage is reported beside
   *mutation* score on the money, stock and tenancy modules. A test suite that
   cannot tell `<=` from `<` has not tested the comparison, and the promo clamp
   and the stock `available` check are both exactly that comparison.
2. **The port-touching files are exempt and must be tiny.** Anything that cannot
   be covered by a unit test is pushed into a `mod.rs` shim that does nothing but
   sequence calls, and those shims are covered by the L3/L4 gates instead. The
   exemption list is committed and may only shrink.
3. **The clients' number comes from the CYCLE tests**, not from a synthetic DOM
   harness. jsdom applies no content policy and has no layout engine — that blind
   spot already cost this product a week, and a coverage run against it would
   have reported 100 % of a storefront that rendered unstyled.

**Order of attack**, since 100 % from 0 is a year and 100 % of what matters is
weeks: money and stock first (a wrong answer costs cash), then tenancy (a wrong
answer costs a tenant), then lifecycle, then the rest.

---

## 7. The migration, phase by phase

No phase leaves the tree un-deployable, and every phase ends with the live gates
green. This is the same discipline the hub-cost work used and it held for seven
phases.

**Phase 0 — the instruments (1 week).** `cargo llvm-cov` and `c8` wired, the
current numbers recorded as a baseline, and the file-size ratchet gate added at
today's worst value so it can only improve. **Nothing is restructured yet**,
because a refactor without a coverage baseline cannot be shown to be safe.

**Phase 1 — `platform/` (1 week).** `Ctx`, `Ports`, `Fault`, `Handler`, the
schema derive, the committed `contracts/` directory. One route ported end to end
as proof — `ordering.place_order`, the most valuable and most dangerous one.

**Phase 2 — dismantle `extra.rs` (2 weeks).** 3,666 lines into five services.
Mechanical, reviewable in slices, and the biggest single drop in the ratchet.
Each slice moves one feature and lands with its `tests.rs`.

**Phase 3 — `identity/` (1 week).** The tenancy rules that produced six of
today's defects, lifted into pure functions behind `Ctx`. `venue_for_login` and
`wallet_who` are already this shape and are the pattern.

**Phase 4 — `ordering/` (2 weeks).** Pricing, promo, modifiers, lifecycle,
settle, payment — the tree in §5. Target 100 % line+branch here first.

**Phase 5 — `catalogue/` and `courier/` (2 weeks).**

**Phase 6 — `engagement/`, `venue/`, `operations/` (2 weeks).**

**Phase 7 — the clients (2 weeks).** Generated TypeScript types consumed by the
storefront, console and courier app; the shared `lib/` split under the same
ceiling; `c8` coverage from the cycle tests.

**Phase 8 — the seams that earn a process (ongoing).** Media processing and the
nightly export moved to queue consumers, because they are the only work in this
system that is neither a request nor a venue's own state.

---

## 8. Standards this plan adopts, and the defect each one answers

Not a style guide. Each rule exists because something specific went wrong.

| Rule | The defect it answers |
|---|---|
| No `async fn` may contain a tenancy, pricing or permission `match` | six venue-guess defects, two money defects |
| Every identifier is a newtype | slug/id confusion between `dubin-sushi` and `dubin-durres` |
| `Fault::Unavailable` writes its own `worker_errors` row | an error table that has never received a row |
| A response's status code comes from `Fault`, never from a literal | cross-tenant answers that leaked existence |
| `deny_unknown_fields` on every schema | silently ignored fields |
| No `.ok()` on a `Result` that can distinguish failure from absence | a storage error read as "no image" |
| No empty `catch {}` in a client without a visible state beside it | four screenshots that wrote nowhere; a silent poll failure |
| Every gate that graduates out of scratch loses its `_` prefix | `.gitignore` deleting three committed gates |
| A number in a comment carries the date it was measured | a retention policy documented as 35 days that keeps 8 |

---

## 9. What this plan explicitly refuses

- **Separate deployments per service.** See §1. Revisit when a service has an
  independent failure domain, not before.
- **An event bus between in-process services.** A direct typed call is
  debuggable; a bus is a distributed system with none of the benefits.
- **A shared `utils` module.** It is where boundaries go to die. A helper lives
  in the service that owns its vocabulary, or in `platform/` with a contract.
- **Rewriting the kernel.** `dowiz-core` and `dowiz-hub` hold the laws, are the
  best-tested code here (265 hub tests), and are not what broke. The kernel's own
  large files are a separate, later question — `academia_p2p.rs` at 5,192 lines
  is not on the order path and can wait.
