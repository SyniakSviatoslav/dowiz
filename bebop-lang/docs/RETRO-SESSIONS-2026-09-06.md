# Retrospective: bebop-lang Claude Code sessions 2026-09-04 .. 2026-09-06

Status: 2026-09-06 REPORT (analyst agent, session 15; copied from the scratchpad unedited; the 12 proposed
improvements in §5 await the operator's pick -- none applied yet)

Sources: 24 transcripts in /root/.claude/projects/-root/*.jsonl (python3 extraction; script at
scratchpad/extract.py), docs/SESSION-HANDOFF.md:49-65, AGENTS.md:179-388 (L1-L21),
docs/BUG-LEDGER-WEEK.md, docs/DEV-LOOP.md:67-72, docs/exp.journal (764 lines), HISTORY.md,
memory/*.md, docs/ANALYSIS-2026-09-06.md §4, `git log -- bebop-lang` (591 commits, 94 fix/revert).
Times are transcript UTC. "Died" = the last assistant message is mid-task ("чекаю", "запускаю"), no
closing report, and the next session opens with "продовжуй з попередньої сесії".

## 1. Session inventory

| # | transcript | start (UTC) | dur | tools | err | commits | headline task | headline failure |
|---|---|---|---|---|---|---|---|---|
| 1 | 640ab602 | 09-04 12:14 | 27m | 63 | 5 | 0 | T13 register-window fixpoint probes | segfault + Bus error probing, 4x identical retried command; died mid-analysis |
| 2 | 7d917ec2 | 09-04 12:21 | 1h14 | 101 | 4 | 2 | roadmap analysis, corpus A, token tools | found working-tree bebop.bp NOT at fixpoint (13:23); headroom init blocked by classifier |
| 3 | 7c34b696 | 09-04 12:29 | 11m | 21 | 0 | 0 | activate graphify/mempalace/rtk | died after first orientation |
| 4-6 | 984647f7, 9b97d025, f10da1fe | 09-04 13:37-13:45 | 2-5m | 0 | 0 | 0 | same roadmap prompt three times | zero tool calls, no assistant text; operator: "у чому проблема" x2 (startup/MCP failure) |
| 7 | b6e68ef3 | 09-04 13:52 | 23m | 49 | 1 | 1 | roadmap start, parallel agents | died at "запускаю разом" |
| 8 | 9008cf94 | 09-04 14:19 | 21m | 53 | 4 | 2 | 4 parallel agents (T31/T39/T66/T44) | `sed: can't read ROADMAP.md` x3 (cwd reset); died waiting for 4 agents |
| 9 | f901d449 | 09-04 15:04 | 36m | 31 | 0 | 2 | 3 agents, T66 | died waiting ("чекаю на результати") |
| 10 | 97af3208 | 09-04 15:51 | 46m | 62 | 0 | 3 | 2 agents; status/% | operator: "телефон уже фрізить, раніше вибивало на 3"; "чому не використовуєш graphify + mempalace?" |
| 11 | 0e0fc106 | 09-04 16:37 | 8m | 21 | 1 | 1 | 2 agents, T42 | `sleep 60` blocked; died at 8 min |
| 12 | 7db08f69 | 09-04 16:53 | 41m | 47 | 0 | 2 | T42 s16, DIVERGE-81/128 ddmin | died waiting on 2 agents' VERDICT blocks |
| 13 | 7737987f | 09-04 17:36 | 31m | 82 | 5 | 2 | T42 unary, 1 agent | exit 144 self-kill x2 (`pkill -f prec_switch.sh`, `pkill -f 'std_tests/lcjit.bp'`); died |
| 14 | cd24782f | 09-04 18:08 | 13h53 | 721 | 24 | 41 | T96/T99/T118/T108/T109/store G1-G8 (marathon) | exit 144 x3 (`pkill -f "bat3_"`, `"BEBOP_BIN=$S/fx/q3.bin"`), context exhausted 02:08, wrong hand-typed constant in T130 (07:28), 52 sleep-poll commands |
| 15 | aedc15c7 | 09-05 08:24 | 5h58 | 244 | 7 | 6 | T123 mutation sweep, T80, DIVERGE-20056 | one Bash call hung 44 min (13:16->14:00); "cwd скинулось на /root після push" (14:19); died |
| 16 | 41a6580d | 09-05 14:50 | 16m | 33 | 4 | 0 | dev-speed measurement | `kill 10016 ...` blocked by classifier; died at 16 min |
| 17 | 36c1706b | 09-05 15:07 | 1h08 | 61 | 0 | 1 | DIVERGE-42122 real miscompile fixed (4f6aaf2) | died after starting 5000-seed fuzz batch (~25 min job) |
| 18 | 016340f6 | 09-05 17:53 | 6m | 8 | 0 | 0 | report dev-speed numbers | died right after "Запускаю chain з батареєю у фоні" |
| 19 | 0e30456f | 09-05 18:01 | 16m | 38 | 2 | 2 | fuzzd.sh, ladder | died (this is "session 12", proc count 38 > 32 per 1f1d2822 19:14) |
| 20 | 18ab7ddb | 09-05 18:20 | 41m | 48 | 8 | 0 | boxguard (operator priority) | phantom-process cap 32 discovered; `kill -9 <orphans>` blocked by classifier x3; adb pairing unfinished |
| 21 | 1f1d2822 | 09-05 19:02 | 1h05 | 121 | 6 | 9 | mode-000 root cause (e7cc07f), fuzzd, L20 | opener "блять з минулої сесії викинуло раптово через бокс"; 4 classifier blocks (Agent spawn, update-config SessionStart hook) |
| 22 | 2674ef2b | 09-05 20:20 | 19m | 56 | 0 | 3 | honest.sh, T90 step 2 | died at 18 min (after boxguard existed) |
| 23 | 9500d28f | 09-05 20:42 | 5h58 | 67 | 1 | 7 | reap.sh/L21, T90 2b/2c brk stub | 313-min gap waiting for operator answer (21:22->02:34); hand brk constant wrong, python value saved it (21:22) |
| 24 | ae87ba63 | 09-06 05:30 | 31m+ | 52 | 0 | 2 | evals research, T96 P3 | (running) |

Totals: 24 sessions in ~44 h wall; ~2,050 tool calls; 74 tool errors; 94 commits; 17 of 24 sessions
ended mid-task (died). Median session length excluding the two marathons: 23 min.
Commits by day: 09-04 38, 09-05 53, 09-06 3 (git log, bebop-lang paths).

## 2. Error / bug pattern catalogue (ranked by cost)

### P1. Box death: over-parallelism + Android phantom-process cap (32) + orphaned proot shells
- Hit: ~17 sessions (every death in the table). 09-04 12:14-18:08 produced 8 dead sessions with 4 -> 3 -> 2 -> 2 -> 1 agents; 09-05 five more (16, 17, 18, 19, 22).
- Mechanism: environment (Termux/proot, no cgroups, Android kills the app at 32 phantom procs; a dead session leaves ppid-1 shells spinning at 100 %) x model behaviour (spawning N agents each running batteries; `J>=3` fuzz; three parallel batteries) x process (no limit existed until 09-05 18:xx).
- Incidents: 97af3208 16:36 operator "телефон уже фрізить, раніше вибивало на 3"; 1f1d2822 19:14 "лічильник процесів 38 > 32 (ladder --jobs 3 + arch-тест + fuzzd)"; 18ab7ddb 18:38 "лічильник сягнув 32"; SESSION-HANDOFF:37-38 orphaned invariants.sh + 4 zombies at the cap again on 09-06.
- Cost: each death = lost in-flight work + 5-15 tool calls of re-orientation next session (~200 tool calls total on "Читаю пам'ять сесії та стан репо") + operator restarts. Estimate 5-7 h of the 44 h.
- Existing: L19 boxguard (09-05), L21 reap.sh (09-06), memory project-boxguard, hookify warn on pkill. Stopped recurrence? Partly: 2674ef2b still died 18 min after boxguard; 9500d28f then ran 6 h. adbfix (the actual cap removal) is still "pending" (memory project-boxguard.md). No mechanism limits agent count or J; it is prose in L19(c).

### P2. "transient COMPILEFAIL rc=90" — a phantom chased for 3 sessions; root cause proot mode-000 files
- Hit: cd24782f (9 rc=90 mentions), 18ab7ddb (attributed to the phantom cap, 18:38), 1f1d2822 (22 mentions; root cause e7cc07f 21:56), 9500d28f.
- Mechanism: environment (proot fake-root chmod race; coreutils `stat` under proot fakes modes, python os.stat shows real ones) x model behaviour (accepted the "transient / not reproducible" label and kept fuzz J<=2 as the workaround for ~24 h instead of a falsifiable probe; L10 "every probe states its expected value" was not applied to the flake) x wrong attribution (18ab7ddb blamed the phantom cap: "Це ж пояснює transient COMPILEFAIL rc=90").
- Incidents: journal 1788641800 (seed variant exiting -errno: EACCES), 1788642600 (fix, one word `mov x3,#0x1a4`); memory gate-discipline:19-21 "pitfall e is CLOSED".
- Cost: ~3-4 h across sessions plus a wrong constraint (fuzz J<=2) that slowed every fuzz batch.
- Existing now: fixed at the root (one emitter word). Lesson not mechanised: nothing forces a "transient" label to come with a journal line + errno.

### P3. Cascading fix chains on bebop.bp without an isolating probe
- Hit: 09-04: 10 fix/revert commits on bebop.bp in one day (b4326b5 10:09 disable window -> 9d9a2ba 10:10 revert -> ff45b17 15:54 flush_window -> 4e6a1d6 16:15 retire window; then six T42 fixes 18:17-20:15); 09-02: 6 fix commits (R6 series); 09-05: 4.
- Mechanism: model behaviour (fix-then-see) x process (L14/L15 written 09-02 but the T13 window churn happened 09-04 anyway; final resolution was an operator decision to retire the feature).
- Incidents: HISTORY.md:761-762, 1069, 2559 (b4326b5 gen3==gen4 corruption, x9-x13 window); BUG-LEDGER P3 "guards -> cap -> signature reorder -> revert chains".
- Cost: session 640ab602 (27 min, 63 calls) + 7d917ec2 fixpoint discovery + two commits/reverts; ~2-3 h on 09-04; the 09-02 week is the BUG-LEDGER.
- Existing: L14, L15, L6 — prose only; the T42 series is L14-compliant (one defect per commit) so partial success.

### P4. Hand-typed instruction constants (L1 is a dead letter for the model)
- Hit: 3 incidents in 3 days despite L1 ("Hand-typing a constant is a defect", AGENTS.md:181): cd24782f 07:28 "Fixing a wrong constant in the T130 patch" (clz became `rev`, passed tq/tdg by luck, only c41 caught it — memory:45-47); 9500d28f 21:22 "моя ручна константа brk була хибна, python-значення врятувало"; "the first entry stub wrote into the RX code image" (SESSION-HANDOFF:57-58).
- Mechanism: model behaviour (confidence in a remembered encoding) — the L1 pipeline (as -> objdump -> int) is only followed when the agent remembers to.
- Cost: ~1-2 h (T130 rework + a battery cycle each).
- Existing: L1 prose, pitfall f. Not stopped. No mechanism checks that a new `em(insns,n,<int>)` literal appeared in an objdump-verified form.

### P5. `pkill -f <literal>` kills the shell running it (exit 144)
- Hit: 5 incidents on 09-04 (7737987f x2 at 17:4x; cd24782f x3: `pkill -f "bat3_"`, `pkill -f "BEBOP_BIN=$S/fx/q3.bin"`, then the agent switched to `b[t]4`-style patterns). Memory says "three times"; transcripts show five.
- Mechanism: tool (Bash cmdline contains the literal) x model behaviour (bundling pkill with the real work in one call — the kill aborted a python patch in the same call, cd24782f line 829).
- Cost: ~30-45 min (each kill also destroyed the sibling command's output).
- Existing: hookify warn-pkill-literal (warn only), pitfall d, L20/L21 "kill by pid via reap.sh". Stopped: yes after 09-04 (2674ef2b uses `chain[.]sh`). Warn -> block would make it permanent.

### P6. Waiting instead of working (sleep-poll turns, blocking on operator answers)
- Hit: cd24782f 52 `sleep N` commands, 7737987f 13 (7 pure wait), 18ab7ddb 15, 1f1d2822 12, 9500d28f 11 + a 313-min idle gap (21:22 -> 02:34) waiting for a design answer; 7db08f69 and 0e0fc106 ended waiting for agents.
- Mechanism: model behaviour (poll loop as a turn) x process (the operator rule L18 only arrived 09-06 after "поки працюють шели не зупиняйся" on 09-05 11:18).
- Cost: ~1 h of pure sleeps on 09-04/05, plus 5.2 h wall in 9500d28f (partly by design: memory user-bebop-operator says ask before architecture decisions; L20 says never pause — the two rules conflict).
- Existing: L18, memory feedback-no-idle-waiting. Stopped: mostly (ae87ba63 2 sleeps) but the ask-vs-continue conflict is unresolved.

### P7. Codegen change -> battery RED by construction (forgot FREEZE=1 / census_allow line)
- Hit: 17 "battery: RED" mentions across 8 sessions (aedc 3, 2674 4, ae87 4, 9500 2 ...); memory gate-discipline:11 "Battery RED after a codegen shrink = c[onstructs re-freeze]"; 36c1706b 15:24 adding census_allow lines after the fact; SESSION-HANDOFF:21-22 documents the manual FREEZE=1 env + census_allow.txt line.
- Mechanism: process (tools/chain.sh --codegen does not export FREEZE=1 — verified: battery.sh:10 reads `${FREEZE:-0}`, chain.sh:9 only sets CG) x model behaviour (forgetting the env var).
- Cost: one extra ~125 s chain per forgotten run, x ~10 = ~25 min, plus misreads of RED as regression.
- Existing: prose in SESSION-HANDOFF; no mechanism.

### P8. Runner-scripts and shell traps: editing a running script, `S=` prefix, `&&`-list + `&`, cwd reset
- Hit: chain.sh "phantom syntax error" after editing it while it ran (memory:30, 1 incident); `S=` prefix clobbering the runner variable (SESSION-HANDOFF:61); `&&`-list backgrounded whole (memory:52-53); cwd reset to /root: 9008cf94 x3 errors, aedc15c7 14:19.
- Mechanism: tool/environment (bash reads scripts by offset; each Bash call resets cwd) x model behaviour (relative paths, editing live scripts).
- Cost: ~40 min total. Existing: prose only (pitfalls). The cwd one recurred in two sessions.

### P9. Timing gates flake under parallel load (lcjit) and boxguard SIGSTOP perturbs timings
- Hit: pitfall SESSION-HANDOFF:53; L19(a); ANALYSIS §4B "silent perturbation of every timing that is never recorded".
- Mechanism: environment x process (battery.sh runs lcjit inside the parallel shards; no `boxguard status` read before timing).
- Cost: reruns (~10-20 min) and possible false REDs. Existing: prose only.

### P10. Session startup / harness failures
- Hit: three consecutive sessions 09-04 13:37-13:45 with 0 tool calls and no assistant text (operator "у чому проблема" x2); mempalace MCP CONNECT_TIMEOUT 30 s again on 09-06; classifier blocks on `kill` (18ab7ddb x3, 41a6580d x1), on the Agent tool and on the update-config Skill (1f1d2822 x4) — the attempted SessionStart hook that would inject handoff state was blocked and never landed.
- Cost: ~20 min + operator frustration; the missing SessionStart hook indirectly costs every restart (P1).

### P11. Artifact identity / empty binary (historical, 09-02)
- BUG-LEDGER P1: `git show 8ec62df:bebop.bin` empty -> exit 91 chased as logic for hours; journal 1788288248 retraction. Guard: tools/guard_artifact.sh + harness preflights. Recurrence in 09-04..06 transcripts: none. This is the one mechanism that demonstrably worked.

### P12. cp over the mmapped bebop.bin -> SIGBUS in every running seed
- 1 incident (memory:30; hookify block-bin-overwrite cites it). Now blocked by hookify (block, not warn). No recurrence. Works.

## 3. Model-behaviour patterns (what Claude did repeatedly)

| behaviour | count | examples |
|---|---|---|
| Confident hand-typed instruction words despite L1 | 3 | T130 clz->rev (cd24782f 07:28); brk constant (9500d28f 21:22); entry stub into RX image |
| `pkill -f <literal>` in the same call as real work | 5 | cd24782f lines 732/746/829/923; 7737987f 595/617 |
| Spawning more agents than the box tolerates | 6 sessions | 9008cf94 4 agents, f901d449 3, 97af3208/0e0fc106/7db08f69 2 — all died; operator lowered the cap each time |
| Wait-only / sleep-poll turns | ~100 sleep cmds | cd24782f 52, 18ab7ddb 15, 7737987f 13 (7 pure waits) |
| Blocking on a question instead of continuing independent work | 1 big | 9500d28f 313 min; contrast 7db08f69 17:07 "Продовжую без блокування на відповідях" (good) |
| Accepting "transient" as an explanation | 3 sessions | rc=90 chased 09-04 -> 09-05 21:56 |
| Wrong attribution stated as fact | 2 | 18ab7ddb 18:38 rc=90 "explained" by phantom cap; ROADMAP projections wrong 10x (cd24782f 19:55 §4.2) |
| Forgetting FREEZE=1 / census_allow after codegen | ~10 | RED mentions in 8 sessions; 36c1706b 15:24 |
| Relative paths after cwd reset | 2 sessions | 9008cf94 (3 errors), aedc15c7 14:19 |
| Editing a running script / `S=` prefix / `&&…&` | 3 one-offs | memory:30, SESSION-HANDOFF:61-62 |
| Re-orientation cost at every restart (re-deriving state) | 24 x 5-15 calls | 640ab602 63 calls to "I have the full picture" then died; 0e30456f last msg "Читаю пам'ять сесії" |
| Over-long reports | some | cd24782f 19:55 a multi-thousand-word cost-model report mid-marathon; the operator's replies are 1 line |
| Trying `kill -9` by hand -> classifier block -> retry | 4 | 18ab7ddb x3, 41a6580d x1 (led to reap.sh, the right fix) |
| Not reaping before starting the next task | until 09-06 | L21 exists since 9500d28f; reap.sh mentioned 37x there, 8x in ae87ba63 — adopted |
| Repeating the same heredoc-python edit pattern 8-11x per session | cd24782f 11+9+8+8+7 | fine functionally, but each carries the full `S=…` prefix and prose; context ran out at 02:08 |

Dead letters (written, still violated in the window): L1 (3 violations), L14/L15 (09-04 T13 churn), L10 (rc=90 flake never given an expected value), L19(c) agent/J caps (prose, violated by 1f1d2822's `--jobs 3 + arch test + fuzzd` = 38 procs), the "FREEZE=1" note in SESSION-HANDOFF.
Live mechanisms that worked: guard_artifact.sh (P11), hookify block-bin-overwrite (P12), reap.sh (P1 partial), hookify block-tasks-md.

## 4. What would have prevented each pattern (cheapest mechanism)

| pattern | cheapest mechanism | effort | evidence it would have caught it |
|---|---|---|---|
| P1 box death | (a) apply adbfix (the cap itself); (b) PreToolUse hook on Agent: deny if `pgrep -c .` > 24 or if >1 Agent already running; (c) chain.sh/battery.sh/fuzz refuse to start when proc count > 26 (print `tools/reap.sh` output) | S | 1f1d2822 19:14: 38 procs from `--jobs 3 + arch test + fuzzd`; 9008cf94 4 agents; all deaths preceded by proc count > 32 |
| P2 phantom flake | rule: a "transient" verdict is illegal without a journal line carrying errno + an expected value (L10) — enforce via journal linter: `VERDICT:transient` requires `errno=` in the line | S | the flake was solved in 1 probe (1788641800) once it was actually probed |
| P3 fix chains | pre-commit hook: a `fix(` commit touching bebop.bp must reference a construct/neg gate added in the same commit (grep bench/parity_constructs diff) — L15 mechanised | M | T13 sequence had no probe until the operator retired it; T42 fixes each carried c26-c29 |
| P4 hand-typed words | tools/check_words.py: for every `em(insns, n, <int>)` literal added in the diff, require the int to appear in a scratch objdump listing or in a `# as:` comment; run in battery.sh | M | clz->rev word 2442396668 would not have matched any objdump output |
| P5 pkill self-kill | flip hookify warn-pkill-literal to `action: block` | S | all 5 commands match the existing regex |
| P6 waiting | PreToolUse regex block on `^\s*(cd [^;]*;)?\s*sleep \d+;?\s*(tail|cat|ls)` — forces Monitor/run_in_background or real work; ask-vs-continue: SESSION-HANDOFF line "design questions are asked in the closing report; never block a turn on them" | S | 8 pure-wait calls in cd24782f, 7 in 7737987f match |
| P7 FREEZE/census | chain.sh: `--codegen` exports FREEZE=1; invariants.sh prints the ready `census_allow.txt` line on a census delta | S | every RED-after-codegen in the 8 sessions |
| P8 shell traps | chain.sh/battery.sh: `exec` from a temp copy (`cp "$0" "$T/self.sh" && exec bash "$T/self.sh" "$@"`) so editing the file cannot change a running run; Bash usage line in SESSION-HANDOFF "always `cd /root/dowiz/bebop-lang;` first" (already an env note; the failures were relative paths after a `cd /root/dowiz` push) | S | memory:30 phantom syntax error; 9008cf94/aedc15c7 relative-path errors |
| P9 timing flakes | battery.sh: run lcjit (and any gate tagged `# timing`) after the shards finish, single-threaded, and print `boxguard status` first | S | pitfall SESSION-HANDOFF:53 "rerun standalone first" is exactly this, done by hand |
| P10 startup | mempalace MCP: raise timeout or make it lazy; SessionStart hook injecting SESSION-HANDOFF (blocked once by the classifier — do it by editing settings.json directly) | S | 3 dead starts 09-04 13:37-13:45; 30 s timeout today |
| re-orientation | the SessionStart hook above + keep SESSION-HANDOFF <= 60 lines (it is 65, OK) | S | ~200 tool calls across 24 restarts |

## 5. Proposed improvements (ranked)

1. **Process-count gate in the runners** — tools/chain.sh, tools/battery.sh, bench/fuzz/fuzz.sh, fuzz_batch.py: first line `tools/reap.sh --check 26 || exit 97` (reap.sh prints the count; exits non-zero above N). Effort S. Kills P1 (the mechanism side). Accept: with 27 dummy `sleep` procs, `tools/chain.sh` exits 97 and prints the reap list.
2. **Agent-spawn hook** — ~/.claude/settings.json PreToolUse matcher `Agent`: a command hook that denies when `ps -e | wc -l` > 24 or when another Agent is already running (a marker file in the scratchpad). Effort S. Kills P1 (the model side: 6 sessions died with 2-4 agents). Accept: spawning a second agent while one runs returns the hook's deny message.
3. **Apply adbfix** (~/adbfix.sh; memory project-boxguard "still pending") — removes the 32-phantom cap. Effort S (operator: one adb pairing). Kills P1 root. Accept: `adb shell device_config get activity_manager max_phantom_processes` = 2147483647; boxguard log shows no death for a session > 3 h with J=3.
4. **`--codegen` implies FREEZE=1 + auto census_allow line** — tools/chain.sh:9 `[ $CG = 1 ] && export FREEZE=1`; invariants.sh prints `census_allow.txt` candidate line on delta. Effort S. Kills P7. Accept: a one-word codegen edit through `chain.sh --codegen` ends GREEN without any env var.
5. **hookify warn-pkill-literal -> block** — change `action: warn` to `block` in ~/.claude/hookify.warn-pkill-literal.local.md. Effort S. Kills P5 permanently. Accept: `pkill -f "bat3_"` from a Bash call is refused.
6. **Sleep-poll block hook** — hookify rule, event bash, pattern `^\s*(cd [^;]+;\s*)?(S=[^;]+;\s*)?sleep \d+\s*;\s*(tail|cat|ls|wc|grep)`, action block, message "use Monitor or do the next item (L18)". Effort S. Kills P6 (poll side). Accept: `sleep 60; tail -3 log` is refused; `sleep 1; ps` still allowed.
7. **Word-derivation check in battery.sh** — tools/check_words.py: every new `em(insns, n, N)` / `st[i] = N` literal in `git diff bebop.bp` must be present in `$BEBOP_TMP/words.objdump` (produced by the L1 pipeline: the agent runs `as`+`objdump` into that file). Effort M. Kills P4. Accept: a diff adding a literal absent from the objdump file makes battery.sh RED with "unverified word N at line L".
8. **Self-copy exec in runners** — chain.sh/battery.sh line 2: `[ "${SELF_COPY:-}" ] || { cp "$0" "$OUT/.self.sh"; SELF_COPY=1 exec bash "$OUT/.self.sh" "$@"; }`. Effort S. Kills the P8 phantom-syntax-error class. Accept: edit chain.sh while a chain runs; the run finishes normally.
9. **Timing gates serialised** — battery.sh: move lcjit (tag `# timing` in std_golden.sh) to a final single-threaded stage preceded by `boxguard status`; write `stopped=` into the log next to the number. Effort S. Kills P9. Accept: three parallel batteries, lcjit passes 3/3.
10. **SessionStart hook injecting SESSION-HANDOFF + reap output** — /root/dowiz/.claude/settings.json `SessionStart` (matcher startup|compact) -> `cat bebop-lang/docs/SESSION-HANDOFF.md; bebop-lang/tools/reap.sh`. Effort S (edit the JSON directly; the Skill route was classifier-blocked). Kills the re-orientation tax and enforces L21 at start. Accept: a new session's first assistant turn already cites the HEAD hash without a Read.
11. **Journal linter for "transient"** — tools/journal_lint.sh (already implied by L20): a line containing `transient|flaky|not reproducible` must contain `errno=|rc=` and `EXPECT:`; run by the pre-push hook. Effort S. Kills P2 recurrence. Accept: appending "VERDICT:transient" without errno fails `git push`.
12. **Pre-commit probe rule for fix commits** — tools/hooks/pre-commit: a commit whose subject starts `fix(bebop)` and touches bebop.bp must also touch bench/parity_constructs/ or bench/diag_neg/ (L15 mechanised). Effort M. Kills P3 churn. Accept: `git commit -m "fix(bebop): x"` with only bebop.bp staged is refused with the L15 text.

Dead letters to retire or convert: L1 -> item 7; L14/L15 -> item 12; L18 -> item 6; L19(c) -> items 1-2; the FREEZE note in SESSION-HANDOFF:21 -> item 4; SESSION-HANDOFF:16 mentions `dev_loop.sh STEPS=<subset>` but tools/dev_loop.sh does not exist (docs drift; fix the line).
Also drop from the prose: pitfall e (closed) can go; keep the memory file under 60 lines by moving (a)-(m) into the AGENTS laws they duplicate.

## VERDICT
- The dominant cost was not compiler bugs but the box: 17 of 24 sessions died, almost all with >1 agent or >26 processes; the only real fix (adbfix) is still unapplied and the caps are prose (L19c).
- Mechanisms beat laws in this record: guard_artifact.sh, block-bin-overwrite and reap.sh stopped their classes; L1, L14/L15, L10, L18 were each violated after being written.
- Three rules can be mechanised for ~S effort each and would have caught ~80 % of the tool-error minutes: proc-count gate in runners, pkill warn -> block, `--codegen` implies FREEZE.
- The single most expensive phantom (rc=90, ~4 h over 3 sessions) fell to one falsifiable probe; enforce "no 'transient' without errno + expectation" in the journal linter.
- Wall-clock leaks: ~100 sleep-poll calls and one 5-hour block on a design question; resolve the L20 "never pause" vs "ask before architecture" conflict by asking in the closing report only.
