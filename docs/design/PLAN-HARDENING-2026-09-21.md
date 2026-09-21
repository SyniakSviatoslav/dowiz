# The hardening plan — four blueprints, one programme

**2026-09-21.** The index and the sequence for the four documents written today,
plus the research pass that revised them. It exists because three separate
blueprints with their own phase numbering is a plan nobody can execute.

| # | Blueprint | Answers |
|---|---|---|
| 1 | [Verification pyramid](./BLUEPRINT-VERIFICATION-PYRAMID-2026-09-21.md) | is it **right** |
| 2 | [Modular architecture](./BLUEPRINT-MODULAR-ARCHITECTURE-2026-09-21.md) | where the code **lives**, so it can be asked |
| 3 | [Resilience and evolution](./BLUEPRINT-RESILIENCE-AND-EVOLUTION-2026-09-21.md) | what happens when two cashiers, two clocks, two schema versions or two networks **disagree** |
| 4 | [Evals: traffic, memory, latency](./BLUEPRINT-EVALS-TRAFFIC-MEMORY-LATENCY-2026-09-21.md) | is it **affordable**, **fast**, and are its **predictions** any good |

---

## 0. The stack these plans are written against

**Directive, 2026-09-21: no SQL anywhere. bebop, Rust, wasm — nothing else.**
D1 is removed. The 29 relational tables move into bebop-store images held in
Durable Objects. The migration is [blueprint 5](./BLUEPRINT-NO-SQL-BEBOP-EVERYWHERE-2026-09-21.md).

```
  browser surfaces          store · admin (owner console) · courier · kit · platform
        │                   plain ES modules, no framework, no bundler
        ▼
  ONE Cloudflare Worker     Rust → wasm32. workers-rs 0.8.
        │                   links dowiz-kernel (the sole order/FSM/money authority),
        │                   dowiz-hub and bebop-store (zero dependencies).
        │                   TRANSPORT AND STORAGE ONLY — it decides nothing.
        ├── DO per venue     the venue's bebop images: log, stock, catalog,
        │   (HubImages)      settings, posts — and, after the migration, every
        │                    venue-scoped fact that was a D1 row
        ├── DO __platform    the platform's bebop images: identity, tenancy,
        │   (same class)     sessions, keys, waitlist, errors, the venue registry
        ├── KV "MEDIA"       images
        ├── ASSETS           the static surfaces
        ├── send_email       the waitlist bell
        └── cron triggers    nightly rotation, backup, S3 copy (external S3, not R2)
```

**bebop-lang compiles to raw AArch64 machine words and cannot run in a Worker.**
So "bebop everywhere" means what it already means at the one seam that exists
today: **bebop owns the format and the schema**, `selfhost/std/kv.bp` creates the
layouts (the digests are sha256 and that stays on the bebop side), and the Rust
half — `crates/bebop-store`, zero dependencies — reads and writes the same
pointer-free bytes on the wasm side. The four-way fold check (bebop, bebop-store,
`InMemoryStore`, python) is what keeps the two halves honest, and it has already
caught a defect on the Rust side.

**The invariants that fall out of it:**

> **The venue's event log is the system of record. Everything else — the
> catalogue image, the console's replica, the analytics buckets, the socket's
> pushes — is derived data.** A fact with two sources of truth eventually has two
> values, and six of the thirty-three defects were exactly that.

> **Tenancy becomes structural rather than a filter.** `id_from_name(location_id)`
> gives each venue its own object and its own bytes. A venue that cannot be named
> cannot be read, so the six "venue chosen by a guess" defects stop being a class
> of bug and start being a compile-time impossibility.

## 1. The research pass, and what it actually changed

Not a bibliography. Each row is a specific idea, the rule it produced here, and
the defect that rule answers. Where an idea changed something already written in
blueprints 1–3, the change is marked **▲ revision**.

### Architecture and decomposition

| Source | Idea taken | What it changes here |
|---|---|---|
| *Domain-Driven Design* (Evans); *Implementing DDD* (Vernon) | the **aggregate is the transaction boundary**, one aggregate per transaction | **▲ revision to blueprint 2 §1.** The eight services are bounded contexts; the missing sentence is that **the venue's hub object is the aggregate root**, and *a handler may write exactly one aggregate per call*. Anything touching two is a saga and must be named one in its type. This is the rule that makes blueprint 3 §1's four whole-image paths visibly anomalous rather than merely documented. |
| *Software Architecture: The Hard Parts*; *Monolith to Microservices* (Newman) | a network hop must buy an **independent failure domain**, not tidiness | confirms blueprint 2 §1's refusal to split processes; adds the test to apply later — *name the failure you want to isolate, or do not split* |
| *Building Evolutionary Architectures* (Ford, Parsons, Kua) | **fitness functions**: architecture characteristics expressed as automated, continuously run checks | **▲ revision.** The gates now scattered across four documents become **one registry** (§3), each classified *atomic/holistic* and *triggered/continuous*. The file-size ratchet, the design gate and `chain_check` are all fitness functions and were not being managed as one thing. |
| *Fundamentals of Software Architecture* (Richards & Ford) | architecture characteristics are chosen, few, and measurable | the four budgets in blueprint 4 §2 are this product's characteristics; everything else is negotiable |
| *Working Effectively with Legacy Code* (Feathers) | find a **seam**; pin behaviour with **characterization tests before** moving code | **▲ revision to blueprint 2 phase 2.** Dismantling `extra.rs` (3,666 lines) must land the characterization test **in the commit before** the move, not beside it. A test written after a move tests the move. |
| *Refactoring* (Fowler) | **parallel change** (expand → migrate → contract) | the schema derive's compatibility rule, and how a client type generation lands without a flag day |
| *Software Engineering at Google* | **Hyrum's law**; coverage is a poor proxy; the "if you liked it, put a test on it" rule | **▲ revision to blueprint 2 §6.** Reinforces `deny_unknown_fields` and the committed `contracts/`; and makes explicit what was implicit — **every fixed defect lands with the test that would have caught it, in the same commit**, which is now a gate, not a habit |

### Data, evolution and time

| Source | Idea taken | What it changes here |
|---|---|---|
| *Designing Data-Intensive Applications* (Kleppmann) ch. 4 | backward **and forward** compatibility; readers must tolerate what writers do not yet write | blueprint 3 §3's rule that a field is added, never removed or retyped, and that removal is a version bump with an upcaster |
| DDIA ch. 8, "Unreliable Clocks" | a wall clock is a **hint**; a monotonic clock is a **measurement**; never merge them | blueprint 3 §9's two-timestamp rule for offline writes |
| *Streaming Systems* (Akidau et al.) | **event time vs processing time**, and that reconciling them is a design decision, not an accident | the same rule, named: `captured_before_ms` is event time and is a claim; `received_at_ms` is processing time and is the truth |
| Greg Young, versioning in event-sourced systems | **upcasters**: never change a stored event; convert on read; upcasters are append-only too | blueprint 3 §3 in full, including the rule that an upcaster, once written, is never edited |
| *Database Internals* (Petrov) | append-only log + compaction; **write amplification** is the cost that matters | already the hub's design; names why `bebop-parttab-21-cells` is the dominant cost line and why the cell budget gates on *any* increase |
| *Patterns of Enterprise Application Architecture* (Fowler) | **Optimistic Offline Lock** | names the generation guard, and imports its known failure mode: the loser must get a **domain refusal the user can act on**, never a transport error |
| *Enterprise Integration Patterns* (Hohpe & Woolf) | **Idempotent Receiver**, **Dead Letter Channel**, **Correlation Identifier** | blueprint 3 §§2, 5, 8 are all three of these. Adopting the names imports their documented failure modes — chiefly that a dead-letter channel nobody watches is a data-loss feature, which is why the quarantine count is a **failing gate** and not a log line |
| *Fundamentals of Data Engineering* (Reis & Housley) | **data contracts**; idempotency as a pipeline property | the committed `contracts/` directory, and the nightly export reading through upcasters rather than raw |

### Reliability and operations

| Source | Idea taken | What it changes here |
|---|---|---|
| *Release It!* (Nygard) | **circuit breaker**, timeout, bulkhead, fail fast; *integration points are where systems die* | blueprint 3 §6: breakers on Stripe, `ai.endpoint`, Meta — the three integration points that can make a healthy product look broken |
| *Site Reliability Engineering* (Google) | **SLIs, SLOs, error budgets**; the four golden signals | **▲ revision to blueprint 4 §2.** A hard threshold on p95 will flap and then be disabled. Latency and request count become **SLOs with an error budget**: the gate fails when the budget is *spent over a window*, not when one run is unlucky. Bytes and cells stay hard gates, because they are deterministic. |
| *Systems Performance* (Gregg) | the **USE method** — utilization, saturation, errors, per resource | gives `/api/owner/health` its shape for the hub: **utilization** = cells used ÷ ceiling, **saturation** = rotation lag and catch-up-window depth, **errors** = `worker_errors` + quarantined records. Three numbers instead of a gauge dump. |
| *Chaos Engineering* (Rosenthal & Jones) | define the **steady-state hypothesis** first; limit the **blast radius** | **▲ revision to blueprint 1 L5.** Fault injection needs its steady state written down first — and this product already has it: the conservation audit (the ledger balances, `stranded()` is empty, the fold equals the stored status). Blast radius: a QA venue, **never** `LEGACY_VENUE`. |
| *Observability Engineering* (Majors et al.) | one **wide event per request**, high cardinality, trace-first | blueprint 3 §8: the trace id on the response, on the error row **and in the event envelope**, so the order log joins to the trace |
| *Building Secure and Reliable Systems* (Google) | design for understandability; least privilege; **recovery** as a design goal | the closed `Fault` taxonomy, and blueprint 3 §4's external witness — the chain is only evidence if the witness is written by a different principal |
| *Accelerate* (Forsgren, Humble, Kim) | the four key metrics; small batches | why every phase leaves the tree deployable, which has now held for seven phases |

### Types, tests and security

| Source | Idea taken | What it changes here |
|---|---|---|
| *Domain Modeling Made Functional* (Wlaschin); *Secure by Design* (Deogun et al.) | make **illegal states unrepresentable**; domain primitives | the newtype rule, `Money { minor, currency }`, and the `Permit` that a call site cannot forget. **Stated as a principle: where a type and a test can both prevent something, the type wins** — a test tells you afterwards |
| *Growing Object-Oriented Software Guided by Tests* (Freeman & Pryce) | **only mock types you own** | confirms blueprint 1 §5's refusal of mocked Stripe/Meta/S3, and gives the rule for hardware ports later: the *port* is ours and gets a fake; the *provider* is not and gets a live probe |
| *Unit Testing Principles* (Khorikov) | test observable behaviour, not implementation; the four pillars | why the L1 targets are named decisions (`venue_for_login`, `pricing`, `promo`) and not files |
| property-based testing practice (QuickCheck lineage; `proptest`) | state **laws**, let the machine find the counterexample | blueprint 3 §10 — and the finding that `proptest` is already a dependency of `dowiz-core` and `kernel`, aimed at mathematics, with **not one property on money, stock, tenancy or the order machine** |

### AI, ML and data science — applied to what this product actually predicts

| Source | Idea taken | What it changes here |
|---|---|---|
| *Designing Machine Learning Systems* (Huyen) | **natural labels**; a baseline before a model; **slice-based evaluation** | the ETA is a prediction whose ground truth arrives ~40 minutes later **for free**, in the same log. Blueprint 4 §4 is the backtest that has never been run. Slices: distance decile, queue depth, hour of day |
| *Machine Learning Design Patterns* (Lakshmanan et al.) | **Continued Model Evaluation**; checkpoints | the nightly trailing-window backtest reported **beside** the frozen corpus, because the gap between them is the drift signal — and here drift is a business fact (*"your dishes take 6 minutes longer than their stated cooking time"*), not a metric |
| *Building ML Powered Applications* (Ameisen) | ship the heuristic, measure it, and only then consider a model | **the explicit decision not to learn the ETA.** The kernel's premise is determinism (MANIFESTO C2); a learned estimator would have to be trained, versioned, monitored and explained to a restaurant owner. Revisit only if the backtest shows a bias parameters cannot remove |
| *AI Engineering* (Huyen) | **eval-driven development**; exact-match evals over LLM judges for anything that gates | blueprint 4 §6: the assistant's **tool-call correctness** is graded exactly and gates; the **scope/refusal** suite is a security gate; the rubric score is reported and never gates |
| MLOps practice generally | a **frozen, versioned evaluation set** | the same instrument, used twice: `contracts/corpus/` for upcasters and `contracts/corpus/eta/` for the estimator. One mechanism, two consumers |

---

## 2. What the research did **not** change

Recorded so that the next reader does not re-open them.

- **No separate deployments.** Blueprint 2 §1 stands, and *The Hard Parts* strengthens it.
- **No event bus between in-process services.** A typed call is debuggable.
- **No mocked provider tests.** GOOS agrees.
- **No pixel-diff visual regression** until the design system is frozen.
- **No learned ETA** (above).
- **No Merkle tree** until someone must prove one receipt without shipping the log.
- **No POS hardware drivers**, because there is no POS surface — the *discipline*
  (every external actor is a port with a fake, a timeout and a two-phase state)
  is adopted now; the devices are a market decision. Blueprint 3 §7.

---

## 3. The fitness-function registry

Every automated check this architecture is defended by, in one place, each with
its kind. *Atomic* = one module; *holistic* = the system together. *Triggered* =
runs in CI or a gate; *continuous* = runs in production.

| # | Fitness function | Kind | Owns |
|---|---|---|---|
| F1 | file-size ratchet, 300 hard / 150 target, may only fall | atomic · triggered | blueprint 2 §5 |
| F2 | no tenancy/pricing/permission `match` inside an `async fn` | atomic · triggered | 6 venue defects, 2 money defects |
| F3 | line + branch coverage on money, stock, tenancy, **beside mutation score** | atomic · triggered | blueprint 2 §6 |
| F4 | every fixed defect ships with the test that would have caught it | holistic · triggered | the Beyoncé rule |
| F5 | schema compatibility: added, never removed or retyped, without a version bump | atomic · triggered | blueprint 2 §4 |
| F6 | **upcaster totality over the committed corpus** | atomic · triggered | blueprint 3 §3 |
| F7 | `delta` round-trip law | atomic · triggered | two shipped defects |
| F8 | **conservation audit** — ledger nets to zero, `stranded()` empty, fold equals stored status | holistic · triggered **and** continuous | the steady-state hypothesis |
| F9 | replay: every archived log folds to its stored status | holistic · triggered | divergence between the two truths |
| F10 | `chain_check` nightly, before the backup, loud on failure | holistic · **continuous** | a dormant instrument |
| F11 | the off-site archive manifest carries `(count, tip, generation)` | holistic · continuous | tamper evidence with an external witness |
| F12 | idempotency replay matrix on every unsafe route | atomic · triggered | the duplicate order |
| F13 | N-way placement collision: exactly `min(N,k)` succeed | holistic · triggered | the last unit |
| F14 | year-long timezone simulation: the day boundary moves exactly twice | atomic · triggered | the live DST defect |
| F15 | 120 simulated nights: retention matches the stated policy | holistic · triggered | backups keeping 8 copies |
| F16 | poison-pill injection: venue serves, record quarantined and counted, gate red | holistic · triggered | containment |
| F17 | breaker trip: the fallback appears **without paying the timeout** | holistic · triggered | three integration points |
| F18 | instrument-fires: `sqlite_sequence` moves for `worker_errors` | holistic · triggered | an error table that has never received a row |
| F19 | four budgets — requests, bytes, cells, p95 — cells and bytes hard, the rest on an error budget | holistic · triggered | blueprint 4 §2 |
| F20 | ETA backtest: signed bias and range coverage | holistic · continuous | blueprint 4 §4 |
| F21 | assistant tool-call correctness and scope/refusal | atomic + holistic · triggered | blueprint 4 §6 |
| F22 | two-venue probe on every venue-resolving route | holistic · triggered | blueprint 1 L3 |
| F23 | cycle test per feature, asserting the **answer**, ending by putting the venue back | holistic · triggered | blueprint 1 L4 |
| F24 | config contracts: CSP, cron, defaults, no PAN-shaped string in any log or column | atomic · triggered | 3-D Secure; PCI shape |
| F25 | **named corruption tests** on every loader that reads bytes off a network — NOT a fuzzer | atomic · triggered | a hand-written parser on every request path |
| F26 | the SQL ratchet, `tools/gates/no-sql.sh` (at 0) | atomic · triggered | blueprint 5 |
| F27 | **the product's own crates run in CI at all** — `bebop-store`, `dowiz-core`, `dowiz-hub`, `workers/api` | atomic · triggered | the gap below |

**F8 is the one to build first among the holistic ones**, because it is this
product's steady-state hypothesis: every chaos, soak and concurrency test in the
list is defined as *"F8 still holds while X is happening"*.

**F25 IS NOT A FUZZER, and the change is the operator's, 2026-09-21.** A seeded
generator over corrupted images was written, it found two real defects in a day
— an 8 GiB allocation that ABORTS the process rather than failing a test, and a
truncated log that loaded and then answered `len() == 40` while handing back two
records — and it was then deleted. It ran for minutes, and on this box an
allocation that size is not a red test, it is the session. What replaces it is
one named test per corruption, each setting one known cell: *the key length with
bit 33 set*, *a chain that loops*, *nine truncations of a forty-record log*, *a
`next` ref aimed out of the image*. A named corruption says what it protects and
costs milliseconds; a generator says "something, somewhere" and costs the box.
The defects it found are in `crates/bebop-store` (`Store::obj_cells`, and the
refusals above it) and the law is stated there.

**F27 IS THE GAP NOBODY HAD LOOKED FOR.** Until 2026-09-21 CI ran `kernel`,
`engine` and `apps/courier` — and nothing else. The order log, the storage
format, the tables that replaced D1 and every Worker route had four hundred and
fifty passing tests and **no gate that ran them**, so a green badge was a
statement about code the venue does not execute. `scripts/verify-hub.sh` and the
`hub` job in `.github/workflows/ci.yml` run them. Wiring an existing test suite
is the cheapest fitness function in this registry and it was the missing one.

---

## 4. The programme

Blueprint 2's phases are the spine. The resilience and eval items slot in where
they are cheapest. **Nothing here restructures code before there is a baseline to
restructure against** — that ordering is the single most load-bearing decision in
the plan.

### Phase 0 — instruments and the dated defect (1 week)
- `cargo llvm-cov` and `c8` wired; the current numbers **recorded as a baseline**.
- **Run `traffic.mjs` once** and commit the baseline; set the four budgets from it.
- F1 file-size ratchet added at today's worst value.
- **The timezone function and its three call sites.** First item of real work in
  the whole programme, because it is the only one with a deadline: the last
  Sunday of October.
- F10 nightly `chain_check`. One call site, closes a dormant instrument.
- F8 the conservation audit, as a runnable gate after every E2E.
- **F27 the hub crates in CI**, and F25's loaders hardened against the bytes
  they are handed. **DONE 2026-09-21** — see the registry note above.

### Phase 1 — `platform/` (2 weeks, was 1)
`Ctx`, `Ports`, `Fault`, `Handler`, the schema derive, committed `contracts/`.
Now also carries, because they are all the same contract and building them
separately would mean building `Ctx` twice:
- **F12 the `Idempotency-Key` middleware**, placement first;
- **the trace id** on every response, every `worker_errors` row, every envelope;
- **F17 breakers** on the three outbound rails;
- **F6 the upcaster chain** and `contracts/corpus/`.
`ordering.place_order` ported end to end as the proof, and it is the route that
gets all four.

### Phase 2 — dismantle `extra.rs` (2 weeks)
3,666 lines into five services. **Each slice lands its characterization test in
the commit before the move.** F16 quarantine lands here, with the health gauge.

### Phase 3 — `identity/` (1 week)
The tenancy rules that produced six defects, lifted into pure functions behind
`Ctx`. F22 applied to every venue-resolving route.

### Phase 4 — `ordering/` (2 weeks)
Pricing, promo, modifiers, lifecycle, settle, payment. **First target for 100 %
line + branch, with mutation score beside it** (F3). F13 the collision probe and
the stock properties land here, against the code they describe.

### Phase 5 — `catalogue/` and `courier/` (2 weeks)
F15 the 120-night soak lands here, beside rotation and backup.

### Phase 6 — `engagement/`, `venue/`, `operations/` (2 weeks)
F11 the off-site witness. F20 the ETA backtest, nightly.

### Phase 7 — the clients (2 weeks)
Generated TypeScript consumed by storefront, console and courier; `lib/` split
under the same ceiling; `c8` coverage from the cycle tests. The **offline write
queue** becomes possible here and only here, because F12 is by then a contract.

### Phase 8 — the seams that earn a process (ongoing)
Media processing and the nightly export to queue consumers. F21 when a venue
turns the assistant on.

---

## 5. Risks, named

- **The refactor finds new defects and the programme stalls.** Expected. The
  rule: a defect found during a phase is fixed in its own commit with its test
  (F4) and the phase continues. It is not re-planned.
- **Coverage becomes the goal.** The counter is F3's mutation score and blueprint
  2 §6's three rules; if mutation score is not measured, the coverage number is
  reported and not celebrated.
- **The budgets flap and get disabled.** The counter is §1's SRE revision: error
  budgets for the noisy signals, hard gates only for the deterministic ones.
- **`platform/` becomes a `utils` with a better name.** The counter is blueprint
  2 §9's refusal: a helper lives in the service that owns its vocabulary, or in
  `platform/` **with a contract**.
- **Twelve weeks of plan, one operator.** The phases are ordered so that stopping
  after any one of them leaves the product better and deployable. Phase 0 alone
  fixes a live defect, lights a dormant instrument and produces the first real
  cost measurement this platform has ever had.

---

## 6. The one thing with a deadline

**Sunday 25 October 2026**, the last Sunday of the month (checked with `date`, 34 days from today). On that date Europe/Tirane leaves summer
time and three hard-coded `+2 h` constants become wrong: the owner's daily
takings boundary, the schedule's open/closed decision at the edges of the day,
and the daily analytics bucket. Nobody has seen it because the platform has not
yet had a winter. It is Phase 0, item 1.
