# AUDIT 2026-07-18 — Code Quality / Infrastructure Health ("Torvalds pass")

Read-only audit of /root/dowiz + /root/bebop-repo. No fixes applied (operator directive: change nothing).
Persona: blunt, evidence-first. Every finding carries file:line evidence gathered live this session.

**Headline:** the Telegram monitoring is broken for a boring, fully-diagnosable reason — on Jul 17 at
00:54 somebody committed "native Rust ports replace python/bash telemetry" (`4519bd7ff`), which DELETED
the running Python telemetry stack while ZERO of the five Rust replacements had ever been compiled, and
none had a caller. The system has been coasting on ghost processes executing deleted code ever since.
That is not a migration. That is demolition with a press release.

Severity totals: **2 CRITICAL · 9 HIGH · 16 MEDIUM · 5 LOW** (32 findings) + pattern checklist + genuine-strengths list.

---

## Part 1 — Telegram monitoring diagnosis (operator: "багато чого не відправляється, або відправляється")

### [SEVERITY: CRITICAL] [MONITORING] TORVALDS-01
**Where:** commit `4519bd7ff` (2026-07-17 00:54); `tools/telemetry/hetzner-exporter/` (no `target/`); `tools/telemetry/telemetry:149,469`
**What:** The live Python telemetry stack was deleted before its Rust replacements were built, wired, or started — the monitoring outage is self-inflicted by an unfinished migration.
**Evidence:**
- `git log --diff-filter=D` on `tools/telemetry/`: commit `4519bd7ff` "native Rust ports replace python/bash telemetry+security scanners" deleted `hetzner_exporter.py`, `living_memory.py`, `ser.py`, `swarm_proof.py`, `topics.sh`.
- None of the 5 replacement crates (`hetzner-exporter`, `native-ser`, `native-trackers`, `topics`, `swarm-proof`) has EVER been built: `find */target -name release` → **0 hits in all five**. `hetzner-exporter/` has no `target/` directory at all.
- Zero callers: `grep -rn "native-trackers|native-ser|hetzner-exporter|topics/target" tools/telemetry/*.sh tools/telemetry/telemetry` → empty.
- Timeline from Gatus logs (`docker logs ops-gatus`): `hetzner-box-resources` last success **2026-07-17 02:00:12**, first fail 02:01 — 8 minutes before the Rust replacement's `Cargo.toml` was written (mtime 02:09). Port 9091 has been dead **37+ hours**.
- The CLI still invokes the deleted files: `tools/telemetry/telemetry:469` `python3 "$DIR/living_memory.py"` and `:149` `PY="$(dirname "$0")/ser.py"` — both files no longer exist.
**Why it matters:** This is THE root cause of "багато чого не відправляється": resource-breach alerting (disk/mem/load via Gatus topic 257) has been structurally blind since Jul 17 02:01. Gatus fired exactly one TRIGGERED alert at 02:02 and then went silent by design (`send-on-resolved` — nothing to resolve). If the disk hit 100% right now, no alert would distinguish that from "exporter down".
**Fix guidance:** Rule for every future port: old code dies only AFTER the replacement is built, started, supervised, and observed green. Immediately: `cargo build --release` in `hetzner-exporter/`, run it under a systemd unit, and fix `telemetry:149,469` to point at the native binaries or delete those subcommands.

### [SEVERITY: HIGH] [MONITORING] TORVALDS-02
**Where:** PIDs 1093122 + 1165607 (both `tg_send` heart loops, topic 272); `tools/telemetry/telemetry` hetzner-heart body
**What:** TWO duplicate heartbeat daemons post content-free heartbeats ("disk=% mem=% load1=%") twice per minute — this is the operator's "або відправляється" (sends, but garbage).
**Evidence:** `pgrep -af` shows an inline bash heart loop (PID 1093122, started Jul 15 17:40) AND `telemetry hetzner-heart` (PID 1165607, started Jul 15 19:29) both alive, both curling `127.0.0.1:9091/health` (connection refused since Jul 17 02:01), both interpolating the resulting EMPTY strings into the message with zero validation, both sleeping 60s. `/tmp/.tg_send_last` mtime confirms sends succeed every minute.
**Why it matters:** The channel looks alive while carrying no data — the worst monitoring state. Duplicates spam the topic; empty values train the operator to ignore it.
**Fix guidance:** One supervised heart daemon; refuse to send (or send an explicit "exporter DOWN" alert) when the health probe fails; kill the duplicate.

### [SEVERITY: HIGH] [MONITORING] TORVALDS-03
**Where:** all telemetry daemons; PID 1757313 (`/proc/1757313/exe → ... (deleted)`); `/etc/systemd/system/` (no telemetry units)
**What:** Zero process supervision: every telemetry daemon is a session-scoped `nohup` running STALE code parsed Jul 15, nothing survives a reboot, and the spool drainer runs from a binary that no longer exists on disk.
**Evidence:** `ls /etc/systemd/system/ | grep -i "telemetry|heart|gatus"` → only `dowiz-backup-cleanup.*`. Daemon start times (Jul 15 17:40–19:29) predate the `tg_deliver`/spool rewrite (`f199c9e30`, Jul 15 21:52) — the running loops execute the pre-rewrite `tg_send` code from memory. Drainer PID 1757313: `exe → target/release/telemetry-spool (deleted)`; `rust-spool/target/` now contains only `debug/` — the release binary was removed and never rebuilt, so `tg_spool_ensure` (`lib.sh:37-38`) cannot restart it after the next crash/reboot.
**Why it matters:** Every reboot or crash silently removes another piece of monitoring, permanently, with no restart and no alert about the death. This "designed once, silently stopped" pattern is even self-diagnosed in `BLUEPRINT-DISK-OPS-CLEANUP-2026-07-18.md:18`.
**Fix guidance:** systemd units (`Restart=always`) for exporter, heart, drainer, watchers. A daemon that matters is a daemon under supervision — everything else is a demo.

### [SEVERITY: HIGH] [MONITORING] TORVALDS-04
**Where:** `tools/telemetry/lib.sh:46-65`; `/tmp/telemetry-spool/` (absent)
**What:** The "durable" message queue lives in `/tmp`, was deleted out from under the running system, and `tg_deliver` reports success the moment a line is appended — fire-and-forget with no delivery confirmation and no queue-depth monitoring.
**Evidence:** `/tmp/telemetry-spool/` does not exist; the drainer's fds 1/2 point to `/tmp/telemetry-spool/drainer.log (deleted)`. Any messages queued at deletion time are gone, uncounted. Deleter unidentified — eliminated: deep-clean never ran (its log dir `/root/.backups/clean-log` doesn't exist, and `REMOVE_DIRS` in `tools/deep-clean/src/main.rs:52-63` doesn't include the spool); systemd-tmpfiles ages 30d (files were <2 days old).
**Why it matters:** Silent, unbounded message loss — the literal "не відправляється". A queue whose loss nobody notices is not a queue, it's a wastebasket with extra steps.
**Fix guidance:** Move the spool to a persistent path (e.g. `/var/spool/dowiz-telemetry/`), add a queue-depth + drainer-liveness check to the heart message.

### [SEVERITY: HIGH] [MONITORING] TORVALDS-05
**Where:** `tools/telemetry/governance.sh:239-240,93-104`; `tools/telemetry/telemetry:149,469`
**What:** governance.sh ships logic that is broken by inspection: an undefined variable, python3 invoked on a Rust ELF, and a "3-judge jury" that reads a file named with the current timestamp — which can never exist.
**Evidence:**
- `governance.sh:239` `if [ -f "$GOV_LM" ]` — `GOV_LM` is never defined anywhere (the defined var is `GOV_LM_BIN`, line 31). Under the file's own `set -u` this is an unbound-variable error; `gov_recall` ("PRIMARY retrieval") can never use the native engine and always falls through to `gov_precedent`.
- `governance.sh:240` `python3 "$GOV_LM" --query …` — even if the var existed, it points at a compiled Rust binary (`kernel/target/release/lm`); running an ELF through python3 fails unconditionally.
- `governance.sh:93-94` `vf="$GOV_DIR/jury_$(date -u +%Y-%m-%dT%H:%M:%SZ).jsonl"; if [ -f "$vf" ]` — tests for a file stamped with the CURRENT second; dead branch, `gov_judge` always returns ESCALATE. And the case arm `"$b|$b"*)` at `:98` would echo `Decide($a)` for a unanimous-B jury — wrong answer even if the file existed.
**Why it matters:** The "governance" layer's retrieval and jury functions are decorative. Any process trusting `gov_judge`/`gov_recall` output is trusting a function that structurally cannot do what its name claims.
**Fix guidance:** Fix the variable name AND the invocation (`"$GOV_LM_BIN" --query …`, no python3); make the jury read a caller-supplied votes file; fix the case arm; add a shellcheck + smoke-invoke pass to `selftest-telemetry.sh` (bash -n catches none of this).

### [SEVERITY: HIGH] [MONITORING] TORVALDS-06
**Where:** `gh` PAT on-box; `.github/workflows/heartbeat-monitor.yml:59-60`; `BLUEPRINT-P45-ops-security-monitoring.md:40`
**What:** The box is completely blind to its own external dead-man's switch — the on-box PAT 404s on the repo, so nobody can verify from here whether the heartbeat workflow is even enabled, exactly the "watch the watcher" gap P45 already named.
**Evidence:** `gh api repos/SyniakSviatoslav/dowiz` → HTTP 404 (fine-grained PAT `github_pat_11ATQJFRI0…` lacks repo access; git-over-SSH works, API doesn't). The workflow IS on `origin/main` (`git ls-tree origin/main .github/workflows/` lists it), but its enabled/disabled state (GitHub's 60-day scheduled-workflow auto-disable, P45 §1.1-A) is unverifiable from this box. P45:40 also documents that a failed pager send just exits 1 — "nobody is paged about the pager failing". Both silent modes remain open.
**Why it matters:** A dead-man's switch you cannot observe is indistinguishable from a dead one. If GitHub auto-disabled it, the outage detection layer is gone and nothing on this box would ever know.
**Fix guidance:** Scope the PAT to the repo (or add a repo-scoped read token); implement P45 §4a.2 "watch the watcher" (on-box check that the workflow ran within N minutes, page via `tg_send` if absent).

### [SEVERITY: MEDIUM] [MONITORING] TORVALDS-07
**Where:** `tools/telemetry/rust-spool/src/main.rs:240-247,95-117`
**What:** The deployed spool drainer has no dead-letter path: one permanently-rejected message (>4096 chars, bad topic id) head-of-line-blocks the ENTIRE queue forever, invisibly; and a timeout-after-delivery produces duplicate sends.
**Evidence:** `run()` always takes `entries[0]` (`main.rs:240`); `send_one` returning false leaves the line in place (`:244-246`) — retried every ~2s for eternity. Telegram-rejected (`ok:false`) messages are never quarantined. Malformed lines are skipped by `read_spool` (`:86`) but retained forever by `drop_line`'s filter (`:103`). Diagnostics go to `eprintln` → fd 2 → `drainer.log (deleted)`. Meanwhile the caller's `tg_deliver` already returned success. The at-least-once duplicate: a network timeout after Telegram processed the send returns false → line resent. The NEWER `tools/async-spool/src/main.rs:107,186` has a proper `.deadletter` file — the fixed version exists and is not the one deployed.
**Why it matters:** One oversized message = permanent, silent, total spool outage — a precise mechanism for "багато чого не відправляється".
**Fix guidance:** Port async-spool's deadletter handling back (or deploy async-spool and retire rust-spool); enforce the 4096-char limit at `tg_spool` time.

### [SEVERITY: MEDIUM] [MONITORING] TORVALDS-08
**Where:** `tools/telemetry/lib.sh:101-140`
**What:** `tg_send` holds a global flock across the entire retry loop (up to 6 attempts × 30s backoff + 10s curl each ≈ minutes) with no lock timeout — one bad send convoys every sender on the box.
**Evidence:** `exec 9>"$lockf"; flock 9` (`lib.sh:104-105`, no `-w`); lock released only at function exit (`:128,:139`). The comment at `:99-100` says the flock exists because "6 concurrent daemons" share the pace gate — which means all 6 serialize behind the slowest failure. After 6 failed attempts the message is dropped with only an stderr echo (`:138`) — often into a deleted or /dev/null log.
**Why it matters:** During any Telegram degradation, every 60s loop on the box stalls for minutes and messages die quietly — intermittent "не відправляється" that heals itself and defies casual debugging.
**Fix guidance:** `flock -w 10` + spool-on-lock-timeout; route failures into the spool instead of dropping.

### [SEVERITY: MEDIUM] [MONITORING] TORVALDS-09
**Where:** `tools/telemetry/lib.sh:69-84`; `tools/telemetry/logs/metric.jsonl`
**What:** `log_event` writes corrupt JSONL — duplicate `"kind"` keys (last-wins destroys the event kind) and double-escaped payloads — so the "Anu learner" that reads it back cannot see the host metrics; and the ledgers have no rotation.
**Evidence:** live tail of `metric.jsonl`: `{"ts":…,"kind":"metric","host":…,"kind":"host","sample":"{\\\"load1\\\":1.90,…"}` — python parse shows `kind == "host"` (the `metric` value is silently clobbered) and `sample` is a doubly-escaped string (caller pre-escapes with `_jesc`, `log_event:79` escapes again). `gov_learn` (`governance.sh:284-303`) looks for top-level `ms`/`rss_mb`/`eta_err_pct` — host samples never match. 4.3MB and growing; no logrotate entry.
**Why it matters:** The self-learning loop is learning from a ledger it cannot parse. Garbage-in is generous — it's garbage-shaped-like-data.
**Fix guidance:** Reserve the `kind` key (reject/rename collisions), stop double-escaping (pass raw, escape once), add rotation.

### [SEVERITY: MEDIUM] [INFRA] TORVALDS-10
**Where:** `/root/ops/gatus/docker-compose.yml` vs running container `ops-gatus`
**What:** Config drift: the compose file declares bridge networking + port map, but the running container uses `NetworkMode=host` — the next `docker compose up` silently regresses and breaks every `127.0.0.1` endpoint check from inside the container.
**Evidence:** compose: `ports: "127.0.0.1:8081:8080"` + `extra_hosts`; `docker inspect ops-gatus --format '{{.HostConfig.NetworkMode}}'` → `host`. Under the on-disk compose config, `http://127.0.0.1:9091/health` and `:8080` (config.yaml:24,97) would resolve to the container's own loopback, not the host.
**Why it matters:** The monitoring stack only works because its on-disk definition is NOT what's running. First restart from the file = second silent monitoring outage.
**Fix guidance:** Add `network_mode: host` to the compose file (and drop the now-meaningless `ports:` stanza).

### [SEVERITY: MEDIUM] [PACKAGING] TORVALDS-11
**Where:** `tools/telemetry/hermes-kernel` (git-tracked, 659,712 bytes, built Jul 15 20:51)
**What:** A compiled ELF binary is committed to git and is STALE relative to its own source — and `governance.sh` routes all its "native compute" through it.
**Evidence:** `git ls-files tools/telemetry/` lists `hermes-kernel`; `find /root/hermes-agent-kernel-rewrite -name "*.rs" -newer …/hermes-kernel` → `lib.rs`, `control.rs`, `reporting.rs`, `cli/src/main.rs` all newer than the binary. `governance.sh:20` `KERNEL_BIN=$DIR/hermes-kernel`.
**Why it matters:** Unauditable platform-specific blob in version control, silently drifting from the source of truth it claims to embody. This from the same codebase that lectures about "dual-authority risk".
**Fix guidance:** Untrack the binary (gitignore it like the other tool binaries), build from source at provision time, stamp the source commit into `--version`.

---

## Part 2 — System-design pattern gap audit (verdicts, evidence-backed)

Framing correction, verified: the TS/Supabase app (`apps/api`, `packages/db`) is **retired — 0 git-tracked files**. The Repowise index block in CLAUDE.md describing it (145 API files, `server.ts` entry point, hotspot list) is stale — see TORVALDS-27. Live tree = Rust kernel/engine/tools + bebop2 mesh + a static-SPA server.

| # | Pattern | Verdict | Key evidence |
|---|---------|---------|--------------|
| 1 | Idempotency | **DONE** | `kernel/src/event_log.rs:7,257,349-351` — content-addressed, dup = structural no-op (P07 fix present) |
| 2 | Saga | PARTIAL | `kernel/src/domain.rs:257-264,359,377` compensation FSM; single-process, no distributed orchestrator |
| 3 | CQRS | **DONE** (in-proc) | `kernel/src/ports/payment.rs:174-182` events=write / orders=projection; `ports/agent/scope.rs:116` ReadProjection |
| 4 | Event sourcing | **DONE** | `bebop2/core/src/event_log.rs:1-16` SHA3-256 hash-chain; `kernel/src/event_log.rs:1-23` |
| 5 | Circuit breaker | **MISSING** | no half-open/auto-recovery anywhere; `kernel/src/hydra.rs:195,224` is a relief-trip, `mesh-node/src/kill_switch.rs` is permanent |
| 6 | Load balancing | MISSING | single systemd process (`deploy/native-spa-server.service:23,30`); no LB config exists |
| 7 | API gateway | PARTIAL | `native-spa-server/src/lib.rs:93` static-only; `proto-cap/src/hybrid_gate.rs` is crypto admission, not routing |
| 8 | Connection pooling | PARTIAL | PgBouncer sidecar **commented out** (`deploy/pgrust.toml:17`, `pgrust.env`); PgStore is a stub |
| 9 | Rate limit/token bucket | **DONE** | `kernel/src/token_bucket.rs`; consumed `llm-adapters/src/dispatch.rs:69`, `agent-adapters/src/dispatch.rs:155` |
| 10 | Opt/pess locking | PARTIAL | CAS (`token_bucket.rs:13`, `bebop2/core/src/lib.rs:68`), per-actor `actor_seq` (`event_log.rs:131`); no SQL row locking (no relational schema) |
| 11 | Sharding | PARTIAL | HRW hub-ownership partitioning `bebop2/delivery-domain/src/hub_ring.rs:5-12`; no DB sharding |
| 12 | Consistent hashing | **DONE — confirmed HRW/rendezvous** | `bebop2/proto-cap/src/matcher.rs:1-9` + `hub_ring.rs:5-12` explicitly "Highest-Random-Weight (HRW / rendezvous)" |
| 13 | Batch processing | PARTIAL | SIMD batch lanes `kernel/src/simd.rs:165,297`; no job scheduler |
| 14 | Dead-letter queue | PARTIAL | `tools/async-spool/src/main.rs:107,186` has it; deployed `rust-spool` does NOT (TORVALDS-07) |
| 15 | Blue-green deploy | MISSING | restart-in-place systemd only; no slots, no strategy files |
| 16 | Failover/fallback | PARTIAL | pervasive fail-CLOSED (`domain.rs:194,248`, `kill_switch.rs`) — refuse-on-doubt, not failover-to-backup |
| 17 | Service discovery | **DONE** (mesh) | `proto-wire/src/lib.rs:31-32` gossip roster; 3-node QUIC convergence proven `tests/mesh_sync_integration.rs:610` |
| 18 | DB-per-service | SHARED (stated) | one pgrust store/node, RLS deny (`deploy/pgrust.toml:19-22`); mesh sync across nodes |
| 19 | Sidecar | PARTIAL | PgBouncer sidecar documented, disabled (`deploy/README.md`, `pgrust.toml:17`) |
| 20 | Strangler fig | **DONE** | TS fully retired to `archive/`+`attic/`; Rust replacements self-document (`crates/bebop/src/portkey.rs:3`); seams = store traits. (Contrast: the TELEMETRY port violated this exact discipline — TORVALDS-01) |
| 21 | Bulkhead | PARTIAL | genuine per-agent cache isolation `agent-adapters/src/cache.rs:1-8`; no pool-level bulkheads (and lib.sh flock is an anti-bulkhead — TORVALDS-08) |
| 22 | Externalized config | **DONE** | `native-spa-server/src/main.rs:5` env/CLI only; `deploy/pgrust.toml`+`.env` split; secrets untracked |
| 23 | Health checks | **DONE (design) / degraded (live)** | `/healthz` `native-spa-server/src/lib.rs:114-115`; `/health` exporter — currently dead (TORVALDS-01) |
| 24 | Distributed tracing | MISSING | only an envelope correlation id `proto-wire/src/envelope.rs:7,23`; no spans/OTel usage |
| 25 | Audit trail | **DONE** | hash-chained event log with full-chain `verify()` (`bebop2/core/src/event_log.rs:1-16`) |
| 26 | Soft delete | MISSING (by design) | append-only log + compensation instead; no `deleted_at` anywhere — acceptable, but state it, don't discover it |
| 27 | Status/FSM | **DONE** | `kernel/src/order_machine.rs:8,78,139` transition table + `assert_transition` enforced at mesh edge (`delivery-domain/src/intake.rs:19-20,44`) |
| 28 | Counter | DONE (minor) | `event_log.rs:131` actor_seq; `typed_metrics.rs:17` |
| 29 | Pagination | MISSING | no data-listing API exists to paginate; `cursor` hits are hash-chain walkers (`event_log.rs:476-503`) |
| 30 | DB indexes | MISSING/MINIMAL | only DDL: single KV PK (`kernel/src/retrieval/memory_store.rs:158`); PgStore stub |
| 31 | Read replicas | MISSING | single pgrust node; mesh gossip is multi-writer sync, not read fan-out |
| 32 | Cache layer | **MISSING — documented P0, unaddressed** | see TORVALDS-12 |

### [SEVERITY: HIGH] [DESIGN] TORVALDS-12
**Where:** `docs/regressions/REGRESSION-LEDGER.md:59`; `docs/design/owner-token-revocation/breaker-findings.md:28-48`; live tree
**What:** The caching gap — the codebase's own 4x-corroborated P0 — remains completely unaddressed: no cache layer exists in the current tree, and the one in-proc fix that existed died with the retired TS app.
**Evidence:** REGRESSION-LEDGER:59 records the storefront "blinks empty" under load ("no server-side cache on the hottest read"). Grep for redis/memcached/distributed cache in live code → only OTel semconv definitions inside `node_modules`. In-proc memoization exists (`llm-adapters/src/cache.rs`, `agent-adapters/src/cache.rs`, `kernel/src/spectral_cache.rs:64`) but nothing serves the hot read path.
**Why it matters:** Your own documents call this the only ecosystem gap, four separate research passes agreed, and the tree still contains nothing. Known-P0 + zero motion = process failure, not knowledge failure.
**Fix guidance:** When PgStore/W13 lands, land the read-path cache in the same wave; until then put the requirement in the W13 blueprint's DoD so it can't slip again.

### [SEVERITY: LOW] [DESIGN] TORVALDS-13
**Where:** patterns 5/8/24 above
**What:** Circuit breaker, wired connection pooling, and distributed tracing are absent — tolerable TODAY only because there is no live deployment (P45:83-85 "zero live deployment"), but all three become launch blockers the day P37 ships.
**Evidence:** table rows 5, 8, 24 above; `BLUEPRINT-P45-ops-security-monitoring.md:83-85`.
**Why it matters:** These are the patterns that turn "one slow dependency" into "everything is down". Building them after launch means building them during an incident.
**Fix guidance:** Add breaker + pool wiring + trace-id propagation to the P37/W13 DoD explicitly.

---

## Part 3 — Bugs and logical errors (Rust sweep)

Unwrap/expect discipline is genuinely good in most crates (engine: 0 prod hits; async-spool, native-spa-server, loop-signals, rust-spool, wasm-host: 0). The problems concentrate where it hurts:

### [SEVERITY: HIGH] [CODE] TORVALDS-14
**Where:** `kernel/src/backup.rs:198,209,217`
**What:** `FileBlockStore::put()` PANICS on `create_dir_all`/`fs::write`/`fs::rename` failure — a full disk crashes the process — despite the trait returning `bool` precisely so failure can be signalled.
**Evidence:** three `panic!` calls in the I/O path of the backup primitive; the same file's crash-atomic tmp+rename design (`:416` `RestoreError`) shows the author knows how to do it right.
**Why it matters:** The BACKUP path is the code most likely to run during ENOSPC — the exact condition it panics on. A backup system that dies when the disk is full is a punchline.
**Fix guidance:** Return `false` (or better: convert the trait to `Result`), log the error; never panic on I/O in a durability primitive.

### [SEVERITY: HIGH] [CODE] TORVALDS-15
**Where:** `bebop2/proto-wire/src/discovery.rs:184,192,226,233,239,243,351,354,355`; `iroh_transport.rs:303,318`; `transport_policy.rs:50`; `wss_transport.rs:124,677`
**What:** The long-lived mesh discovery daemon uses bare `.lock().unwrap()` on the gossip hot path — one panic while merging an EXTERNAL frame under the lock poisons it and cascade-kills discovery for the whole node.
**Evidence:** 9 bare unwrapped locks in `discovery.rs` alone, on paths that process remote input. The kernel already has the correct pattern in-tree: `kernel/src/budget.rs:167` `.lock().unwrap_or_else(|e| e.into_inner())`.
**Why it matters:** Availability bug in the mesh's nervous system, triggered by whatever malformed input first finds a panic path. Your own kernel does it right; the mesh copied the wrong idiom.
**Fix guidance:** Adopt `budget.rs`'s poison-recovery pattern (or `parking_lot`) across proto-wire.

### [SEVERITY: MEDIUM] [CODE] TORVALDS-16
**Where:** `kernel/src/householder.rs:457`, `spectral.rs:364`, `retrieval/diffusion.rs:137`; `bebop2/core/src/dmd.rs:114`, `field.rs:119,412,524`, `linalg.rs:243,255`, `resonator.rs:316`
**What:** `.partial_cmp().unwrap()` in float sort comparators panics on NaN — and these sorts order eigenvalues/rankings of possibly-degenerate matrices, exactly where NaN is born.
**Evidence:** nine sites across both repos' spectral stacks, same anti-pattern.
**Why it matters:** One degenerate input matrix → NaN → panic in the middle of spectral ranking. The Lyapunov-NaN-guard branch that appeared in the swarm suggests NaN is not hypothetical here.
**Fix guidance:** `f64::total_cmp` everywhere. It exists since Rust 1.62; there is no excuse.

### [SEVERITY: MEDIUM] [CODE] TORVALDS-17
**Where:** `kernel/src/kalman.rs:260`
**What:** Kalman gain computation `expect("gain: S must be invertible")` — a genuinely singular innovation covariance (numerically plausible) panics instead of degrading.
**Evidence:** `mat_inverse(&s).expect(…)` in the update step.
**Why it matters:** GPS/ETA pipelines feed this; a co-linear measurement burst shouldn't kill the process.
**Fix guidance:** Return the prior (skip update) or regularize S on inversion failure.

### [SEVERITY: MEDIUM] [CODE] TORVALDS-18
**Where:** `engine/src/zerocopy.rs:129,130,230`
**What:** `unsafe from_raw_parts` views into wasm linear memory at a caller-supplied `offset` — sound ONLY if `offset + count*4 ≤ mem.len()` is checked by every caller, which this audit could not confirm.
**Evidence:** three unsafe raw-slice constructions over guest memory.
**Why it matters:** If any call path lets guest-influenced offsets through unchecked, that's an out-of-bounds read across the sandbox boundary — the one place `unsafe` must be paranoid.
**Fix guidance:** Move the bounds check INTO the unsafe helper (checked constructor returning `Option<&[f32]>`), don't trust call sites.

### [SEVERITY: MEDIUM] [CODE] TORVALDS-19
**Where:** `bebop2/proto-wire/src/iroh_transport.rs:195-197`
**What:** `InsecureAcceptAny` — a TLS verifier that accepts ANY certificate — with trust delegated entirely to the app-layer signed envelope (`HybridGate::RequireBoth`).
**Evidence:** `unsafe impl Sync for InsecureAcceptAny` (:197) — the impl itself is sound; the accept-any-cert design is the hazard, documented at :195.
**Why it matters:** The claim "envelope verification covers it" is plausible but unproven against active-MitM metadata attacks (traffic analysis, connection hijack pre-envelope). Defensible design, but it deserves an adversarial review, not a code comment.
**Fix guidance:** Security-review ticket: enumerate what a MitM gets BEFORE envelope verification rejects them; consider pinning node certs to mesh identity.

### [SEVERITY: MEDIUM] [CODE] TORVALDS-20
**Where:** `bebop2/core/tests/eigensolver_parity.rs:20,23`; kernel `householder.rs`/`spectral.rs`/`markov.rs`
**What:** FOUR eigensolver authorities (three in-kernel families + bebop2's own stack) with the cross-repo parity — the one that matters — left as a TODO: bebop2 names the dowiz kernel "AUTHORITATIVE" and then never asserts agreement with it.
**Evidence:** kernel families: Householder/Jacobi (`householder.rs:223,387` — note `:499-503` documents the TQL2 route FAILED its KAT and sits disabled next to the Jacobi fallback), Faddeev-LeVerrier+Durand-Kerner (`spectral.rs:113,141`), Markov's own power iteration (`markov.rs:158,202`). In-kernel reconciliation KATs exist (`householder.rs:878`, `spectral_laplacian.rs:245`) — good. Cross-repo: `eigensolver_parity.rs:20` "TODO(cross-repo): also assert agreement with the AUTHORITATIVE dowiz kernel eigensolver" — not enforced.
**Why it matters:** Two repos computing "the same" spectra with unbound implementations WILL drift, and the drift will surface as a physics/ranking discrepancy nobody can bisect.
**Fix guidance:** Ship shared KAT vectors (JSON fixtures both repos test against) — no code sharing needed, just shared truth.

### [SEVERITY: MEDIUM] [CODE] TORVALDS-21
**Where:** `engine/src/field_frame.rs:114` vs `kernel/src/csr.rs:545-553`, `spectral.rs:449-452`, `spectral_laplacian.rs:35`
**What:** Two things named "laplacian" with OPPOSITE signs — engine's stencil is `−(D−A)` (physics `∇²`, center −4), kernel's is `+(D−A)` (PSD graph Laplacian) — with no adapter or convention note binding them.
**Evidence:** cited lines; the E1 incidence work (`incidence.rs:231`) parity-checks within the kernel convention only.
**Why it matters:** Anyone moving code across the boundary flips every stability/monotonicity sign silently. This audit's own session history shows the sign split was "discovered" once already — it will be rediscovered until it's documented at both definition sites.
**Fix guidance:** One doc-comment at each site naming the convention and pointing at the other; a sign-convention KAT (apply both to the same 3-node path graph, assert the relation).

### [SEVERITY: LOW] [CODE] TORVALDS-22
**Where:** `tools/telemetry/rust-spool/src/main.rs:130-151` = `tools/async-spool/src/main.rs:246-267`
**What:** `backoff_delay` + `jitter_unit` duplicated character-for-character between the two spool drainers — self-documented as "a conscious, accepted trade".
**Evidence:** identical constants, seeds, formula, and the same ADR citation in both doc comments.
**Why it matters:** Acceptable AS LONG AS both live; but TORVALDS-07 shows the real fix is retiring rust-spool, which dissolves the duplication for free.
**Fix guidance:** Retire rust-spool in favor of async-spool; the duplication then deletes itself.

### [SEVERITY: LOW] [CODE] TORVALDS-23
**Where:** `bebop2/proto-crypto/src/{ladder,fips_regen,wycheproof,constant_time}.rs`, `lib.rs:12-50`
**What:** The crypto VALIDATION harness (Wycheproof vectors, FIPS re-implementations, constant-time assertions) is a skeleton of `TODO(P0-6/H)` bodies while the crypto it would validate ships.
**Evidence:** module docs state "bodies are TODO markers"; 8 TODO sites, all tracked under one named ticket. To be fair: no FIXME/XXX/HACK exists anywhere in either repo, and real validation DID happen (the SSR-2020 batch-verify forgery was found and fixed).
**Why it matters:** Tracked debt, honestly labelled — but crypto without its planned adversarial vectors is running on the last review, not continuous proof.
**Fix guidance:** Wycheproof JSON loading first (cheapest, highest yield); the constant-time shims can wait for dudect wiring.

---

## Part 4 — Error handling / logging / versioning / packaging

### [SEVERITY: MEDIUM] [CONVENTIONS] TORVALDS-24
**Where:** `kernel/src/money.rs:17,71,92,105,164,188,256`, `cart.rs:42,90`, `wasm.rs` (~40 fns) vs 21 typed error enums elsewhere in the same crate
**What:** No error-handling convention: the SAME crate mixes hand-rolled typed enums (`MatrixError`, `TransitionError`, `StoreError`…) with stringly-typed `Result<_, String>` — and the String variant owns the MONEY core.
**Evidence:** zero `thiserror`/`anyhow` in any dowiz Cargo.toml (bebop declares both at `crates/bebop/Cargo.toml:25-26` and then hand-writes `impl Display` anyway — `proto-wire/src/error.rs:48,70`, `proto-cap/src/error.rs:70,104`).
**Why it matters:** String errors in the money path mean callers match on substrings or give up — unexhaustive, untyped, unrefactorable. Money errors are exactly the ones you want the compiler to force you to handle.
**Fix guidance:** One convention (hand-rolled enums are fine — just be consistent); migrate `money.rs`/`cart.rs` first.

### [SEVERITY: MEDIUM] [CONVENTIONS] TORVALDS-25
**Where:** kernel+engine: 6 `tracing::` call sites vs 30 `println!` + 13 `eprintln!`
**What:** Structured logging is a dependency declaration, not a practice — `tracing` is in Cargo.toml (`kernel/Cargo.toml:74,79`) while actual runtime output is printf.
**Evidence:** counts above; JS side has no logger at all (no pino/winston; `web/src` console calls only in smoke harnesses). The only structured stream is the telemetry JSONL bridge — which TORVALDS-09 shows corrupts its own records.
**Why it matters:** When the first real incident hits, the debugging surface will be uncorrelated stdout fragments from a dozen unsupervised processes.
**Fix guidance:** Pick tracing-subscriber JSON output for the daemons that matter (drainer, exporter, mesh node); leave demos alone.

### [SEVERITY: LOW] [CONVENTIONS] TORVALDS-26
**Where:** `CHANGELOG.md:1-4` (both repos), all Cargo.toml/package.json
**What:** Three uncoordinated versioning schemes: CalVer changelogs (`2026.07.0`), frozen SemVer crates (all dowiz `0.1.0`; bebop mixed `0.1/0.2/0.4`), ad-hoc package.json (`2.2.3`, `1.0.0`, `0.5.0`) — and no document reconciles them. dowiz also has NO workspace `Cargo.toml` (standalone crates, path deps, two tools with isolating `[workspace]` stanzas).
**Evidence:** agent sweep, confirmed: editions uniformly 2021, no wildcard deps (good); `engine/Cargo.toml:21` path-dep to kernel.
**Why it matters:** "What version is deployed?" currently has no answer. Pre-1.0 this is survivable; it becomes a real problem the day two nodes must negotiate compatibility (the mesh's ALPN/protocol versioning will force this).
**Fix guidance:** Declare CalVer authoritative for releases, keep crate SemVer for API breaks, write the one-paragraph policy into CHANGELOG.md's header; add a dowiz workspace root.

### [SEVERITY: MEDIUM] [CONVENTIONS] TORVALDS-27
**Where:** `.claude/CLAUDE.md` (Repowise block); `docs/audit/RELEASE-GATE.md:3`; `BLUEPRINT-P45…:52`
**What:** Documentation actively lies about the present: the CLAUDE.md index describes a 145-file TS API with `server.ts` entry points and hotspots that have 0 tracked files; RELEASE-GATE documents Fly.io rollback for a server that no longer exists; P45:52 claims spool reporting "never goes silent" — falsified live this session.
**Evidence:** `git ls-files apps/ packages/` → empty; this audit's own scope brief cited the stale paths.
**Why it matters:** Stale ground-truth docs redirect every agent (including this one) into auditing ghosts. In an agent-operated repo, wrong docs are wrong CODE.
**Fix guidance:** Re-index Repowise; add tombstone headers to RELEASE-GATE.md; amend P45:52 with a pointer to this audit's Part 1.

---

## Part 5 — Backups / rollback / SPOF

### [SEVERITY: CRITICAL] [OPS] TORVALDS-28
**Where:** `/root/dowiz/.env` (mode **666**, single copy); `/usr/local/bin/dowiz-backup-cleanup.sh`; `BLUEPRINT-P45…:218`
**What:** The secrets file — `JWT_PRIVATE_KEY`, `COURIER_PII_ENCRYPTION_KEY`, `CLOUDFLARE_API_TOKEN`, `TELEGRAM_BOT_TOKEN`, DB URLs — is world-writable, exists in exactly one copy, and is excluded from the only backup automation. P45's own words: "НЕМАЄ off-Hetzner копії НІЧОГО → один скомпрометований Hetzner-акаунт = total-loss".
**Evidence:** `ls -la .env` → `-rw-rw-rw-`; backup script subjects are only `/root/.backups`, `/root/cleanup-safety`, rotated `/var/log` — verified by reading it. Losing the box loses the PII encryption key → any encrypted PII becomes permanently unrecoverable. The rclone S3 credentials (`/root/.config/rclone/rclone.conf`) — the keys to the backup bucket — also exist only on the box they're meant to outlive. Off-site target is a SINGLE Hetzner account (S3 fsn1), not off-provider.
**Why it matters:** Every durability guarantee in the system bottoms out in one file, one account, one box. This is the single most consequential operational fact in the audit.
**Fix guidance:** `chmod 600 .env` today; encrypted off-provider copy of `.env` + `rclone.conf` (age/sops to a second provider or offline); add both to the backup script's subjects.

### [SEVERITY: HIGH] [OPS] TORVALDS-29
**Where:** `/etc/systemd/system/cloudflared.service` (mode 644)
**What:** The Cloudflare tunnel JWT is hardcoded in plaintext in the `ExecStart` line of a world-readable unit file.
**Evidence:** verified live: `-rw-r--r--`, token `eyJ…` embedded (1 grep hit).
**Why it matters:** Any local process/user can read the credential that fronts `webhook.dowiz.org`; it also can't be rotated without editing a unit file nobody monitors.
**Fix guidance:** Move to `--token` via `EnvironmentFile=` with 600 perms, or credentials-file mode; chmod the unit.

### [SEVERITY: MEDIUM] [OPS] TORVALDS-30
**Where:** `kernel/src/backup.rs` (never exercised e2e — P45:63); `BLUEPRINT-DISK-OPS…:71-98` (scripts that don't exist)
**What:** The backup story is a well-built primitive that has never run end-to-end, plus designed-only automation: `disk-target-sweep.sh`/`disk-alert.sh` exist as blueprint code blocks, not files.
**Evidence:** P45:63 "REAL, unit-proven, never exercised end-to-end — nothing to back up yet"; `scripts/` contains no disk/backup scripts (verified); DISK-OPS:18 admits the memory claim of "deep-clean tool + cronjobs" is stale — corroborated in Part 1 (deep-clean has never run; its log dir doesn't exist).
**Why it matters:** An untested backup is a hypothesis. The blueprint's own "designed once, silently stopped" diagnosis applies to the blueprint.
**Fix guidance:** P45 §4a.3's synthetic-fixture backup drill is W0 (no blockers) — run it; land the two scripts or delete them from the blueprint.

### [SEVERITY: MEDIUM] [OPS] TORVALDS-31
**Where:** `docs/audit/RELEASE-GATE.md:3` vs P45 §4a.1
**What:** Two rollback designs exist — one stale (Fly.io blue-green for a deleted server), one unbuilt (releases/<sha> symlink swap, W1-blocked) — and zero working rollback.
**Evidence:** no `fly.toml`, no `deploy-staging.sh` in tree; P45 §4a.1 all-designed.
**Why it matters:** "Rollback" currently means `git revert` and hoping. Fine pre-launch; fatal after.
**Fix guidance:** Tombstone the Fly doc now; build the symlink path with P37.

### [SEVERITY: LOW] [OPS] TORVALDS-32
**Where:** `/mnt/volume-fsn1-1` (84% of 49G)
**What:** The "backup" volume is a single near-full disk mixing ad-hoc root snapshots, ollama model weights, and living memory — one volume failure takes the fallback tier AND the memory corpus.
**Evidence:** `backup-root-2026-07-13/`, `bebop-P4-backup/`, `pruned-root-2026-07-15/`, `dowiz-memory/`, `ollama/` all on one mount; 39G/49G used.
**Why it matters:** Co-locating the safety copy with model-weight ballast on a filling disk is how safety copies get deleted "to free space".
**Fix guidance:** Ollama weights are re-downloadable — move them off the backup volume; that alone frees the pressure.

---

## GENUINE — what is actually well built (no manufactured negativity)

1. **The event-sourced money core is real engineering.** Hash-chained, content-addressed event log where a duplicate is a structural no-op (`event_log.rs:7,257`); compensation FSM that nets cancelled orders to exactly zero (`domain.rs:257-264`); audit trail and event store are the same verified object. This is the right architecture, correctly executed.
2. **The HRW matcher IS rendezvous hashing, done properly** — deterministic, coordination-free, weight-over-identity-never-score (`matcher.rs:1-9`, `hub_ring.rs:5-12`), with the trust-is-capability-never-reputation stance enforced in code comments and guards.
3. **Test discipline is genuinely strong**: ~559 kernel tests green (452 default + 107 pq-KAT per GROUND-TRUTH-2026-07-17), parity KATs binding every acknowledged dual implementation (ema↔Kalman↔eqc_gen bit-parity `geo.rs:552-556`, `kalman.rs:396`; eigensolver reconciliation `householder.rs:878`), a real chaos-injection module (`chaos.rs`), and a mesh integration test that proves 3-node gossip convergence over real QUIC.
4. **The crypto work is honest.** The Ed25519 batch-verify SSR-2020 forgery was found by genuine adversarial review, fixed correctly (every batch accept re-verified singly), and the perf claim was publicly walked back (`84a1e272d`) — correctness over marketing. Infallible `unwrap`s on fixed-width slices are genuinely guarded (`pod.rs:109-110` length-checks first). No `FIXME`/`XXX`/`HACK` anywhere; every TODO is named and tickised.
5. **Several crates are spotless**: engine, async-spool, native-spa-server, loop-signals, wasm-host — zero production unwraps. `budget.rs:167` shows the correct poison-resilient lock idiom. `async-spool` (deadletter, jittered backoff, malformed-line quarantine) is strictly better engineering than its predecessor — the fix for TORVALDS-07 is already written, it just isn't deployed.
6. **Fail-closed as a reflex** — money/domain/mesh all refuse on doubt (`domain.rs:194,248`, kill_switch, HybridGate RequireBoth). Wrong for availability patterns (no failover), but the SAFE wrong.
7. **`backup.rs`'s write path** (tmp + rename + content re-hash, fail-closed restore) is the right design — TORVALDS-14 is a 3-line fix away from a good primitive.

---

## Bottom line

The kernel/mesh code is better than most production systems — tested, honest, adversarially reviewed.
The OPERATIONS layer around it is a graveyard of half-finished migrations run by unsupervised ghost
processes. The pattern is consistent: build the replacement's skeleton, delete the working original,
never wire the new thing, never supervise anything, let /tmp and reboots collect the corpses. The code
knows better (strangler-fig was executed correctly for the TS app; async-spool fixed rust-spool's
flaws) — the discipline just doesn't reach the ops surface. Fix the process, not just the findings:
nothing gets deleted until its replacement is running under systemd and observed green.
