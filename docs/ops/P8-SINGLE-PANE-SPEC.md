# P8 — Single-Pane Ops / Reliability Readiness Spec

> **HONEST LINE (read first):** No canonical prod target exists; this spec is not yet
> deployed. Items below are **verified-by-design**, not **verified-in-prod**. The only
> thing actually running today is `/root/dowiz/tools/health-gate.mjs` (a local,
> fail-closed pre-flight gate) — everything else in this doc is a *spec*, not a live
> system. Labeling legend used below:
> - **[SPEC]** — designed, not deployed. No live service exists.
> - **[RUNNING]** — a real artifact exists and was executed/verified on this box.
> - **[OPERATOR]** — requires an operator (human) action; this agent does not and will
>   not apply it.

Source of truth for P8 status: `docs/design/MASTER-ROADMAP-10-PHASES-2026-07-14.md:66-71`
("Honest: no canonical prod yet → these are spec'd, not deployed.").

---

## 1. Single-Pane — what signals matter

The single pane is one screen where an on-call can see "is the order business actually
healthy?" in <10s. **[SPEC]** — the pane itself (VictoriaMetrics + Grafana + Netdata +
Gatus + DEAD-MAN'S-SWITCH, per `docs/design/ops-reliability/OPS-RELIABILITY-PLAN.md`)
is not deployed. The *signals* it must surface are:

| Signal | Why it matters | Fail-closed color | Source today |
|---|---|---|---|
| **Order lifecycle state counts** (created→confirmed→preparing→ready→delivered / cancelled) | A stuck lane (e.g. 0 transitions out of `preparing` for N min) means the kernel FSM is wedged or the worker died. | RED on lane-stall | **[SPEC]** Gatus synthetic + app `/metrics` |
| **Payment circuit state** (CLOSED / OPEN / HALF-OPEN) | If the payment provider is down, the circuit must be visibly OPEN and the cash-degrade path engaged — not silently retrying. | RED on OPEN-with-no-degrade | **[SPEC]** app `/metrics` gauge |
| **Kernel `cargo test -p dowiz-kernel` green** | The deterministic core (order FSM + integer money) is the canonical source of truth. A red kernel = no deploy. | RED on non-green | **[RUNNING]** asserted by `health-gate.mjs` |
| **WASM bundle size** (kernel `cdylib` .wasm) | Regressions bloat the client/edge load. Budget = track Δ vs last release. | RED on >X% Δ | **[RUNNING]** build artifact exists on disk |
| **Disk free on `/`** | `/` (sda) is **93% used (≈7% free)** measured today — one near-full root can wedge Postgres, the api, and the build cache simultaneously. | RED at **<10% free** | **[RUNNING]** asserted by `health-gate.mjs` |
| **Volume `/mnt/volume-fsn1-1` mounted** | 50G Hetzner volume (currently `/dev/sdb`, 49G, 69% used) is the only non-`/root` scratch + backup staging space. Absent = no backup headroom. `/root` (~47G) is **OFF-LIMITS**. | RED if unmounted | **[RUNNING]** asserted by `health-gate.mjs` |

> Measured on this box at spec authoring time: `/` = 93% used (5.5G avail of 75G);
> volume `/mnt/volume-fsn1-1` = present, `/dev/sdb` 69% used (15G avail of 49G);
> Node v22.22.3, cargo 1.96.1.

---

## 2. The three fail-closed circuit breakers that matter most

All three default to the **safe / degraded / locked** state when their signal is unknown
or the check itself fails ("fail-closed" = a broken detector must NOT unlock the system).

### 2.1 Payment → cash degrade breaker **[SPEC] [OPERATOR]**
When the payment provider (Stripe/Adyen/etc.) circuit is OPEN (error rate / latency budget
exceeded, or the provider health probe is itself unverifiable), the order path must
**degrade to cash / manual-pay on delivery**, never silently retry-and-fail or accept an
unverifiable charge. **Fail-closed default:** if the circuit state cannot be proven CLOSED
from a fresh successful probe, the system behaves as if OPEN → cash-degrade engaged. No
"assume provider is up because the last check was 5 min ago." This is the P8 "degrade-closed
circuit breaker" from the roadmap. Not deployed; the api crate in `attic/apps-api` is the
legacy code to resurrect the resilience path from (`attic/apps-api/dist/lib/resilience/`).

### 2.2 RLS-bypass guard (P8-NOBYPASSRLS-FLAG) **[SPEC] [OPERATOR]**
Per `docs/ops/P8-NOBYPASSRLS-FLAG.md`: `dowiz_app` runs with `BYPASSRLS`, so ~123 RLS
policies are currently **dormant**. The guard's fail-closed rule: **before any prod role
flip to `NOBYPASSRLS` is honored, the staging probe must assert 0 rows returned under a
`NOBYPASSRLS` throwaway role with no `app.user_id` GUC** (RED→GREEN). If the probe is
unavailable or returns >0 rows, the flip is refused and the system stays in its current
(bypassing) state — i.e. it fails *closed* to "do not change authz," exactly matching the
red-line "flag, do not auto-apply" stance of the source flag. The flip itself is an
operator gate, not a code change.

### 2.3 Secret-push pause (P1 runbook) **[SPEC] [OPERATOR]**
Per `docs/ops/P1-PAUSE-SECRET-PUSH-RUNBOOK.md`: the literal "6-hourly secret-replay loop"
is **NOT present** (honest finding) — the real residual risk is the one-off manual
clean-snapshot (`a7d198db`, pre-commit hook skipped) plus diff-only CI secret scanning.
The fail-closed breaker: **a push-freeze is engaged if (a) a secrets-rotation is in flight,
or (b) the full-tree `verify:secrets` gate is not passing.** Default = frozen. The P1
runbook is operator-executed (disable `deploy` job / block `--force`/mirror push); this
agent does not execute it. This breaker is "closed" = no push, which is the safe state.

---

## 3. Backup 3-2-1-1-0 gap analysis **[SPEC] [OPERATOR]**

`3-2-1-1-0` = 3 copies · 2 media · 1 offsite · 1 **immutable** · 0 verified-restore-errors.

| Copy | Where | Status today | Gap |
|---|---|---|---|
| **1** | Live Postgres on the Hetzner box (sda) | exists | single media, single account |
| **2** | Hetzner volume `/mnt/volume-fsn1-1` snapshot / `backup-root` | **partially exists** (volume present, 15G free) — but no verified automated snapshot job observed | not immutable, same account |
| **3 (offsite)** | **off-Hetzner, cold, immutable** | **🔴 MISSING — top gap** | see below |

**What exists:** the Hetzner volume is mounted and has headroom (15G free) — adequate as
backup *staging* + a same-account copy. `rsync` to the volume is **NOT off-Hetzner**; it
is the same Hetzner project/account and fails the "1-offsite" leg (a compromised Hetzner
account or a single-region outage takes both copies).

**Concrete off-Hetzner target (operator-provisioned):** per
`docs/design/ops-reliability/OPS-RELIABILITY-PLAN.md:210` the designed target is
**rsync.net** (SSH-only, zero-egress) for the `age`-encrypted `pg_dump`, layered with an
**Object-Lock COMPLIANCE-mode bucket** for immutability (set at bucket creation, cannot be
overwritten/deleted for the retention window). WAL-G continuous archiving to object storage
is a stretch after a restore drill (pgrust WAL-binary format unverified). **None of this is
provisioned.** Flagged explicitly as **[OPERATOR]** — this agent does not create the
rsync.net account, the bucket, or run the first offsite copy.

**0-verified-errors:** until a real restore drill runs against copy 3 and a row-count
reconciliation passes, the "0 errors" leg is unmet. Currently **unverified**.

---

## 4. The one thing actually running

**[RUNNING]** `/root/dowiz/tools/health-gate.mjs` — a zero-dependency, fail-closed Node
pre-flight gate. It is NOT the single pane; it is a tiny local stand-in that proves the
fail-closed pattern on the three cheap, local, high-signal checks (disk free on `/`,
volume mount present, kernel build/test green). Run it before any deploy or backup:

```bash
node /root/dowiz/tools/health-gate.mjs            # human-readable, exits 0 if OK, non-zero if any check fails
node /root/dowiz/tools/health-gate.mjs --json     # small status object
node --test /root/dowiz/tools/health-gate.test.mjs  # proves fail-closed + green behavior
```

It fails **closed**: any check failure (including the disk check, volume check, or a
non-green kernel) returns a non-zero exit code and prints `FAIL`. A forced-failure
injection (e.g. `ROOT_PATH=/nonexistent` or `DISK_FREE_MIN_PCT=100`) demonstrably exits
non-zero — see the test file.

---

## 5. Status summary

- **[RUNNING]** local fail-closed health gate (verified by the test + this spec's run).
- **[SPEC]** VictoriaMetrics/Grafana/Netdata/Gatus/DEAD-MAN'S-SWITCH single pane.
- **[SPEC]** the 3 circuit breakers (payment→cash, RLS-guard, secret-push-pause).
- **[SPEC]** backup copy 3 off-Hetzner (rsync.net + Object-Lock) — **top gap, unprovisioned**.
- **No system above is deployed to any prod.** This is a readiness spec with one runnable
  local guard.
