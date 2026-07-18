# BLUEPRINT P45 — Deployment + Monitoring Floor: maintenance/ops · security/benchmark tracing · human-readable regression checking · full-layer monitoring · the expanded Telegram tunnel (2026-07-18)

> **Planning document — writes no product code, provisions no infrastructure.** Written against
> the 20-point contract in `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map
> in §9 below — every point addressed, none skipped). This blueprint gives the EXISTING phase
> **P45** (`MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §10.5.5:1073-1091) the full
> blueprint depth it lacked — it does NOT create a new phase number, and it does not change P45's
> position: **explicitly last on the critical path, HARD-blocked on DELIVERY P37**.
>
> **Supersedes/absorbs:** this file is the canonical home for the ops-reliability arc
> (`docs/design/ops-reliability/OPS-RELIABILITY-PLAN.md` + `BLUEPRINTS-OPS-RELIABILITY.md`,
> units OPS-01..22), per the master roadmap's absorbed-arc convention (§10.5.5 audit ledger:
> "All OPS-01..22 → P45", and `CORE-ROADMAP-INDEX.md:151`). The arc's technology choices are
> **reused, not re-derived** (§4d); its one dead premise (attic resurrection) is corrected (§2.3).
>
> **Operator ask this blueprint answers (verbatim intent):** "розробити maintenance/ops,
> security/benchmarks tracing, human understandable & readable documentation/regression checkers,
> повний моніторинг усіх шарів і важливих частин, релізів, метрик і т.д. у існуючому тунелі для
> телеграму — але значно доробленішому і розширенішому." §4 is organized as exactly those five
> asks: 4a maintenance/ops · 4b security/benchmark tracing · 4c human-readable regression checker ·
> 4d full-layer monitoring · 4e the expanded Telegram tunnel.

---

## 1. Ground truth — every cite re-verified live this pass (standard §2 item 1)

All files below were opened and read this session, not inherited from an older doc's claim.

### 1.1 The existing Telegram tunnel — two DIFFERENT systems, one send mechanism

**System A — the external dead-man's-switch (REAL, running):**
`.github/workflows/heartbeat-monitor.yml:1-60`. Exact current shape:

| Property | Value (live file) |
|---|---|
| Schedule | `cron: "*/10 * * * *"` (:13) + `workflow_dispatch` — runs on GitHub's infra, fully decoupled from Hetzner/Cloudflare (:3-6) |
| Probe | `curl … "https://webhook.dowiz.org/"` with `--max-time 10 --retry 3` (:29-31) — the Cloudflare Tunnel webhook endpoint, a **proxy for box + tunnel + WAF chain** (:25), NOT an app |
| DOWN condition | `http_code` empty, `000`, or `>= 500` (:33) |
| Alert | Telegram `sendMessage`, bot token from repo secret `TELEGRAM_BOT_TOKEN` (bot `@dowizbot_bot`, :8), chat id `-1003901655568` **hard-coded** (:43), plain text `🔴 heartbeat-monitor: webhook.dowiz.org unreachable (http_code=…)` (:46), 3 send attempts (:47-58) |
| Silent failure modes (found this pass, §4a.2) | (i) if the Telegram send itself fails, the job exits 1 with `::error::` (:59-60) — **nobody is paged about the pager failing**; (ii) GitHub disables scheduled workflows after 60 days of repo inactivity — the switch can silently stop existing; (iii) no topic id → posts to the chat's general thread |

**System B — the harness's own telemetry bridge (REAL, running, DIFFERENT subject matter):**
`tools/telemetry/` — the self-improvement loop's agent-harness telemetry (tool outcomes, benches,
task/session events, Markov-attractor signals). **NOT product/infra monitoring** — the master
roadmap says so explicitly (§10.5.5:1079 "Real-but-not-this") and P43's anti-scope hardens it
("Do NOT touch the `tools/telemetry` Telegram bridge; it is OPS plumbing, not a product channel",
:1059). What IS reusable from it is the **send mechanism**, verified this pass:

| Primitive | Where | What it gives P45 |
|---|---|---|
| `tg_send` | `tools/telemetry/lib.sh:92-141` | rate-controlled send: global `TG_MIN_GAP` (default 3.5 s) via atomic `flock` (:101-114), 6 attempts, honors Telegram `429 retry_after` with capped backoff (:131-135), forum-topic support via `TELEGRAM_TOPIC_ID` (default 267, :118), never prints the token |
| `tg_deliver`/`tg_spool` | `lib.sh:27-65` | fast path: append JSONL to `/tmp/telemetry-spool/queue.jsonl`, drained by the Rust `telemetry-spool` binary at 3.5 s pace; sync `tg_send` fallback if the drainer is down — reporting never blocks the caller and never goes silent |
| `log_event` | `lib.sh:69-84` | per-kind JSONL ledger `tools/telemetry/logs/<kind>.jsonl` (`{"ts","kind","host",…}`) |
| `bench_run` | `lib.sh:168-194` | wall-clock ms + peak RSS per command, emits `bench` + `metric` events + a Telegram line |
| Topic routing | `tools/telemetry/topics/src/main.rs:8-12`, `report.sh:3-10` | existing forum topics in chat `-1003901655568` ("Dowiz-Reporting"): **257** Reports, **267** Hermes (default), **291** Planning, **292** Git, **294** Benchmarks |
| Host gauges | `tools/telemetry/hetzner-exporter/src/main.rs:1-13` | pure-std HTTP edge on `127.0.0.1:9091/health` serving `{"disk_pct","load1","mem_pct","ts"}` JSON "for Gatus polling"; `--once/--selftest/--probe` modes |
| Config | `tools/telemetry/README.md:26-32` | `TELEGRAM_BOT_TOKEN` from gitignored `dowiz/.env` — the SAME secret as the heartbeat workflow's repo secret; `TELEGRAM_CHAT_ID` non-secret; `TELEMETRY_NO_TG=1` local-only |

### 1.2 What else exists (and does not)

| Claim | Fresh cite | Status |
|---|---|---|
| Native backup primitive: `BlockStore` trait, `MemStore`, `FileBlockStore` (crash-atomic `tmp/<id>.partial` + POSIX rename, content-address re-hash on read, fail-closed `None`) | `kernel/src/backup.rs:1-57` (702 lines total; dedup + exact-restore properties documented :9-14) | REAL, unit-proven, **never exercised end-to-end** — nothing to back up yet |
| CI already runs a gitleaks full-tree secret scan | `.github/workflows/ci.yml:160-168` (`gitleaks detect --config .gitleaks.toml --redact --exit-code 1`) | REAL — push/PR-triggered only, no schedule, no alert on failure |
| CI already runs supply-chain audit | `ci.yml:195-211` (`cargo audit` + `cargo deny` for kernel + engine) | REAL — same two gaps: no schedule, no alert |
| CI already gates kernel hot-path benches | `ci.yml:136-158` `bench-regression` job + `bench-history` artifact; `kernel/benches/bench_track.py --threshold 10`; regression-ledger row 23 | REAL — PR-blocking A/B only; no over-time tracking, no alert channel |
| Regression ledger | `docs/regressions/REGRESSION-LEDGER.md` (190 lines): ratchet rule :7-22, guardrail-type taxonomy :19-22, live rows 18/20/21 + 22-38, archive sections, reversal log :188-190 | REAL — format assessed honestly in §4c.1 |
| Single-pane spec | `docs/ops/P8-SINGLE-PANE-SPEC.md` — every stack signal `[SPEC]`; the ONE `[RUNNING]` item is `tools/health-gate.mjs` (fail-closed local pre-flight: disk, volume mount, kernel green, :105-122); "No canonical prod target exists" (:3-4) | REAL spec, reused in §4d |
| Ops-reliability arc | `docs/design/ops-reliability/OPS-RELIABILITY-PLAN.md` (§2 stack + 8 pager rules :98-101, §2★ co-location trap + dead-man's-switch :103-106, §3 symlink deploy :119-123, §7 3-2-1-1-0 :201-221) + `BLUEPRINTS-OPS-RELIABILITY.md` (OPS-01..22) | REAL research, reused throughout §4 |
| No observability stack anywhere in-repo | grep: zero VictoriaMetrics/Grafana/Netdata/Gatus/SOPS/WAL-G/OpenTofu/Dokploy/PgBouncer/CF-Tunnel config files | CONFIRMED — master roadmap §10.5.5:1080 restated, still true |
| The two live regressions this blueprint uses as worked examples | MASTER §10.5.2 P36 DoD-1/2 (:883-884): (a) `cargo build --target wasm32-unknown-unknown --no-default-features -p bebop2-core` fails `E0425` at `at_rest.rs:74`, introduced by `d23e7aa` (2026-07-17) **after** the remediation doc claimed GREEN; (b) `proto-wire/Cargo.toml` ships `default=["insecure-tls"]` | CONFIRMED live at blueprint time — worked through in §4b.4 |
| pgrust tenant rebuild is a SEPARATE operator-gated track | `docs/design/BLUEPRINT-P-NATIVE-PGRUST-TENANT-REBUILD.md` (proposal, /council-gated); `CORE-ROADMAP-INDEX.md:121` ("Deliberately NOT a Layer A-I item"), `:151` (attic path "dead twice over") | CONFIRMED — cross-referenced only (§2.3), never re-scoped here |
| Sibling blueprint owning repo-internal ops/telemetry | `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P-H-ops-telemetry.md` — chaos harness (§2), `ci.yml:23` fix (§3), bench CI gating (§4), ledger prune/migrate (§5) | CONFIRMED — boundary drawn in §2.2 |

Ground truth is non-discussible; everything below builds on this section only.

---

## 2. Scope — what P45 owns, what blocks it, what it must not touch

### 2.1 The defining constraint, restated firmly (anti-scope FIRST)

**P45 is HARD-BLOCKED on DELIVERY P37 existing — blocked, not merely sequenced after**
(MASTER §10.5.5:1090). There is zero live deployment: no HTTP server, no prod URL, no service
emitting signals. Therefore, in this phase and in this blueprint:

- Do NOT stand up VictoriaMetrics / Grafana / Netdata / Gatus before a service emits signals.
- Do NOT write OpenTofu / Dokploy / Cloudflare-Tunnel config for infrastructure hosting nothing.
- Do NOT create Grafana dashboards, Terraform files, or any live config in this design pass —
  this document's job is to make the build-out fast and unambiguous when P37 lands.
- Do NOT revive attic migrations (files physically deleted; canon forbids it — §2.3).

**The honest wave split that keeps the block meaningful:** a few items in §4 touch only the repo,
CI, and the already-running Telegram bridge — they monitor things that ALREADY exist (the CI
suites, the benches, the ledger, the heartbeat workflow) and are landable pre-P37 without
violating the block. Everything that monitors a *deployed service* waits. Every build item in §4
carries a wave tag:

| Wave | Gate | Contents |
|---|---|---|
| **W0** | none — landable now (repo/CI/Telegram-bridge only, no infrastructure) | §4a.2 dead-man's-switch hardening, §4b.1-.3 security/bench tracing, §4c digest generator, §4e.1-.3 taxonomy + topics + formats, §4a.3 backup drill against a synthetic fixture |
| **W1** | DELIVERY P37 live | §4a.1 deploy path, dead-man's-switch retarget to the app health endpoint, §4d app-layer signals |
| **W2** | W1 + real traffic | §4d full stack (VM+VLogs+Grafana+Netdata+Gatus), 8 pager rules armed |
| **W3** | operator provisioning ([OPERATOR]) | §4a.3 off-Hetzner immutable backup (rsync.net + Object-Lock), separate infra bot creation |

### 2.2 Boundary with sibling blueprint P-H (no double-build)

`BLUEPRINT-P-H-ops-telemetry.md` owns **repo-internal** ops: the chaos/fault-injection harness,
the CI bench gate, the regression-ledger schema and its prune/migrate. **P45 owns everything from
"a service is live" outward**: deployment, infra monitoring, alert routing, off-site backup — plus
the alert/digest *consumption* of P-H's artifacts (bench history, ledger rows). Concretely:
P-H's `bench_track.py` gate stays P-H's; §4b.2's over-time tracker and Telegram alerting CONSUME
its output. P-H §5 owns the ledger's table schema; §4c builds a generated VIEW over it and changes
nothing in the source file's schema.

### 2.3 Superseded piece — cite, don't re-derive

The ops-reliability arc's data-layer path (OPS-01 "resurrect from attic", OPS-02/03 restore +
RLS-fix) is **dead twice over**: `attic/` is physically deleted, and the approach is formally
superseded by `docs/design/BLUEPRINT-P-NATIVE-PGRUST-TENANT-REBUILD.md` (native sqlx adapter,
NOBYPASSRLS inversion, /council-gated red-line track registered at `CORE-ROADMAP-INDEX.md:121`).
P45 does not renumber, restate, or gate on that track's internals. The ONLY contact points:
(i) once the pgrust store holds tenant data, §4a.3's backup subject list grows a `pg_dump`-class
entry; (ii) §4d's DB row activates. Everything else about tenant schema/RLS lives there, not here.

### 2.4 Also not P45's

- Customer-facing messenger channels (Telegram to customers/couriers) — **P43** (MASTER :1056).
- Social auto-posting — **P22**.
- The harness telemetry's OWN subject matter (agent tool-outcomes, Markov signals) — that system
  stays as-is; P45 reuses its send primitives (§1.1-B) and shares its chat only per §4e.2's
  routing rules.

---

## 3. Predefined types & constants (standard §2 item 4 — named before implementation)

Declared up front; no magic numbers or stringly-typed slots appear later in §4.

```rust
// ── tools/ops-alert/src/lib.rs — NEW small crate (W0), pure std + ureq (same dep
//    posture as tools/telemetry/topics). The single alert-shaping authority: every
//    P45 alert — CI step, box cron, future Grafana webhook — flows through this
//    shape before any Telegram send.

/// Alert severity — the four-tier taxonomy (§4e.1). Ordering is load-bearing
/// (routing compares tiers), so Ord is derived deliberately — unlike DomainTag
/// (ledger row 34), severity IS a rank by design.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    S3Ledger,   // ledger/metrics store only — never sent to Telegram
    S2Digest,   // rolled into the daily/weekly digest — never sent standalone
    S1Warning,  // sent to Telegram (spooled, rate-limited) — act within 24h
    S0Critical, // page NOW: bypasses the digest, immediate send, retry-forever-bounded
}

/// The five ecosystem components (MASTER §10.2) — the `component` label's closed set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Component { Core, Protocol, Delivery, Agent, Ops }

/// One alert event. `dedup_key` = "<name>/<target>"; a repeat inside
/// ALERT_DEDUP_WINDOW_MIN is counted, not re-sent (§4e.1 anti-noise).
pub struct AlertEvent {
    pub name: &'static str,        // machine name, e.g. "backup_staleness"
    pub component: Component,
    pub severity: Severity,
    pub value: f64,                // measured value (or 0/1 for boolean probes)
    pub threshold: f64,            // the crossed threshold, for the message body
    pub dedup_key: String,
    pub ts_unix: u64,
}

// ── Thresholds (single authority; every §4 rule cites these consts) ──────────
pub const BENCH_REGRESSION_PCT: f64 = 10.0;     // matches bench_track.py --threshold 10 (ledger row 23)
pub const BENCH_CONFIRM_RUNS: u32 = 2;          // consecutive breaches before an S1 fires (§4b.2 noise math)
pub const BENCH_RUNS_PER_SAMPLE: u32 = 3;       // median-of-3 per scheduled run
pub const DEADMAN_PERIOD_MIN: u64 = 10;         // existing heartbeat cadence (heartbeat-monitor.yml:13)
pub const DEADMAN_ALERT_WITHIN_MIN: u64 = 10;   // P45 DoD-2 (MASTER :1087): outage → page ≤ 10 min
pub const BACKUP_STALENESS_FACTOR: f64 = 1.5;   // pager rule 5 (OPS plan :99-100): age > 1.5× interval = S0
pub const ALERT_DEDUP_WINDOW_MIN: u64 = 60;     // identical dedup_key suppressed within window
pub const ALERT_STORM_N: u32 = 10;              // >N distinct alerts in 5 min → collapse to ONE S0 "storm"
pub const ACTIONABLE_RATIO_FLOOR: f64 = 0.30;   // OPS-09: an alert <30% actionable over a month is demoted/deleted
pub const RELEASES_KEPT_MIN: usize = 5;         // §4a.1: rollback targets never GC'd below this
pub const POST_SWAP_WATCH_S: u64 = 120;         // §4a.1 adversarial: crash-loop watch window after symlink swap
// TG_MIN_GAP (3.5 s) already exists at lib.sh:101 — reused, not redefined.
```

**Metric-naming convention (all layers, one grammar):**
`dowiz_<component>_<subsystem>_<metric>_<unit>` — snake_case; `component ∈ {core, proto,
delivery, agent, ops}`; Prometheus conventions (base units: seconds, bytes; counters end
`_total`). Examples: `dowiz_delivery_order_place_seconds` (histogram),
`dowiz_ops_backup_last_success_seconds` (gauge, unix ts), `dowiz_agent_tool_errors_total`,
`dowiz_core_ci_suite_green` (0/1 gauge). **Label cardinality is bounded by construction**: only
`component`, `host`, `probe` labels; NEVER order-ids/user-ids/shas as label values (cardinality
scaling axis, §5.2).

**Telegram routing table (constants, not prose — consumed by §4e):**

| Route | Lane | Chat | Topic | Who creates |
|---|---|---|---|---|
| `OPS_ALERTS` | S0/S1 product-infra | W0: existing `-1003901655568` · W3: separate infra bot+chat | NEW topic `Ops-Alerts` (id assigned at creation, recorded here) | W0 agent / W3 [OPERATOR] |
| `OPS_RELEASES` | release notes (§4e.3) | same as above | NEW topic `Releases` | same |
| `OPS_DIGEST` | daily/weekly rollups (§4e.4) | same as above | NEW topic `Ops-Digest` | same |
| harness topics 257/267/291/292/294 | self-improvement telemetry | existing chat | existing | untouched |

**Regression-digest row schema (§4c — a VIEW schema; the source ledger schema is P-H's):**
`{ id, class (one clause, ≤15 words), guardrail_type, run_cmd (or "CI:<job>"), status
(GREEN|RED|UNVERIFIED), last_verified (date + source), source_row_anchor }`.

---

## 4. Build items — the operator's five asks, each spec → RED test → code, each with an adversarial case (items 3, 5)

### 4a. Maintenance/ops (deploy path · extended dead-man's-switch · backup exercised + off-site)

#### 4a.3 FIRST — backup end-to-end + off-Hetzner immutable copy (highest-priority item in this file)

The ops-reliability arc's own #1 flagged gap (plan §7:203 "Топ-gap ЗАРАЗ: НЕМАЄ off-Hetzner копії
НІЧОГО → один скомпрометований Hetzner-акаунт = total-loss"), still completely unaddressed
(P8-SPEC §3 "🔴 MISSING — top gap"; MASTER P45 DoD-4 "the one item that stays red until proven").
It leads this section deliberately: the moment P37 produces state worth protecting, this is the
first thing that must already be designed to the bolt.

**Spec (reuses OPS-14/OPS-15 + `backup.rs`, reconciled honestly):** two backup *subjects*, two
*mechanisms* — do not force one tool to do both:

1. **Kernel-native state** (event log, spool, JSONL ledgers, release manifests):
   `kernel/src/backup.rs` `FileBlockStore` manifests — Buzhash-CDC dedup + exact restore are
   already unit-proven (:9-14); what's missing is the END-TO-END drill: back up the real
   directories on the box, restore to a scratch dir, byte-compare. W0-landable against a synthetic
   tenant fixture (a generated event-log + ledger tree); re-run against real state at W1.
2. **pgrust tenant DB** (only once the /council-gated rebuild track lands, §2.3): nightly
   `pg_dump -Fc` + `age` encryption per OPS-14 — `backup.rs` is NOT a Postgres PITR tool and this
   blueprint does not pretend it is. WAL-G stays the arc's stretch item, after a restore drill.

Topology = the arc's 3-2-1-1-0 verbatim (OPS-14): copy 1 live on the attached volume; copy 2
Hetzner-near (bucket, lifecycle tiers); copy 3 **off-Hetzner: rsync.net (SSH-only, zero-egress,
credential-isolated) + an Object-Lock COMPLIANCE-mode bucket for the immutable leg** (set at
bucket CREATION — not retrofittable). Key custody = OPS-15 verbatim: age **multi-recipient**
(primary + cold-escrow pubkeys), ≥2 offline copies, key never inside the Hetzner blast radius,
rotate-by-risk / never destroy old keys. Copy-3 provisioning is **[OPERATOR]** (account/bucket
creation + first upload are human actions — this agent does not create accounts).

**Monitoring hook (the new piece the arc didn't wire):** backup freshness becomes a first-class
metric — `dowiz_ops_backup_last_success_seconds` per subject per copy (source: OPS-08's
`s3_latest_file_timestamp` pattern for buckets; an `mtime` probe for rsync.net via its SSH
interface). Alert rule: age > `BACKUP_STALENESS_FACTOR` × interval ⇒ **S0** (pager rule 5 — the
arc's own "тихий-fail!" warning: backup failure is precisely the failure that stays silent).

**RED tests:**
- R1 (W0): drill script backs up the synthetic fixture via `FileBlockStore`, restores to scratch,
  `diff -r` byte-identical; then **flip one byte in one stored block file** → restore REFUSES
  (fail-closed `get_owned` re-hash, `backup.rs:24-26`) — proven RED by observing refusal, not
  absence of error.
- R2 (W3): restore from copy 3 **with Hetzner credentials deliberately withheld** succeeds
  (MASTER DoD-4's falsifier: survives Hetzner-unreachable).
- R3 (W2): stop the backup cron for 1.5× interval on staging → S0 page arrives.

**Adversarial cases:** (a) ransomware / stolen bucket key deletes copies 1-2 → copy 3's
COMPLIANCE-mode Object-Lock makes early deletion impossible even for the key holder (OPS-14);
(b) primary age key lost → escrow recipient decrypts (OPS-15 RED: assert escrow-only decrypt
works during the drill, not after a disaster); (c) backup job "succeeds" but writes 0 bytes →
freshness metric alone is insufficient; the drill's restore+verify is monthly and the metric
carries bytes-written, alerting on a sudden size drop (pager rule 7).

#### 4a.1 Deploy path — releases/<sha> symlink swap, one-command rollback (W1)

**Spec (OPS-13 reused verbatim, plan :119-123):** `releases/<sha>/` dirs + `current` symlink;
deploy = build → local health-check (reuse the fail-closed pattern of `tools/health-gate.mjs`,
the one already-[RUNNING] gate) → `ln -sfn` (one `rename()` syscall — no half-deployed state);
**rollback = `deploy-rollback.sh <prev-sha>` = one command runnable over SSH from a phone**;
fail-closed schema-guard (refuse boot when schema-version ≠ binary expectation); ≥
`RELEASES_KEPT_MIN` releases retained. Deploy gate: no unapproved auto-deploy touches live state
(the arc's D5-F2 lesson — never reintroduce it). On successful swap, the deploy script emits the
§4e.3 release notification.

**RED tests:** deploy a deliberately broken release (health-check fails) → symlink NOT swapped,
prod untouched, S1 alert "deploy refused"; rollback executes in one command and the health
endpoint returns the previous version id.

**Adversarial case:** health passes at swap time, app crash-loops 30 s later — a swap-time gate
alone is blind to it. Therefore: `POST_SWAP_WATCH_S` watch window after every swap; ≥3 restarts
inside it → **auto-rollback to the previous symlink target + S0 page** (Snapshot Re-entry, §5.4;
restart-intensity predicate pattern reused from `bounded_drainer::launch_permitted`, ledger row 28).

#### 4a.2 Extended dead-man's-switch (W0 hardening now · W1 retarget)

**Spec:** keep the proven GitHub-Actions external poller (it is the pattern the arc demanded,
OPS-10, and it already exists) and extend it in four concrete ways:

1. **Parameterize targets (W0):** probe list moves to a workflow-level env matrix —
   `PROBE_TARGETS="tunnel=https://webhook.dowiz.org/"` today; W1 adds
   `app=https://<prod>/healthz`; W2 adds `pane=<grafana-url>/api/health`. Retargeting at P37
   becomes a one-line diff (MASTER DoD-2). Per-target DOWN message names the target.
2. **Watch the watcher (W0)** — closes the two silent failure modes found in §1.1-A: an on-box
   cron (`tools/ops-alert deadman-check`) asks the GitHub API for the workflow's last scheduled
   run; if the most recent run is older than 3 × `DEADMAN_PERIOD_MIN` (not merely failed —
   *absent*, catching the 60-day schedule-disable) it pages S0 via the box's own `tg_send` path.
   The two watchers use **different infrastructure and different send paths** (GitHub runner +
   repo secret vs box + `.env`), so one compromised/successful-silent lane cannot mute both.
3. **Topic + severity (W0):** the workflow's alert step posts with `message_thread_id` =
   `OPS_ALERTS` topic and the S0 prefix format (§4e.1), replacing the bare-chat plain text.
4. **Send-failure escalation (W0):** on 3 failed Telegram attempts the workflow currently just
   exits 1 (:59-60). Add a fallback send to the harness chat's general thread (different thread,
   same bot) AND leave the `::error::` — item 2's cron independently catches a permanently-failing
   workflow anyway.

**RED tests:** (i) point a probe target at a known-dead URL on a scratch branch →
`workflow_dispatch` run produces the S0 Telegram message ≤ `DEADMAN_ALERT_WITHIN_MIN`; (ii)
disable the workflow → the on-box cron pages within 3 cycles; (iii) W1: induce an outage on the
staging deploy → page ≤10 min (MASTER DoD-2's falsifier, verbatim).

**Adversarial case:** both lanes share ONE bot token — a revoked/blocked bot mutes the entire
pager. Mitigation: W3's separate infra bot (§4e.2) makes the tokens distinct per lane; until then
this is a NAMED accepted residual (honest, not hidden).

### 4b. Security / benchmark tracing

"Security tracing" here means, concretely: **recurring** scans (not only push-triggered), a
**fence** class for security-relevant configuration, and **alerting that reaches a human** —
today a red security job on CI is a silent red box nobody is paged about. "Benchmark tracing"
means: the PR-gate (P-H's) plus an **over-time tracker on main** whose breaches become Telegram
alerts with a designed noise floor.

#### 4b.1 Recurring security scan (W0 — extends existing CI jobs, duplicates nothing)

**Spec:** the `gitleaks` (ci.yml:160-168) and `supply-chain` (ci.yml:195-211) jobs gain a
`schedule:` trigger (daily, off-peak) so a RUSTSEC advisory published *between* pushes — the
current blind window — is caught within 24 h without a commit. A shared final step (all three
security-class jobs) posts on failure: severity **S0** for a leaked secret (gitleaks), **S1** for
a new advisory (cargo-audit) — using the same `curl` send shape as heartbeat-monitor.yml:47-58
with the `OPS_ALERTS` topic. No new scanners: gitleaks + cargo-audit + cargo-deny already cover
secrets + advisories + licenses/bans; adding a fourth tool needs a falsifiable gap first
(standard item 19 — none identified).

**RED test:** on a scratch branch, plant a fake AWS-shaped key in a fixture → gitleaks job red
(already proven by its own design) AND the S0 message arrives; a `workflow_dispatch` of the
scheduled lane completes green end-to-end.

**Adversarial case:** alert spam from a flaky advisory (e.g., an unfixable transitive advisory
red-lining every day). Mitigation: `dedup_key = "cargo_audit/<RUSTSEC-id>"` — one page per NEW
advisory id, then digest-only until resolved or explicitly acknowledged in an ignore-list file
that itself requires a ledger row (ratchet rule: never silently weaken).

#### 4b.2 Security fences (W0) — the insecure-TLS class

**Spec:** a declarative `tools/ops-alert/fences.toml` of must-never assertions, checked by a CI
step (`grep-CI-gate` class per the ledger taxonomy :19-22). Initial fence set (each traceable to
a real incident, not imagined):
- `default-features NOT CONTAINS insecure-tls` for `proto-wire` (bebop-repo CI — the P36 DoD-2
  fence, stated there as "a fence that fails CI if it ever re-enters the default feature set";
  this blueprint supplies its mechanism);
- `no BYPASSRLS in any migration SQL` (guards the pgrust track's R7 seam from here-on-out);
- `heartbeat-monitor.yml probe list non-empty and workflow schedule present` (a fence on the
  pager itself).
Checked via `cargo metadata` (feature sets) and plain grep (SQL/workflow) — no new deps. A fence
trip is **S0** (security class) and RED CI.

**RED test:** scratch branch re-adds `insecure-tls` to defaults → fence step exits 1 + S0
message. Green on the current tree.

**Adversarial case:** the fence file itself is edited to delete a fence — same class as weakening
a guardrail. Mitigation: fences.toml is listed in the regression digest (§4c) with its fence
count; count decrease without a ledger reversal-log entry is flagged by the digest's drift check.

#### 4b.3 Benchmark-regression tracker → Telegram (W0)

**Spec — the mechanism, not "run cargo bench":** nightly on-box cron runs the ALREADY-existing
absolute tracker (`native-trackers bench kernel --threshold 10`, ledger row 23's honesty note:
absolute baselines are host-pinned and deliberately NOT a CI-runner gate) with
`BENCH_RUNS_PER_SAMPLE=3`, taking the **median**; appends to the existing
`tools/telemetry/logs/bench.jsonl` via `log_event` (mechanism already exists, lib.sh:186-187).
A small checker (`tools/ops-alert bench-drift`) compares the rolling median against the pinned
`kernel/benches/baseline.json`:

- breach = median > baseline × (1 + `BENCH_REGRESSION_PCT`/100);
- an S1 alert fires only on **`BENCH_CONFIRM_RUNS` consecutive nightly breaches** of the same
  bench id; the alert body carries the numbers (baseline, last-2 medians, Δ%) and the first
  breaching night's commit range (`git log --oneline` between the two nights);
- recovery (next night under threshold) posts a single S2 line into the digest, not a page.

**Noise floor, argued not asserted:** measured host noise on this box is single-run ±3-5 % (P-H
audit, Area 3 — the reason the same-runner CI gate was rejected as false-RED-prone). Median-of-3
compresses that tail; requiring 2 *consecutive* nightly median breaches at a 10 % threshold means
a false page needs the median of three runs to exceed +10 % twice in a row — for a ±5 % noise
process that is a sustained shift, i.e. a real change, not noise. The threshold consts live in §3;
tightening them is a one-line diff with the ledger row updated.

**Baseline-refresh discipline (anti-gaming):** refreshing `baseline.json` is an explicit act that
requires a regression-ledger row naming the accepted new number and why (precedent: row 23's
refresh recorded exactly that). The §4c digest displays each bench's baseline date, so a
"refreshed yesterday, regression hidden" move is visible on one line.

**RED test:** plant `std::thread::sleep(1ms)` in `bench_place_order` on a scratch branch (row
23's proven RED arm), run the nightly path twice → exactly one S1 with correct numbers; remove →
recovery line in next digest; separately, replay 30 synthetic nights of ±5 % noise through
`bench-drift` → **zero** alerts (the cry-wolf falsifier).

**Adversarial case:** a regression smaller than 10 % each week compounding (boiling frog).
Mitigation: the weekly digest (§4e.4) includes each bench's Δ vs the *baseline date*, not vs
yesterday — a +8 % cumulative drift is visible even though no single alert fired.

#### 4b.4 Worked examples — the two live regressions this design would have caught

1. **bebop no_std wasm32 RED (`d23e7aa`, `at_rest.rs:74`, E0425):** the build broke the same week
   a remediation doc still claimed GREEN, and nobody noticed — because (a) no CI lane built that
   target (P36 DoD-1a adds the lane), and (b) even a red lane pages nobody. Under this blueprint:
   the target lane is a scheduled+push CI job in bebop-repo; a red run on the default branch
   triggers the §4b.1 failure step → **S1 Telegram within 24 h of `d23e7aa`**, naming job + commit
   range. The stale-doc half is caught by §4c: the digest lists the claim's guardrail with
   `status: RED (CI:no_std-wasm32, since 2026-07-17)` — a doc saying "fixed" beside a live RED
   row is a one-glance contradiction.
2. **insecure-TLS default-on (`proto-wire/Cargo.toml`):** a security-relevant default that no
   grep-gate guarded. Under this blueprint it trips the §4b.2 fence **at the PR that introduces
   it** — S0, CI red, never shipped. The fence is cheaper than the incident by construction: one
   `cargo metadata` assertion.

### 4c. Human-readable regression checker

#### 4c.1 Honest assessment of the current format (read in full this pass)

`docs/regressions/REGRESSION-LEDGER.md` is **excellent as a machine-of-record and audit trail**
and **genuinely not human-readable** — the operator's phrasing ("human understandable &
readable") is warranted, not imagined:

- Single table cells run 100-200 words with inline code, formulas, and multi-clause proofs
  (e.g. row 30's "Where" cell ≈ 150 words; row 24 similar). Correct for auditing; unreadable for
  "what protects what, and is it still green?".
- 38 IDs across FOUR sections (live, archive, archive-with-heirs, new-rows) with `a/b` suffixes —
  answering "how many guardrails are live right now?" requires parsing the whole 44 KB file.
- No status column exists at all: a row documents that a guardrail WAS proven red→green at commit
  time; nothing shows whether it is green TODAY (exactly the "remediation doc still claims fixed"
  failure class of §4b.4-1).

The right fix is NOT rewriting the ledger (P-H §5 owns its schema and keeps it verbatim;
move-not-delete discipline) — it is a **generated view plus a status probe**.

#### 4c.2 Build item — `regression-digest` generator (W0)

**Spec:** a small Rust bin `tools/regressions/` (native-port precedent: `tools/telemetry/topics`
is explicitly the Rust port of `topics.sh` — same tech-selection, no comparison needed beyond
citing it) that:

1. Parses the ledger's LIVE rows into the §3 digest schema — one line per guardrail: id, bug
   class in ≤15 words, guardrail type, how to run it (`run_cmd` or `CI:<job-name>`), status,
   last-verified.
2. Derives `status` cheaply, without re-running the world: grep-gates and fences are re-executed
   directly (milliseconds); `cargo`-suite-backed rows read the latest default-branch CI conclusion
   via the GitHub API (source recorded, e.g. `GREEN (CI run #1234, 2026-07-18)`); rows with no
   runnable check are honestly `UNVERIFIED` — no fabricated green.
3. Emits `docs/regressions/REGRESSION-DIGEST.md`: a header ("N live guardrails, N green, N red,
   N unverified, generated <ts> by <cmd>"), then the one-line-per-row table. The raw ledger keeps
   a pointer to the digest and is otherwise untouched.
4. Weekly, the digest's header line goes to Telegram (`OPS_DIGEST` topic, S2).

**RED tests:** (i) drift gate — CI step regenerates the digest and diffs against the committed
file (same pattern as P-A's regenerate-and-diff codegen gate); a hand-edited or stale digest is
RED; (ii) break a cheap guardrail deliberately (delete a fence from fences.toml on a scratch
branch) → regenerated digest shows that row RED and the summary count changes; (iii) the
**readability falsifier**: the operator (a non-agent reader) reads the digest and answers "how
many guardrails are live, which are red, how do I re-run row 23" in under a minute — recorded as
a one-line sign-off in the digest header on first acceptance. If the operator finds it unreadable,
the format iterates; DoD-c is not done until the sign-off exists.

**Adversarial case:** the digest becomes a second source of truth that drifts semantically (a row
summarized wrongly). Mitigation: every digest row carries `source_row_anchor` linking back to the
ledger row; the digest states in its header that the LEDGER is authoritative and the digest is a
generated view — plus the regenerate-diff gate makes stale content structurally impossible.

### 4d. Full-layer monitoring — one pane over all five ecosystem components

**Stack: reused, not re-derived.** VictoriaMetrics (single-node) + VictoriaLogs + Grafana
(unified alerting = the one alert brain, no separate Alertmanager) + Netdata (host agent,
remote_write) + Gatus (synthetic/cert/DNS) — the ops-reliability arc's reasoned choice
(OPS-07/OPS-08; plan §2:73-91 records the reasoning: single store, 1.3 GB vs Loki's 6-7 GB,
PromQL drop-in, least ops for one operator on one box). Grafana reachable only via private
net/Tunnel — never public. This blueprint adds NO new monitoring tools: the operator's
security/regression asks are satisfied by CI + the `ops-alert` crate (§4b), not by another
stack component — re-examined per standard item 19 and no genuine gap in the arc's selection was
found. Deployment of the stack is **W2**.

**The signal map — all five components, one table (each row: signal, source, alert rule, status
today).** The P8-SPEC signal table (§1) is absorbed as the DELIVERY rows' basis.

| Component | Signal | Source | Alert rule (severity) | Today |
|---|---|---|---|---|
| **CORE** | test-suite green on main | CI `cargo-test` job (ci.yml:106) conclusion → `dowiz_core_ci_suite_green` | red on main → S1 (§4b.1 step) | W0 |
| CORE | bench drift | §4b.3 tracker over `bench.jsonl` | 2-consecutive median breach → S1 | W0 |
| CORE | wasm bundle size Δ | build artifact size vs last release (P8-SPEC §1) | >X % Δ → S2 digest line | W1 |
| **PROTOCOL** | cross-repo build matrix (incl. `wasm32-unknown-unknown --no-default-features`) | bebop-repo CI lanes (P36 DoD-1a) | red on default branch → S1 — the §4b.4-1 worked example | W0 (CI lane is P36's; alert wiring is P45's) |
| PROTOCOL | security fences (insecure-TLS class) | §4b.2 fence step | trip → S0 — the §4b.4-2 worked example | W0 |
| PROTOCOL | mesh-node liveness (peer count, gossip lag) | future node `/metrics` once P34B lands | absent-data or lag → S1 | blocked (P34B) |
| **DELIVERY** | uptime (external synthetic, storefront+admin) | Gatus + the §4a.2 external poller | 2 consecutive fails → S0 (pager rule 8) | W1 |
| DELIVERY | order-placement p95 latency | app `/metrics` (`metrics-exporter-prometheus`, per OPS-08 — not the dead opentelemetry-prometheus) | SLO breach 5 min → S1, 15 min → S0 (pager rule 2) | W1 |
| DELIVERY | 5xx / order-fail rate | app `/metrics` | >2 %/5 min → S0 (pager rule 1) | W1 |
| DELIVERY | order-lifecycle lane-stall | app `/metrics` state counts (P8-SPEC §1: 0 transitions out of a lane for N min = wedged FSM) | stall → S0 | W1 |
| DELIVERY | DB connections/locks/WAL · cert expiry | Netdata PG collector · Gatus cert probe | pager rules 6 & 4 | W2 / gated §2.3 |
| **AGENT** | tool-loop error rate | existing `tools/telemetry/logs/*.jsonl` (`log_event` stream) → `dowiz_agent_tool_errors_total` | error ratio >20 %/h → S1; Markov attractor LIMIT_CYCLE signal (ledger row 18) → S2 digest line | W0 (source already live) |
| AGENT | loop liveness | `bench.jsonl`/`task.jsonl` freshness | no events for 24 h while sessions active → S2 | W0 |
| **OPS (itself)** | dead-man's-switch — the watcher watched | §4a.2 both lanes (GitHub↔box, disjoint paths) | either lane silent → S0 | W0 |
| OPS | disk / mem / load | Netdata + the existing `hetzner-exporter` `127.0.0.1:9091/health` (already Gatus-shaped) | disk >85 % S1, >95 % S0 + `predict_linear` time-to-full (pager rule 3) | W0 exporter live; W2 rules |
| OPS | backup freshness + size | §4a.3 metrics | staleness → S0 (pager rule 5); size-drop → S1 (rule 7) | W1/W3 |
| OPS | monitoring stack self-health | Gatus → Grafana/VM `/health`; external poller → pane URL | stack down → the dead-man's-switch still pages (co-location trap, §5.1) | W2 |

**The 8 pager rules** are the arc's list verbatim (plan :98-101) — adopted as the W2 arming set,
severity-mapped above; nothing added, nothing dropped.

**RED test (the pane's own):** one Grafana URL shows host + app + DB + agent + synthetic from ONE
store (OPS-07's RED, unchanged); plus per-row: kill the signal's source and observe the stated
alert — including the **absent-data arm** below.

**Adversarial case (the classic silent-layer trap):** a layer's exporter dies and the pane shows
"no data" which reads as green. Every §4d alert rule therefore carries an `absent()` twin —
no-data for 2 evaluation windows fires the SAME severity as breach (fail-closed monitoring: absence
of evidence-of-health ≠ health — the exact principle `health-gate.mjs` already proves locally).

### 4e. The expanded Telegram tunnel — "значно доробленіший і розширеніший"

What "significantly extended" concretely means: today the tunnel is (a) one hard-coded
plain-text DOWN message from CI, and (b) the harness's own event stream with forum topics. It
becomes: a severity-routed, deduplicated, storm-collapsed alert plane with release notes and
human-readable digests, on lanes that cannot drown each other out.

#### 4e.1 Structured alert taxonomy (W0)

The §3 `Severity` enum is the routing law:

| Tier | Delivery | Examples | Anti-noise rule |
|---|---|---|---|
| **S0** | immediate send, bypasses spool pacing gap (still bounded-retry via `tg_send`'s 429 handling) | app down, dead-man silent, backup stale, leaked secret, security-fence trip, money-path failure | dedup window `ALERT_DEDUP_WINDOW_MIN`; storm collapse: >`ALERT_STORM_N` distinct alerts/5 min → ONE S0 "⛈ alert storm: N signals, top 3: …" page (a cascade must read as one incident, not 50 pages) |
| **S1** | spooled send (existing 3.5 s pacing) | CI red on main, confirmed bench regression, new advisory, disk warn, cert <14 d | `for:`-durations before firing (no single-sample flap — the hysteresis lesson of ledger row 28 applied to alerting); auto-resolve note goes to digest, not a second message |
| **S2** | daily/weekly digest only | recoveries, drift trends, Markov signals, wasm-size Δ | never standalone |
| **S3** | JSONL/metrics store only | everything else | never Telegram |

Message grammar (every tier, one shape):
`<tier-emoji> [<component>/<name>] <one-line fact with numbers> | <target> | <next action or runbook pointer>`
— the "next action" field is mandatory for S0/S1: a page you can't act on is noise by definition
(OPS-09's actionable-ratio doctrine, enforced by §5.1's monthly prune).

**RED test:** fire one synthetic alert per tier through `ops-alert` → S0 arrives immediately in
`OPS_ALERTS`, S1 arrives spool-paced, S2 appears only in the next digest, S3 appears nowhere on
Telegram; replay 20 identical S1s in an hour → exactly one message + a count.

#### 4e.2 Channel separation — recommendation with reasoning (W0 topics · W3 bot)

The source arc already ruled the product side: **the infra bot must be SEPARATE from the
business bot** (OPS-09: "ОКРЕМИЙ infra-бот, НЕ бізнес-бот замовлень — щоб page не тонув у чаті
замовлень"; plan :93-96). This blueprint extends the same reasoning one step, as its own
recommendation: **product-infra ops should also be separate from the harness's self-improvement
lane** — for the same signal-to-noise reason in both directions (a 3 a.m. S0 page must not sit
between agent bench chatter; agent telemetry must not inherit pager gravity) plus one the arc
didn't need yet: **token blast-radius** — one revoked/leaked bot token must not simultaneously
mute the pager and the harness (§4a.2's named residual).

Three lanes, end state:
1. **Harness lane (exists, untouched):** `@dowizbot_bot` → chat `-1003901655568`, topics
   257/267/291/292/294 — self-improvement telemetry stays here.
2. **Infra-ops lane (P45's):** W0 = new forum topics `Ops-Alerts`/`Releases`/`Ops-Digest` in the
   existing chat (zero new secrets, landable today; ids recorded in §3's routing table on
   creation). W3 = a **separate bot + chat** created by the operator ([OPERATOR] — bot creation
   is a human action); cutover = env-var change in `ops-alert` + the workflow secret, because
   every sender already goes through the §3 routing table.
3. **Business lane (NOT P45):** customer/courier-facing sends are P43's scope (MASTER :1056);
   this blueprint only reserves the name so nobody routes product notifications through the ops
   bot.

Mechanism reuse: `lib.sh`'s `tg_send`/`tg_deliver` already parameterize chat/topic/token via env
— the send machinery is shared as a library; only the routing table differs per lane. No second
sender implementation (reuse-first, standard item 19).

#### 4e.3 Release notification format (W1 — emitted by §4a.1's deploy script)

```
🚀 dowiz release a1b2c3d → prod (2026-XX-XX 14:02 UTC)
7 commits since f9e8d7c:
  • feat(kernel): …           • fix(delivery): …           • chore(ci): …  (+4 more)
gates: kernel 495 ✅ · bench Δmax +1.8 % ✅ · gitleaks ✅ · fences ✅
deploy: symlink swap OK · health 200 in 1.4 s · watch window clean (120 s)
rollback: ssh box 'deploy-rollback.sh f9e8d7c'
```
One message per deploy to `OPS_RELEASES` (S1 lane, spooled). Content rules: subjects from
`git log --oneline <prev>..<new>` (top 3 + count), every gate that guarded the release with its
real number, and the **exact rollback command with the real previous sha** — a release note that
doesn't tell you how to undo it is half a release note. Auto-rollback events (§4a.1 adversarial)
post the same shape with `⏪ AUTO-ROLLBACK` and severity S0.

#### 4e.4 Metrics-summary digest format (W0 skeleton · W2 full)

Daily, one message to `OPS_DIGEST`, human-readable, no raw PromQL output:

```
📊 dowiz daily — 2026-XX-XX
product   uptime 100 % · order p95 212 ms (SLO 500) · 5xx 0.00 % · orders OK
box       disk 78 % (time-to-full >30 d) · load ok · cert 71 d
backup    copy2 age 6 h ✅ · copy3 age 22 h ✅ · size Δ +0.4 %
CI        main green · bench drift: none (worst +3.1 % vs baseline 2026-07-17)
security  0 new advisories · fences 3/3 ✅ · secrets scan ✅
agent     132 tool-calls · 3.0 % err · loop signals: none
alerts    S0 0 · S1 1 (disk-warn 14:02, resolved 14:31) · digest-only 4
guardrails 27 live · 27 green · 0 red   (full: docs/regressions/REGRESSION-DIGEST.md)
```
Weekly rollup = same sections with 7-day trends (arrows vs prior week) + the §4b.3 cumulative
bench-drift line (the boiling-frog counter) + the §4c digest header. W0 lands the generator with
the rows that have live sources today (CI, security, agent, guardrails); product/box/backup rows
say `n/a (pre-P37)` — printed honestly, not faked.

**RED test:** generate a digest from a fixture day of JSONL/CI data → every number in the message
is reproducible from the source files; a non-agent reader confirms the daily digest answers "is
everything OK, and what happened?" without opening any dashboard (same sign-off protocol as
§4c.2's readability falsifier).

**Adversarial case:** the digest itself becomes noise (nobody reads a daily wall). Mitigations:
the digest is ONE message (never a thread of 10); a `nothing to report — all green` day
compresses to three lines; if the operator mutes the digest topic for a month, that is treated
as a failed actionable-ratio signal and cadence drops to weekly (the prune discipline applies to
the digest too).

---

## 5. Cross-cutting design obligations (standard items 6, 8, 11, 13)

### 5.1 Hazard-safety as engineering, not prose (item 6)

- **Alert fatigue is the system's primary self-defeat mode** — a monitoring system that pages too
  much trains its human to ignore it, which equals no monitoring at a higher cost. The anti-noise
  discipline is structural, not aspirational: the four-tier taxonomy with hard routing (§4e.1),
  `for:`-durations and confirm-runs before any page (§4b.3's argued noise floor), dedup windows,
  storm collapse, and OPS-09's quantified hygiene loop — any alert under
  `ACTIONABLE_RATIO_FLOOR` (30 %) actionability over a month is demoted or deleted at the monthly
  prune. Target steady-state: S0 ≈ 0/week, S1 ≤ a handful/week.
- **A monitoring system co-located with what it monitors cannot report its own death** — the
  arc's ★-flagged trap (plan :103-106) and precisely why the external dead-man's-switch already
  exists. This blueprint reinforces it to two mutually-watching lanes on disjoint infrastructure
  and send paths (§4a.2), and adds `absent()` twins on every alert rule so no-data can never
  render as green (§4d adversarial).
- **Fail-closed detectors:** a broken check refuses, it does not pass — the pattern is already
  proven in-repo (`health-gate.mjs`, `backup.rs` `get_owned`) and every P45 probe inherits it.

### 5.2 Schemas designed for scaling (item 8)

Design point: **one box, one operator**. Stated scaling axes and their break points:
- **Alert volume:** ~10² rules evaluated/min, target ≤10 sends/day. Telegram forum hard limit
  ~20 msg/min is already respected by `TG_MIN_GAP` 3.5 s (ceiling ≈17/min). Breaks at multi-node
  fan-in (duplicate alerts per node) → that is the point to add Alertmanager-style grouping;
  explicitly NOT built now.
- **Metric cardinality:** bounded label set (§3 naming convention — no per-order/per-user
  labels). VM single-node handles millions of series; this design stays <10⁴ by construction.
  Breaks if anyone labels by order-id — forbidden by the convention, checkable by grep on
  `/metrics` output.
- **JSONL ledgers:** append-only, per-kind; rotation at 50 MB/kind (a `logrotate` stanza, W0).
  Breaks at multi-writer concurrency → the spool/drainer pattern already serializes sends; ledger
  writes stay single-host.
- **Releases dir:** `RELEASES_KEPT_MIN`=5 floor, prune above 20 — bounded disk (pager rule 3
  watches the disk anyway).

### 5.3 Isolation / bulkhead (item 11) — monitoring failure must never touch the product

- All collection is **read-only scrape/poll** (OPS-08's stance): exporters bind loopback
  (`hetzner-exporter` already does, `127.0.0.1:9091`); the monitoring stack holds **zero
  credentials capable of mutating product state** — an unsafe "monitoring took the product down
  via a write" state is unrepresentable by capability construction, not by policy.
- The alert path is fully asynchronous to the product path: `tg_deliver`'s spool + sync-fallback
  means a Telegram outage blocks nothing and loses nothing (queue drains later); the app's
  `/metrics` endpoint is a passive read (budget in §7).
- Stack death degrades to the dead-man's-switch lane only — the product keeps serving (the
  bulkhead is: monitoring depends on the product's *observability surface*, the product depends
  on monitoring for *nothing*).
- Lane isolation on Telegram: separate topics now, separate bot tokens at W3, so a failure or
  flood in one lane cannot mute another (§4e.2).

### 5.4 Rollback / self-healing / self-termination — which one and why (item 13)

- **Deploy rollback = Snapshot Re-entry** (cheap regenerative recovery to the last valid epoch):
  `releases/<prev>` IS the snapshot; the symlink swap is the re-entry, one `rename()` — atomic by
  the filesystem's math, not by a supervisor's hope. Auto-rollback on crash-loop (§4a.1) is the
  self-healing arm, guarded by a restart-intensity predicate (row-28 pattern) so heal cannot
  become flap.
- **Backup restore = Snapshot Re-entry with an error-correcting property**: content-addressed
  blocks re-hash on read (fail-closed), manifests restore bit-identically — recoverability is a
  proven round-trip identity (`backup.rs:9-14`), not a promise.
- **Self-termination boundary:** monitoring has no actuator toward the product (5.3) — the only
  "action" it can take autonomously is the deploy script's auto-rollback within its own watch
  window, which is re-entry to a previously-proven-healthy state, never a novel state. Alerts
  escalate to the human; they never mutate.
- **The monitoring stack itself is cattle:** rebuilt from committed config (W2's compose/config
  files once written) — its rollback story is `git checkout` + redeploy, nothing hand-tuned.

---

## 6. DoD — falsifiable, per sub-section (item 2)

| # | Ask | Falsifier (RED unless proven) | Wave |
|---|---|---|---|
| D-a1 | deploy path | Broken-release deploy: symlink NOT swapped, prod untouched, "deploy refused" S1 received; rollback = exactly one command; post-swap crash-loop → auto-rollback + S0 (staged drill) | W1 |
| D-a2 | dead-man's-switch | Deliberately induced outage on the staging deploy → S0 Telegram ≤ 10 min (MASTER DoD-2 verbatim); workflow disabled → box cron pages within 3 cycles; dead probe target on scratch branch → correct per-target message | W0/W1 |
| D-a3 | backup (TOP) | Synthetic-fixture drill: backup→restore `diff -r` clean AND 1-flipped-byte → restore refuses; W3: restore from copy 3 with Hetzner credentials withheld succeeds (MASTER DoD-4); staleness 1.5× → S0 page | W0→W3 |
| D-b1 | security tracing | Planted fake secret → S0 received; scheduled lane green via `workflow_dispatch`; duplicate advisory pages once then digests | W0 |
| D-b2 | fences | Scratch-branch `insecure-tls` re-added to defaults → CI red + S0; current tree green | W0 |
| D-b3 | bench tracker | Planted 1 ms sleep → exactly one S1 after 2 nights with correct numbers; 30 synthetic ±5 % noise nights → zero alerts; baseline refresh without ledger row visible in digest | W0 |
| D-c | regression digest | Regenerate-diff CI gate red on stale digest; deliberately broken cheap guardrail shows RED row; **operator (non-agent reader) sign-off recorded in the digest header** — not done without it | W0 |
| D-d | full-layer pane | One Grafana URL shows all five components from one store; per-row source-kill drill fires the stated alert; `absent()` twin proven on ≥1 row (kill an exporter → same-severity page, never silent green) | W2 |
| D-e | Telegram tunnel | Four-tier synthetic drill (§4e.1 RED); 20 duplicate S1s → 1 message; storm drill → 1 collapsed S0; release note posted on a real staging deploy with working rollback cmd; daily digest numbers reproducible from sources + reader sign-off | W0→W1 |

Every landed item adds its regression-ledger row with the red→green proof, per the standing
ratchet rule (`REGRESSION-LEDGER.md:7-22`) — the alert/digest/fence guardrails become rows of
types `CI-gate`, `grep-CI-gate`, `bench-gate`, and (new taxonomy entry, one word appended to
`:19-22`) `alert-gate`.

---

## 7. Benchmark plan — the monitoring system's OWN overhead budget (item 10)

Monitoring that meaningfully taxes the measured system corrupts its own measurements. Budgets are
gates, measured at W1/W2 arming, not estimates left unverified:

| Surface | Budget | How measured (existing harness only) |
|---|---|---|
| App `/metrics` handler | p99 < 1 ms, zero alloc on the hot counter path | criterion micro-bench beside the handler once P37's server exists; `bench_track.py` gates it like any hot path |
| Host agent (Netdata) + exporters | ≤ 3 % of one core, ≤ 300 MB RSS combined; `hetzner-exporter` is pure-std and stays ≈ 0 | `resource_sample` (lib.sh:146-163) before/after enabling, recorded via `log_event metric` |
| VM + VLogs + Grafana | ≤ 2 GB RSS combined (the arc's own sizing basis: VLogs 1.3 GB vs Loki 6-7 GB was a selection criterion — hold the stack to it) | same sampling, W2 |
| Nightly bench run | ≤ 10 min wall, scheduled off-peak, never concurrent with a deploy | `bench_run` wrapper already logs ms |
| Alert send path | product-path impact = 0 by construction (spool, §5.3) | asserted by design + the spool's existing fallback test |
| **End-to-end falsifier** | order-placement p95 with full monitoring ON vs OFF on staging differs < 2 % | k6/synthetic run at W2 (the arc's own load-gate pattern, OPS-12) — breach = RED, cut scrape cadence until green |

---

## 8. Links to docs & memory (item 7)

- `docs/design/MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §10.5.5 (:1073-1091) — the
  phase this blueprint deepens; its DoD-1..4 are contained in §6 (D-a2, D-a3, D-d).
- `docs/design/ops-reliability/OPS-RELIABILITY-PLAN.md` + `BLUEPRINTS-OPS-RELIABILITY.md` —
  ABSORBED (stack choice, 8 pager rules, 3-2-1-1-0, CF-Tunnel origin-hiding, OPS-09 bot
  separation, OPS-13 symlink deploy). The arc remains the research record; THIS file is canon.
- `docs/ops/P8-SINGLE-PANE-SPEC.md` — absorbed into §4d's signal map; `health-gate.mjs` remains
  the local [RUNNING] gate.
- `docs/design/BLUEPRINT-P-NATIVE-PGRUST-TENANT-REBUILD.md` + `CORE-ROADMAP-INDEX.md:121,151` —
  the separate /council-gated data-layer track (§2.3; cited, never re-scoped).
- `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P-H-ops-telemetry.md` — sibling; boundary §2.2.
- `docs/regressions/REGRESSION-LEDGER.md` — source of §4c's view; rows 18/23/28 reused as
  patterns; every P45 landing adds rows per §6.
- `.github/workflows/heartbeat-monitor.yml`, `tools/telemetry/` — the existing tunnel, §1.1.
- `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` — the 20-point contract (§9).
- Memory: `ops-reliability-arc-2026-07-13.md`, `environment-and-ops-facts-2026-07-16.md`,
  `rust-native-bare-metal-decision-2026-07-14.md` (tech-selection style used in §4c.2/§4d),
  `test-integrity-rules-2026-06-27.md` (no fake-green — §4c's `UNVERIFIED` honesty),
  `never-bypass-human-gates-2026-06-29.md` ([OPERATOR] items in §4a.3/§4e.2).

---

## 9. Standard-compliance map (all 20 points, checkable)

| # | Contract point | Where satisfied |
|---|---|---|
| 1 | Ground truth, file:line, verified this pass | §1 (every row re-read live; two silent failure modes newly found in the heartbeat workflow) |
| 2 | Falsifiable DoD | §6 (per-ask falsifiers incl. MASTER's own DoD-2/4 verbatim) |
| 3 | Spec→test→code, event-driven | §4a-4e each ordered spec → RED test → mechanism; alerts ARE events (`AlertEvent`, JSONL event stream) |
| 4 | Predefined types & constants | §3 (Severity/Component/AlertEvent, 12 named consts, naming grammar, routing table, digest schema) |
| 5 | Adversarial cases incl. intentionally-failing | every §4 sub-item has one (byte-flip restore, crash-loop after green health, 60-day cron disable, noise replay, fence deletion, silent-layer absent-data, alert storm, digest-as-noise) |
| 6 | Hazard-safety grounded in structure | §5.1 (fatigue as structural discipline; unrepresentable-mutation §5.3; fail-closed detectors) |
| 7 | Links to docs & memory | §8 |
| 8 | Schemas with scaling axes | §5.2 (alert volume, cardinality, ledger rotation, releases dir — each with its break point) |
| 9 | Linux-discipline verdict framework | reuse verdicts inline: stack choice ALREADY-EQUIVALENT (arc, §4d); dead-man's-switch REINFORCES (§4a.2); security fences + digest EXTENDS; no GAP tool added without falsifiable need (§4b.1, §4d) |
| 10 | Benchmarks + telemetry hooks | §7 (own-overhead budget with end-to-end falsifier); §4b.3 (bench telemetry loop) |
| 11 | Isolation/bulkhead | §5.3 (read-only collection, zero product credentials, spool decoupling, lane isolation) |
| 12 | Mesh awareness | §4d PROTOCOL rows (node-local `/metrics`, gossip-lag signal deferred to P34B with the dependency named); nothing here rides the mesh transport |
| 13 | Rollback/self-healing as math | §5.4 (Snapshot Re-entry twice, named; restart-intensity predicate; no-actuator self-termination boundary) |
| 14 | Error-propagation isolation + smart index | §4b.2 fences + §4c regenerate-diff gate + `absent()` twins — each turns a silent-failure class into CI-time or alert-time signal |
| 15 | Living-memory awareness | JSONL ledgers as append-only temporal streams with rotation (§5.2); digest = derived view, ledger authoritative (§4c adversarial) — move-not-delete honored (§4c.2) |
| 16 | Tensor/spectral reuse where applicable | honest: not applicable to alert routing; the one spectral consumer is the existing Markov-attractor signal (row 18) consumed as an AGENT digest line (§4d) — reused, not extended |
| 17 | Regression tracking | §6 closing rule (ledger row per landing, new `alert-gate` taxonomy word); §4c is itself regression-tracking infrastructure |
| 18 | Clear instructions, zero context | §10 |
| 19 | Reuse-first, honest upgrade triggers | heartbeat workflow extended not replaced (§4a.2); gitleaks/audit/bench jobs extended (§4b); `tg_send`/spool reused as the sole sender (§4e.2); arc stack reused with a stated no-gap re-check (§4d); ledger untouched, view added (§4c) |
| 20 | Hermetic principles explicit | Correspondence: the digest mirrors the ledger, generated so it cannot lie (§4c). Vibration: alerting is rate/hysteresis-shaped, not threshold-naive (§4e.1). Cause-and-effect: every page carries its measured cause and next action (§4e.1 grammar). Polarity: fail-closed everywhere a detector can break (§5.1). Rhythm: digest cadence adapts to reader signal (§4e.4 adversarial) |

---

## 10. Clear instructions for other agentic workers (item 18 — zero session context assumed)

You are implementing P45 (Deployment + Monitoring Floor) from this blueprint. Read §1-§5 first.
Hard rules: **W1/W2 items must not start until DELIVERY P37 (a live HTTP surface) exists** —
check `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §10.5.3 P37 status before touching
them. `[OPERATOR]` items (rsync.net/bucket provisioning, new bot creation) are human actions —
prepare, never execute. Never route product-customer messages through ops lanes (P43's scope).
Every landing gets a `docs/regressions/REGRESSION-LEDGER.md` row with a red→green proof.

**W0 (landable immediately, order matters only where stated):**
1. Create `tools/ops-alert/` (Rust, std+ureq — mirror `tools/telemetry/topics/` conventions).
   Implement §3's types/consts + a `send` subcommand that shells the routing table into
   `tools/telemetry/lib.sh` `tg_deliver` semantics (or reimplements them natively — parity test
   against `tg_send`'s pacing/429 behavior either way). Acceptance: §4e.1 four-tier drill.
2. Create the three forum topics (`Ops-Alerts`, `Releases`, `Ops-Digest`) in chat
   `-1003901655568` via the bot API; record their ids in this file's §3 routing table (edit the
   table — that is the one sanctioned edit to this document).
3. Harden `.github/workflows/heartbeat-monitor.yml` per §4a.2 items 1-4 (env-matrix targets,
   topic id, fallback send). Add the on-box `deadman-check` cron. Acceptance: D-a2's three
   drills.
4. Extend `ci.yml`: `schedule:` trigger + failure-alert step on `gitleaks` and `supply-chain`
   jobs (§4b.1); add the `fences` step reading `tools/ops-alert/fences.toml` (§4b.2 — the
   proto-wire fence lands in bebop-repo's CI, coordinate with P36 DoD-2, do not fork its scope).
   Acceptance: D-b1, D-b2.
5. Implement `bench-drift` (§4b.3) + nightly cron. Acceptance: D-b3 (including the 30-night
   noise replay — zero alerts).
6. Implement `tools/regressions/` digest generator + CI regenerate-diff step (§4c.2). Request
   the operator readability sign-off; D-c is not done without it.
7. Backup drill against the synthetic fixture (§4a.3 R1) as a repeatable script; wire the
   freshness metric emission (`log_event metric`). Acceptance: D-a3's W0 arm.
8. Digest generator skeleton (§4e.4) over the sources live today; `n/a (pre-P37)` for the rest.

**W1 (after P37):** deploy path per §4a.1 (script + watch window + auto-rollback + release
notification §4e.3); retarget the dead-man's-switch (one-line env diff); arm the DELIVERY rows
of §4d that need only the app's `/metrics`. Acceptance: D-a1, D-a2's ≤10-min outage drill, D-e's
release-note drill.

**W2 (after W1 + real traffic):** stand up VM+VLogs+Grafana+Netdata+Gatus per §4d (configs
committed to the repo first, stack rebuilt-from-git — §5.4); arm the 8 pager rules with
`absent()` twins; run §7's overhead falsifier (breach = cut cadence until green). Acceptance:
D-d.

**W3 ([OPERATOR]-gated):** off-Hetzner immutable copy per §4a.3 (rsync.net + Object-Lock
COMPLIANCE at creation + age multi-recipient escrow proof); separate infra bot cutover (§4e.2).
Acceptance: D-a3's Hetzner-credentials-withheld restore — the single item MASTER P45 calls "the
one that stays red until proven."

**What NOT to do (repeated because it is the failure mode of this phase):** no observability
stack, no IaC, no dashboards before their wave's gate is met; no new monitoring tools beyond the
arc's stack without a falsifiable gap; no attic revival; no edits to the pgrust track's scope; no
second Telegram sender implementation; no weakening any existing guardrail to go green.
