# Blueprint — evals: traffic, memory, latency, and the quality of a prediction

**2026-09-21.** The document the [verification pyramid](./BLUEPRINT-VERIFICATION-PYRAMID-2026-09-21.md)
§4 promised. The pyramid answers *is it correct*. This answers the other half:
**is it affordable, is it fast, and are its predictions any good** — and it
answers each with a number that can fail a build rather than an adjective.

The distinction matters because this platform's economics are already written
down. `dowiz-hub-unit-costs` puts one hub at **$5.93/month all-in**, and the
correction in that note is the whole reason this document exists: *duration is
not billed between polls — request COUNTS and the whole-image log rewrite are
what bind.* A performance document that measured milliseconds would have
optimised the wrong thing.

---

## 1. What already exists, and its exact status

`e2e/evals/traffic.mjs` — **written, complete, and never run.** `/tmp/dowiz-evals`
does not exist, so no report has ever been produced. It is recorded here as an
instrument that has not yet been proved to fire; see
`bebop-instruments-that-measure-nothing` for why that distinction is not pedantry.

What it does when it is run: drives three real browsers (customer on an
iPhone 14 Pro profile, owner console, courier) through **one full delivery**,
placement to `DELIVERED`, and counts four things.

- **Requests**, grouped by route *shape* — `/api/order/<uuid>` collapses to
  `/api/order/:id`, because counting ids separately is how the poll hides.
- **Wire bytes**, from `request().sizes()`. The first version read
  `content-length`, which a compressed response usually omits, and so fell back
  to the decoded body every time: **the menu read 86 KB when the wire carried
  12.** That correction is in the file and is the reason this instrument is
  trusted over an estimate.
- **Latency**, p50/p95/worst, from the browser's own timing, plus first
  contentful paint and `transferSize` from the navigation entry.
- **Cells**, read from `/api/owner/health` before and after, which is the venue's
  image growth and therefore the D1 row that gets rewritten and read on every poll.

It ends by comparing against four budgets and **exiting non-zero** when any is
exceeded. That is the property that makes it a gate: *a measurement that cannot
fail is a dashboard.*

Its first act is therefore to **record a baseline, not to pass**. The default
budgets in the file (120 API requests, 400 KB, 1500 ms p95, 60 cells) are
deliberately loose placeholders; the first green run replaces them with the
measured values plus the margin in §3.

---

## 2. The four budgets, and why each one is money

| Budget | Where the number comes from | Today | Gate |
|---|---|---|---|
| **API requests per delivered order** | Cloudflare bills Workers per request | unmeasured | baseline + 20 %, ratcheting down |
| **Wire bytes per delivered order** | the customer's phone plan; ≈1.2 KB was *computed from the format*, never observed | ≈1.2 KB (derived) | fail above 2 KB observed |
| **Image growth per order** | `bebop-parttab-21-cells`: the whole-image rewrite is the dominant D1 cost | ≈21 cells | fail on **any** increase |
| **p95 latency per route** | not billed; it is the product | unmeasured | fail above baseline + 25 % |

Two supporting gauges with no gate yet, because nothing on this box can read
them: **Worker CPU per request** and the real **request count from the
Cloudflare dashboard**. Both need a day of live service and a dashboard read.
Recorded here as open rather than estimated.

**The ratchet rule**, taken from the file-size gate in the modular blueprint and
from `bebop-lang/tools/arch_check.py`: *a budget may be lowered in any commit and
raised only with a dated note naming what was bought.* Without that clause a
budget drifts upward to meet whatever the code does, which is how every
performance regime dies.

---

## 3. Making a measurement that is not noise

This is where most performance gates fail, and the failure mode is specific:
a number taken once, from a warm cache, on a quiet network, becomes a threshold
that then flaps for a year.

- **Three runs, report the median, fail on the median.** One run is an anecdote.
- **`?fresh=1` is a trap** — the edge cache makes a second run of the same URL
  look faster than the product is. See `dowiz-inventory-recipes-2026-09-19`.
  Every eval run places a *new* order.
- **The run puts the venue back.** Same rule as the cycle tests: an eval that
  leaves a delivered QA order in the log distorts the next run's cell count.
- **Cells are measured on a venue with a known generation**, because rotation
  moves the floor. Record the generation in the report.
- **A budget breach names the route.** `over.push()` already does this; the
  report keeps the top-14 route table beside it so a breach is diagnosable from
  the failure output alone, without a rerun.

---

## 4. The eval this product does not have: is the ETA any good?

`eta.rs` and `live_eta.rs` produce a prediction on every order — a quote at
checkout and a "how long is left" on three screens afterwards. The kernel
computes it in **integer minutes, no clock, no float** (`dowiz_kernel::eta`),
so it is deterministic and replayable. And **nothing has ever measured whether
it is right.**

This is a data-science problem with all of its data already collected, because
the order log holds both halves: the quote is in the placement envelope and the
truth is the `DELIVERED` transition's timestamp.

### The backtest

Fold every archived log; for each delivered order emit
`(quoted_min, actual_min, distance_m, basket_size, queue_depth, hour_of_day)`.
Then report, per venue:

| Metric | Why this one |
|---|---|
| **MAE** in minutes | the headline: how wrong, on average |
| **Bias** (signed mean error) | the one that matters commercially — a system that is *late* on average burns trust far faster than one that is imprecise |
| **p90 absolute error** | the customer who complains is in this tail, not at the mean |
| **Coverage** of the quoted range | if a range is published, the honest number is *what fraction of deliveries actually landed inside it* |
| **Calibration by decile of distance and of queue depth** | where the model is wrong, not just that it is |

**Bias is the gate.** A quote should be *conservative by construction*: the
target is a small positive bias (delivered slightly early) and a coverage of the
published range above 80 %. MAE is reported, not gated, because gating it invites
tuning a model against a metric nobody in the restaurant cares about.

### The discipline borrowed from ML, applied to a deterministic model

The estimator here is arithmetic, not a learned model — which removes most of
the hazards and leaves exactly two, both real:

1. **A frozen evaluation set.** `contracts/corpus/eta/` holds a committed sample
   of historical `(features, outcome)` rows. A change to `eta` is measured
   against that set **before and after**, and the diff goes in the commit
   message. This is the same instrument as the upcaster corpus in the resilience
   blueprint §3, and it exists for the same reason: an argument about whether a
   change helped should be settled by a number, not by a reading of the diff.
2. **Drift, which arrives even without a model.** The kitchen's real speed
   changes with staff, season and menu. The backtest therefore runs **nightly
   over a trailing window** and reports the window's bias beside the corpus's,
   because a widening gap between the two is the signal that the venue's
   `KitchenProfile` settings are stale — a business fact the owner can act on,
   surfaced as *"your dishes are taking 6 minutes longer than their stated
   cooking time"* rather than as a metric.

**What is deliberately not adopted:** learning the ETA from data. A learned
estimator would be a non-deterministic input to a kernel whose entire design
premise is determinism (MANIFESTO C2), and it would have to be trained, versioned,
monitored and explained to a restaurant owner. The arithmetic model with an
honest calibration report is the better engineering position until the backtest
shows a bias that parameters cannot remove.

---

## 5. The other predictions, held to the same standard

Anything this system *asserts about the future or about the world* is an eval
target, and each already exists:

| Prediction | Truth available from | Gate |
|---|---|---|
| ETA (quote and live) | the `DELIVERED` transition | §4 |
| Stock availability at checkout | `StockLedger::stranded()` and the fold | **zero** orders accepted for stock that was not there |
| The courier's route distance | successive GPS fixes | reported; gated only if it feeds the ETA |
| The assistant's answers (`assist.rs`, 19 MCP tools) | §6 |
| Reverse geocoding (cached in the browser) | the address the customer corrects | correction rate reported per venue |

The stock row is the important one and it is not a prediction at all — it is the
**conservation audit** the pyramid §2 named. It belongs in this document because
it is the eval whose failure costs the most per incident: an order accepted for
food that does not exist ends in a refund and a lost customer.

---

## 6. Evaluating the assistant, when it is on

`assist.rs` proxies a venue's own `ai.endpoint`; `mcp.rs` exposes 19 tools. Two
things are true of it that change what an eval must be: the model is **the
venue's, not the platform's**, and the tools it can call **write to a real
venue**.

So the eval is not a benchmark score. It is three gates, in increasing order of
value:

1. **Tool-call correctness over a fixed task set.** A committed set of
   owner-phrased tasks ("how many orders are waiting", "mark the last one ready",
   "which dish sold most this week") with the expected *tool and arguments*.
   Graded exactly, not by a judge. This is the layer that catches a renamed
   argument breaking every venue's assistant silently.
2. **Refusal and scope.** No prompt causes a cross-tenant read, a raw customer
   phone without a reveal row, or a write the principal is not entitled to. This
   is a **security** gate that happens to be phrased in English, and it runs
   against the same two-venue fixture as the L3 tenancy probes.
3. **Answer quality**, last and lightest: a small rubric on a fixed set,
   reported as a trend and never gated. An LLM-judged score that can fail a
   build will eventually fail one for a reason nobody can reproduce — which is
   the same objection this repo already makes to mocked provider tests.

**The rule that makes all three affordable:** the assistant's *evaluation*
fixture is the same two-venue fixture everything else uses, so an eval run costs
a venue's setup and nothing more.

---

## 7. The soak run, which is a different instrument

Everything above measures one order. Two defect classes only appear over time,
and both have already shipped:

- **Backup retention keeps 8 copies, not "7 days + 4 weeks"**, because the week
  buckets slide one day per night. The unit test fed a year of keys at once — a
  state a nightly run never reaches.
- **The catch-up window holds exactly one change**, so `RECENT_KEEP = 256` is
  dead and phase 7's delta economy is not happening.

Neither is findable by a test that runs once. The soak is therefore a **simulated
clock** rather than a long wall-clock run: 120 simulated nights, each with the
real nightly job, the real rotation, the real backup, asserting after each one
that the retention set matches the stated policy and that the catch-up window
holds what it claims. This is the gamedev practice from the pyramid §2, and it
is cheap: the kernel takes its time as an argument, so 120 nights is 120 calls.

The second soak, once sockets have been live for a day: replay a day of real
traffic shapes against the deployed hub and watch the **image grow**. The gate is
the cell budget in §2, measured over a day rather than an order.

---

## 8. Where these numbers live

- **The runner:** `e2e/evals/traffic.mjs` (traffic, bytes, latency, cells),
  `e2e/evals/eta-backtest.mjs` (§4), `e2e/evals/soak-nights.mjs` (§7),
  `e2e/evals/assist-tasks.mjs` (§6).
- **The reports:** JSON under `/tmp/dowiz-evals/`, and **the medians committed**
  to `docs/measurements/` with the date and the deployed version they were taken
  against — because `head-may-not-reproduce-its-own-artifacts` means a number
  without its commit is not evidence.
- **The budgets:** in the runner, overridable by env for a deliberate
  investigation, ratcheting per §2.

---

## 9. Order of work

1. **Run `traffic.mjs` once** and commit the baseline. Everything else in this
   document is an argument until that number exists.
2. **Set the four budgets from it** and put it in the cycle.
3. **The ETA backtest**, because the data is already collected and the result is
   a business fact the owner can use on the day it is produced.
4. **The nightly soak**, which owns two shipped defects outright.
5. **The assistant's tool-call and scope gates**, when a venue turns it on.
