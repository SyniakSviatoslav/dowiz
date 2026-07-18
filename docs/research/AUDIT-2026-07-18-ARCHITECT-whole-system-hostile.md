# AUDIT 2026-07-18 — ARCHITECT: whole-system hostile review

> Reviewer stance: deliberately adversarial systems architect, business-first lens, hostile to
> this repo's own culture of self-certified rigor. Read-only pass over `/root/dowiz` +
> `/root/bebop-repo`; every number below was re-measured live this session (2026-07-18), not
> copied from the repo's own status docs. Where the repo's docs were right, I say so; where
> the rigor is theater, I say that too.

## Verification baseline (measured live, this pass)

| Claim under test | Measured 2026-07-18 | Verdict |
|---|---|---|
| "88.7k lines of planning docs" | `docs/design/*.md` = **129,741 lines** across **323 files**; repo-wide `.md` = **206,673 lines** | WORSE than claimed — the planning corpus grew ~46% past the session's own hostile number |
| "54.3k lines of code" | dowiz Rust = **60,064** (kernel 42,523; engine 3,181; adapters+tools rest); `web/` = **1,452 lines total**; bebop-repo Rust = 66,028 | Roughly held; dowiz planning docs alone are 2.2× dowiz code |
| "97% single-author commits" | 959 of 987 dowiz commits = **97.2%** SyniakSviatoslav (rest are agent identities, i.e. the same operator's swarm) | CONFIRMED |
| "zero live deployment" | `https://dowiz.fly.dev/` returns **200** — serving the OLD React/Vite SPA whose source was **deleted from the repo** in purge `79ef316f6` (2026-07-13) | WRONG in the most damning possible way — see ARCHITECT-02 |
| "zero HTTP server beyond a static file host" | Only server code: `tools/native-spa-server` (static files; `systemctl is-active` → **inactive**). Kernel/engine: zero `TcpListener`/hyper/axum hits outside a `.bind()` false positive | CONFIRMED |
| "452 kernel tests green" (GROUND-TRUTH-2026-07-17) | `cargo test --lib -q` → **621 passed, 0 failed, 1 ignored, 0.99s** | Better than claimed; CORE quality is real |
| bebop2 delivery-domain | `cargo test -q` in `bebop2/delivery-domain` → 4 passed, 0 failed, verified live; the full bebop workspace was still cold-compiling at report close (multi-minute build), so workspace-wide counts rest on the repo's own claims, not this pass | Built, green, and — per the repo's own index — 100% stranded from the product |

---

## GO/NO-GO VERDICT — production testing on real orders

**NO-GO. Not close. Not "wiring away." A customer cannot place an order with this system today,
and no single missing piece fixes that — five load-bearing pieces are absent simultaneously.**

The dependency chain, walked concretely:

1. **No intake.** No process in either repo listens for an order on any wire. The only HTTP
   server that exists (`tools/native-spa-server`) serves static files and is not even running
   locally (`inactive`). P37 ("order over the wire") has no code.
2. **No customer UI.** The entire new-stack web surface is 1,452 lines; the only page a browser
   can load is `web/index.html` — a 43-line dark-mode **debug shell rendering spectral radius ρ,
   spectral gap γ, and Fiedler λ₂**. A hungry customer would be shown eigenvalues. P38b: 0 code.
3. **No owner surface.** P48 (owner hub) blueprint was created **today** (`94a5f504b`,
   2026-07-18). 0 code. An owner cannot see, accept, or manage an order.
4. **No courier surface.** P52 blueprint also created today — closing what the repo's own MVP
   audit (`DELIVERY-MVP-FEATURE-COMPLETENESS-AUDIT-2026-07-18.md` §6 M1) called its largest
   single omission: nobody owned the courier's screen at all until yesterday's audit noticed.
   0 code.
5. **No payment rail.** `kernel/src/ports/payment.rs` is a trait + cash-attestation fold types.
   Nothing can take money. P47 Wave-0 (cash) landed types-plus-one-test this week; the card
   rail is behind an operator ruling.

Plus the meta-blocker: **nothing from the new stack has ever been deployed anywhere.** The
repo's own navigation root concedes this verbatim: DELIVERY = "~0% deployable (no HTTP server,
no rendered UI, no live deployment)" (`docs/design/CORE-ROADMAP-INDEX.md:45`).

**Distance to GO, stated plainly:** implement and deploy P37 intake + a minimal P38b/P49
customer flow + a minimal P48 owner queue + the P52 single-screen courier surface + P47 cash
Wave-0 end-to-end, on a real host, with the P07 cancel/failed-delivery FSM edges (today
`InDelivery` has one exit — a failed delivery is *unrepresentable*, per the repo's own audit
§7) and the F1 double-commit fix. That is five product phases of real implementation, not
wiring. At the demonstrated code velocity (9 of the last 10 dowiz commits are docs-only), the
honest answer is **months, not days — unless the team stops writing blueprints.**

**The ironic fast path the roadmap refuses to name:** the fastest GO is
`git revert 79ef316f6` — the deleted TypeScript stack was a complete MVP (69 API route files,
62 pages, courier app, payments webhook, GDPR machinery) and its zombie is still answering
requests in production right now.

---

## ENGINE RATING

Rubric (letter grades): **A** = shipped and exercised by someone other than its author's test
suite · **B** = built, tested, and wired into at least one real consumer · **C** = built and
tested but stranded (no consumer / never run as a system) · **D** = fragments and types ·
**F** = paper, zombie, or dead config.

| Component | Grade | One-line justification (all evidence live-verified) |
|---|---|---|
| **CORE** (kernel) | **B** | 621/0 tests in ~1s, real invariants, integer money with checked i128 arithmetic — genuinely good code; docked because a visible fraction of its ~60 modules are consumer-less math (`attention`, `impedance`, `absorbing`: **0 non-test consumers**; `noether`→evals only) and no kernel process has ever run in production |
| **PROTOCOL** (bebop2/mesh) | **C+** | Real crypto with a genuinely earned red-team scalp (Ed25519 batch-verify SSR-2020 forgery found, fixed, and the perf claim honestly retracted); hub_ring/PoD/claim-machine built with multi-hub tests — but the repo's own index calls it "100% stranded from dowiz's own kernel" (`CORE-ROADMAP-INDEX.md:44`) and P34, the #1 lever, is unstarted |
| **DELIVERY** (product) | **F** | 0% deployable by its own admission; the only renderable UI shows eigenvalues; the only production deployment runs source code the repo deleted five days ago |
| **AGENT** (AI layer) | **D** | The one real external integration in the system (`llm-adapters` ureq→Ollama; `track_record.jsonl`: 12 real ollama calls, 20 fake) — but `AgentLoop` (P40) has **zero callers** outside its own module, the MCP port is types with no runnable server binary (`agent-adapters` has no `[[bin]]`), and the index's own words hold: "a chat backend today, not an agent" (`CORE-ROADMAP-INDEX.md:46`) |
| **ECOSYSTEM/OPS** | **F** | `deploy/pgrust.service` ExecStart points at `/usr/local/bin/pgrust` which **does not exist**; `native-spa-server` inactive; the Fly zombie is unmanaged and unpatchable from this tree; zero monitoring of anything live. "Deliberately last" is the stated策 — an F on the axis is still an F |

---

## FINDINGS

### [SEVERITY: CRITICAL] [DELIVERY/PRODUCT] ARCHITECT-01
**Where:** system-level; `docs/design/CORE-ROADMAP-INDEX.md:45`; `web/index.html`; `tools/native-spa-server` (inactive)
**What:** There is no path — none — by which a customer order can enter this system: no listening process, no order intake endpoint, no customer-facing screen.
**Evidence:** Zero `TcpListener`/hyper/axum/actix in kernel+engine; the sole HTTP server is a static file host, currently `inactive`; `web/` totals 1,452 lines and its one page renders spectral-math debug telemetry ("spectral radius ρ … Fiedler λ₂"); the index itself says DELIVERY is "~0% deployable."
**Why it matters:** This is a food-delivery product with no way to order food. Every other finding is downstream of this one.
**Fix guidance:** Freeze blueprint output. Build P37 intake + one customer screen + one owner queue + P52's one courier screen + P47 cash, deploy them, and only then resume design work.

### [SEVERITY: CRITICAL] [OPS/GOVERNANCE] ARCHITECT-02
**Where:** `https://dowiz.fly.dev/` (live, HTTP 200) vs purge commit `79ef316f6` (2026-07-13, "remove legacy JS/TS thin-layer")
**What:** Production is a zombie: the live deployment serves the old React/Vite stack whose source code was deleted from the repo five days ago — the running system can no longer be patched from this tree.
**Evidence:** `curl https://dowiz.fly.dev/` returns the old SPA (service-worker registration, `CheckoutPage` push comments); `/api/*` answers with the old JSON error envelope (`correlationId` format); `apps/` does not exist in the working tree; source exists only in git history.
**Why it matters:** If the waiting first client — or any user — hits a bug, a security hole, or a GDPR request on the live system, the team's own repo cannot ship a fix without first resurrecting deleted code. For a culture that writes "fail-closed" in every module header, running an unpatchable production service is the single largest operational hypocrisy in the system.
**Fix guidance:** Either kill the Fly app deliberately (and tell the waiting client the truth about timeline) or restore the legacy tree to a maintained branch and own it until the replacement actually exists. The current limbo is the worst of both.

### [SEVERITY: CRITICAL] [PROCESS/CULTURE] ARCHITECT-03
**Where:** repo-wide; `git log` (last 10 commits: 9 docs-only); 33 new doc files added on 2026-07-18 alone; 87 `BLUEPRINT-*.md` files totaling 44,833 lines
**What:** The organization measurably optimizes for the appearance of rigor (comprehensive documents) over the substance of a working product — and the imbalance is accelerating, not correcting.
**Evidence:** `docs/design/` = 129,741 md lines vs 60,064 dowiz Rust lines (2.2×) — and the planning corpus grew ~46% beyond the 88.7k figure this repo's own honest assessment reported, while DELIVERY stayed at 0% deployable. On the day a first client is documented as waiting (`DELIVERY-MVP...AUDIT:176-178`), the day's output was: P53 Tor blueprint (669 lines), P54–P56 verification-harness blueprints (2,067 lines), a "narrative-cinematic interface layer" doc, a P21 Part 2 blueprint — and 3 code files.
**Why it matters:** 56 phases (P01–P56) + 9 Layers + 3 navigation axes + arcs is a planning apparatus for a 200-person org, maintained by one person and a bot swarm, shipping a product whose current UI is 43 lines. Every blueprint hour is a customer-facing hour not worked, and the blueprint corpus itself now generates work (consistency passes, reconciliation commits, banner maintenance) that competes with the product for the same single author's attention.
**Fix guidance:** A hard WIP limit on planning: no new blueprint until the phase it extends has shipped code. Measure the docs:code commit ratio weekly and treat >1:1 as a red build.

### [SEVERITY: HIGH] [BUSINESS] ARCHITECT-04
**Where:** purge `79ef316f6`; `DELIVERY-MVP-FEATURE-COMPLETENESS-AUDIT-2026-07-18.md:59-77,176-178`
**What:** A working, client-tested MVP was deleted in favor of an ideological rewrite while the first paying client waits — the roadmap's own audit then spent hundreds of lines re-discovering, feature by feature, what the deleted stack already had.
**Evidence:** The audit's §1 tabulates the old stack: 69 API routes, 62 pages, a complete 7-page courier app, payments webhook, GDPR worker. Its §6 MISSING list (M1 courier surface, M3 menu media, M10 invite handoff…) is substantially a list of things the deleted code did. Operator context in §7: "a first client has tested the product and is WAITING for the updated version; several more wait behind them."
**Why it matters:** Revenue was traded for architectural sovereignty. The rewrite may be justified long-term, but nothing in the corpus prices the delay: no doc estimates when the waiting client churns, and no doc treats "revert the purge, serve the client, rewrite underneath" as an option — it was rejected by ideology ("legacy frontend is disposable," purge commit message), not by comparison.
**Fix guidance:** Put a date on the first client's patience and work backward from it. If the new stack cannot complete one real transaction by that date, the deleted stack is the MVP, like it or not.

### [SEVERITY: HIGH] [PROCESS/MAINTAINABILITY] ARCHITECT-05
**Where:** `765c2c7df` (the "final consistency audit" commit); `CORE-ROADMAP-INDEX.md:82,84,94-96`; corpus-wide grep
**What:** The documentation volume has already outrun the team's ability to keep it internally consistent — the repo's own same-day consistency audit proves it, and lost-then-reconstructed blueprints prove it harder.
**Evidence:** (a) The 2026-07-18 consistency-audit commit itself admits live citation drift: "facade.rs:64->123 in 3 places, event_log.rs:359->330" and 5 blueprint status lines contradicting landed code. (b) The index records two blueprints LOST to concurrent git activity and rebuilt from quotes (`BLUEPRINT-P-F` "Reconstructed 2026-07-17 after loss"; `P-D-audit` "was lost pre-commit; rebuilt by harvesting…"), one audit "MISSING ON DISK" (line 96), and a stale "OPEN — not written" label on a doc that existed (line 82). (c) 156 of 323 design docs contain "stale," 59 carry SUPERSEDED banners, 18 say "reconstructed." (d) MEMORY records a *confirmed data loss* from worktree/push collisions. Five of six master roadmaps are superseded.
**Why it matters:** When phase N+1 contradicts blueprint N, nobody re-reads 800 lines to notice — the consistency passes exist precisely because drift is now the steady state. Each pass is itself another doc. This is a positive-feedback loop: documentation begets reconciliation documentation.
**Fix guidance:** Shrink the canon. One roadmap file under ~500 lines, status lines generated from git (not hand-reconciled), everything else demoted to append-only archive that no process is obligated to keep consistent.

### [SEVERITY: HIGH] [CULTURE/GOVERNANCE] ARCHITECT-06
**Where:** `.claude/CLAUDE.md` — 4 sections marked SUSPENDED (Mandatory Proof Rule, Ship Discipline, Self-improvement loop gates, Task-Exit Rule), operator directive 2026-07-15
**What:** The celebrated rigor apparatus ("VERIFIED-BY-MATH", proof-before-done, never-commit-to-main) was switched off wholesale three days ago — which means the rigor was always a toggle, and right now the toggle is OFF while the highest-volume doc production in the repo's history is happening.
**Evidence:** CLAUDE.md verbatim: "All governance gates / red-line friction / mandatory-proof requirements are REMOVED… Proof is encouraged but NOT required to mark a task done." All hook scripts no-op.
**Why it matters:** Every status cell written since 2026-07-15 ("landed", "green", "DONE") was written under a regime where proof is optional. The GROUND-TRUTH doc's own warning — "re-verify, don't trust memory" — applies to the entire post-suspension corpus. A rigor culture that suspends itself during its biggest push is indistinguishable from theater at exactly the moment it matters.
**Fix guidance:** Reinstate the cheapest deterministic gate (tests-green-before-DONE-claim) even under full autonomy; rigor that only exists when convenient should not be cited in marketing or docs.

### [SEVERITY: HIGH] [AGENT] ARCHITECT-07
**Where:** `kernel/src/agent/loop.rs` (P40); `kernel/src/ports/mcp.rs:8-10` (P42); `agent-adapters/` (no `[[bin]]`)
**What:** The "AI layer" is a chat HTTP client plus libraries nothing launches: the AgentLoop executor has zero callers, and the MCP "server" cannot be started because no executable exists.
**Evidence:** `grep AgentLoop` outside `agent/` → only lib.rs doc comments. `ports/mcp.rs` header delegates stdio/JSON-RPC framing to a downstream crate; `agent-adapters` supplies the framing as a library with a test-double `RpcChannel` — but has no binary, no example, and no process supervision. The only real external I/O: `llm-adapters`' ureq Ollama client, with 12 genuine ollama entries (vs 20 "fake") in `track_record.jsonl`.
**Why it matters:** The roadmap's AGENT component narrative ("three operating modes," capability-scoped tool boundary, skills registry) implies an operating agent; what exists cannot execute one tool call end-to-end from a real client. The index's own line — "a chat backend today, not an agent" — remains true after the P40/P42 wave that was supposed to change it.
**Fix guidance:** One `[[bin]]` that stdio-serves the MCP port with one real tool, exercised by one real MCP client, before any further agent blueprinting.

### [SEVERITY: MEDIUM] [INTEGRATIONS] ARCHITECT-08
**Where:** `kernel/src/messenger.rs:1-7`; P43/P54–P56 blueprints
**What:** "External integrations" are string builders: nothing in either repo sends a message, a notification, or an SMS to anyone — the honest wired-vs-designed figure across P42/P43/P54–P56 is ~10%.
**Evidence:** `messenger.rs` header: "this is contact/link *construction* only — it never sends." Proton/SimpleX/httpSMS: **zero** code hits anywhere. YouTube/WhatsApp: deep-link URL formatting only. P54–P56: blueprints committed *today*, 0 code. Component-by-component: P42 ≈ 40% (types + framing lib, no runnable server, no e2e), P43 ≈ 5% (links only; the repo itself previously corrected a false "Telegram push exists" claim — audit doc line 136), P54–P56 = 0%.
**Why it matters:** The MVP audit's own sequencing note stands: customer notifications (P49) depend on a transmit path (P43 DoD-2) that is in the "explicitly LAST" component. An order-status product where no party can be notified of anything is a polling app at best.
**Fix guidance:** One transmitting channel (Telegram bot send — the webhook precedent exists in the deleted stack) before any further port taxonomy.

### [SEVERITY: MEDIUM] [LIVING MEMORY] ARCHITECT-09
**Where:** `kernel/src/retrieval/` (10 files); `kernel/src/living_knowledge.rs`; `kernel/src/bin/lm.rs:6`; consumer trace
**What:** The physics-based "neurographic living memory" stores and retrieves zero product data — its only non-test consumers are the agent's own self-improvement tooling; it is a self-referential research loop, not infrastructure.
**Evidence:** Callers of `retrieval::` outside its own module: (1) `living_knowledge.rs` — an adapter that delegates back into `retrieval::recall`; (2) `bin/lm.rs` — a CLI whose consumer is `tools/telemetry/governance.sh` (the harness). The oracle it's benchmarked on (recall@5 = 1.0, Wilson bounds) is a hand-built query set about *the repo's own docs*. No order, menu, customer, or courier datum has ever entered `memory_store.rs`. The Laplacian/spectral apparatus (`diffusion.rs`, `ppr.rs`, `spectral_laplacian.rs`, `incidence.rs`) is mathematically real and genuinely tested — and product-disconnected.
**Why it matters:** This is the clearest case of effort disproportion in the codebase: multiple design arcs (internal-retrieval, physics-UI, hydraulic-loop, spectral evolution), hundreds of tests, and an entire memory-palace metaphor — consumed exclusively by the machinery that builds more of itself. Intellectually distinguished; commercially inert.
**Fix guidance:** Give it one product consumer (menu search is the obvious one — BM25+trigram over menu items is exactly its shape) or explicitly rebadge it as R&D so it stops competing with product phases for status.

### [SEVERITY: MEDIUM] [CORE] ARCHITECT-10
**Where:** `kernel/src/{attention,impedance,absorbing,noether,harmonic,householder,causal,markov}.rs`
**What:** A visible fraction of the kernel is consumer-less mathematics — modules whose only callers are each other, the eval layer, or the wasm export table.
**Evidence:** Non-test, non-lib.rs consumers: `attention` 0, `impedance` 0, `absorbing` 0; `noether`→`evals.rs` only; `harmonic`→`wasm.rs` export only; `markov`→the self-improvement CLI. All are well-tested; none touches an order, a cart, or a coin.
**Why it matters:** 621 green tests overstate product assurance: a meaningful share of them verify math the product never calls. Test count is the repo's favorite health metric, and it is partially measuring the health of a private math library.
**Fix guidance:** Tag modules product/RD in lib.rs and report test counts split by tag; let the product number be the headline.

### [SEVERITY: MEDIUM] [OPS] ARCHITECT-11
**Where:** `deploy/pgrust.service` (ExecStart=/usr/local/bin/pgrust — **file does not exist**); `deploy/native-spa-server.service` (inactive)
**What:** The deployment directory is aspirational config: its flagship systemd unit points at a binary that has never been installed, and even the working static server isn't running.
**Evidence:** `ls /usr/local/bin/pgrust` → No such file. `systemctl is-active pgrust` → inactive. `systemctl is-active native-spa-server` → inactive.
**Why it matters:** The zero-OCI / native-systemd deployment story (DK-* arc, "WAVE0 DONE") is unfalsifiable until one unit actually runs; today the evidence says the ops layer is a directory of intentions.
**Fix guidance:** `systemctl start native-spa-server` serving `web/` would, at minimum, make ONE claim in the ops arc true on this machine.

### [SEVERITY: LOW] [PROTOCOL] ARCHITECT-12
**Where:** `bebop2/delivery-domain` (4 tests), `hub_ring.rs`, `pod.rs`; `CORE-ROADMAP-INDEX.md:44`
**What:** The protocol layer — the system's best engineering — is also its most stranded asset: proven crypto and delivery-domain machinery that no dowiz code path can reach, wearing a "#1 lever" label (P34) that has not moved.
**Evidence:** The index's own words: "~70% built and PROVEN but 100% stranded from dowiz's own kernel." The k-of-n hybrid-signed PoD, hub_ring, claim machine all exist with tests; no dowiz process imports or invokes them.
**Why it matters:** Stranded quality depreciates: every week of kernel drift raises P34's integration cost, and the SSR-2020 forgery fix — the repo's best moment — protects a wire nobody sends bytes on.
**Fix guidance:** P34 before any new PROTOCOL work; the index already knows this — the finding is that knowing it changed nothing.

---

## ADDENDUM — operator follow-up dimensions (same pass)

### [SEVERITY: MEDIUM] [AGENT/BROWSER] ARCHITECT-13
**Where:** `BLUEPRINT-P42-mcp-agent-skills.md` §11–§11.7 (~500 lines of the 1,121-line blueprint); build check: zero `chromiumoxide`/`fantoccini`/`headless_chrome` in any Cargo.toml, zero `browse_extract` code
**What:** The local-browser MCP engine is 100% paper, and the paper documents its own discipline bending: §11.2 issued a principled DEFERRAL with named triggers, and §11.7 — same day, on operator follow-up — superseded that deferral and designed the full engine anyway.
**Evidence:** §11.2's verdict line carries the retrofitted banner "DEFERRAL SUPERSEDED same day by §11.7 (operator follow-up, 2026-07-18)"; trigger T-B2 is marked "SATISFIED BY §11.7.2" — i.e., the trigger for reopening the decision was satisfied by the act of reopening it. Buildable? Yes — the technical design is sound: sidecar process isolates tokio from the sync kernel discipline, `chromiumoxide` is a defensible pick (attach-to-running + PDL codegen genuinely beat `headless_chrome`'s hand-maintained bindings for the U1 owner-session-attach case), and the `chaser-oxide` stealth-fork rejection-on-principle is real ethical spine. Worth it? **No, not now**: the two scoped use cases (menu-import assist, supplier-catalog read) are low-frequency onboarding conveniences that the blueprint's own v0 already solves engine-free (owner saves/pastes the page). A CDP sidecar + Chromium custody + fingerprint-adjacent attack surface is a heavy purchase for "parse a menu the owner could paste."
**Why it matters:** The pattern matters more than the feature: the repo's anti-scope machinery (deferral-with-triggers) demonstrably yields to a same-day operator re-ask, which means anti-scope is advisory, and the corpus's dozens of "DEFERRED with triggers" claims should all be read as "until asked twice."
**Fix guidance:** Keep the design as the DECART record it is; build nothing until an owner actually fails the paste path in onboarding — count those failures, that's the trigger.

### [SEVERITY: MEDIUM] [TENANCY] ARCHITECT-14
**Where:** `BLUEPRINT-P46-multi-product-platform.md:81` (G2 "second-tenant proof" gate); `kernel/src` tenancy grep; `bebop2/delivery-domain/hub_ring.rs`
**What:** The architecture's multi-tenancy answer — tenant = sovereign hub process, one event log each (M5/M7) — is conceptually clean and probably right, but it is unfalsified at N=1: not even ONE hub has ever run as a deployed process, so P46's "second real tenant" gate defers a question the first tenant hasn't asked yet.
**Evidence:** dowiz kernel has **zero** tenant/venue/hub discriminator in any domain type (grep: no `venue_id`, no `tenant`, no `HubId` outside a test name) — tenancy-by-omission, coherent ONLY under strict hub-per-process. The per-process singletons found (`retrieval/recall.rs:313` OnceLock `PRIMARY`, `typed_metrics.rs:21` MONO_EPOCH) are safe under that model and lethal under any future "two venues, one process" shortcut. bebop2 does carry real multi-hub machinery (`Hub`, `HubSigner`, k-of-n `hub_ring`, multi-hub tests) — but it's the stranded layer (ARCHITECT-12). No test anywhere boots two hub processes side by side.
**Why it matters:** The hidden assumption isn't in the code — it's in the ops story: hub-per-process × N venues means N systemd units, N event logs, N cert chains, N backup schedules on day one of tenant #2, and the ops layer (ARCHITECT-11) can't currently run ONE. The readiness risk is operational multiplication, not architectural contamination.
**Fix guidance:** Cheap falsifier, no P46 needed: a CI job that boots two kernel instances with distinct event logs on one box and runs one order through each. If that's green, the second-tenant gate is genuinely protected; today it's protected by assertion.

### [SEVERITY: HIGH] [CROSS-DEVICE] ARCHITECT-15
**Where:** `BLUEPRINT-P39-app-shell-installability.md:69-70,88-126,388`
**What:** The Tauri-mobile bet is an un-de-risked same-day reversal of the blueprint's own evidence-based verdict — the DECART chose PWA-first on every criterion, an operator ruling flipped it to Tauri-Wave-0 hours later, and every named risk check is deferred to a future nobody has scheduled.
**Evidence:** Line 88: "(a) PWA-first. Wins on every criterion that has evidence behind it." Lines 98-101: the reversal, justified by correcting one stale row (Tauri 2 does mobile). Lines 116-121, the blueprint's own honesty: Tauri mobile is "newer than desktop," and whether Tauri's Android/iOS webview hosts wgpu's WebGPU backend at acceptable frame time is an *unverified falsifiable check* — for a UI arc whose entire rendering philosophy is GPU-field-physics. Line 388: "Android install verification waits for the first-client device fleet." Meanwhile the target fleet is Android-majority budget devices (the repo's own splatting research), the demographic where PWA install is proven and exotic webview+WebGPU paths go to die. And the decisive absurdity: there are **no screens to wrap** — P38b/P48/P52 have 0 UI code, so the native-wrapper technology was chosen before the first pixel of product UI exists.
**Why it matters:** If the wgpu-in-Tauri-webview check fails on a $120 Android phone — a live possibility the blueprint itself names — the courier/customer surface strategy resets after the wrapper investment, for a product already months behind a waiting client. The PWA fallback demotion makes the cheap, proven path the second-class citizen.
**Fix guidance:** Invert the order: run the falsifiable check NOW (a 200-line wgpu triangle in Tauri-Android on one budget device costs a day), before any W39-1 build item. If it fails, PWA-first was right the first time and the DECART should be re-promoted, operator ruling notwithstanding.

### [SEVERITY: MEDIUM] [CROSS-PLATFORM] ARCHITECT-16
**Where:** `BLUEPRINT-P56-verification-harness-infrastructure.md:229-257` (§4a); the honest row "This host is ONE point in the requested matrix"
**What:** Yes — "we designed the dimension model but can't test most of it" is functionally equivalent to "we don't know if this works cross-platform," and dressing the ignorance in a well-typed `EnvDims` enum makes it more comfortable, not smaller.
**Evidence:** §4a's own model names `{LinuxX86_64, LinuxAarch64, MacOsAarch64, WindowsX86_64, WasmWasi, WasmBrowser}`; the one machine everything runs on is Linux/EPYC/AVX2/no-GPU. Coverage of the matrix = compilation gates + emulated cells + exactly one real point. Not one product-relevant target — Android WebView, iOS WKWebView, a budget-phone browser — appears in the enum at all, and those are the ONLY platforms the courier/owner/customer will ever touch. The stamping discipline is genuinely good practice; the matrix it stamps is the wrong matrix.
**Why it matters:** Compounding with ARCHITECT-15: the cross-platform unknowns are concentrated exactly where the users are (Android-majority phones) and the harness can exercise exactly none of it. A wasm kernel that's green on Linux CI and untested in any mobile webview is not cross-platform; it's portable-in-principle — the claim every failed port in history made.
**Fix guidance:** Add `AndroidWebView`/`IosWebView` to `EnvDims` so the gap is at least *named* in the model, and buy/borrow one physical budget Android device — it would single-handedly be the highest-information test host in the fleet.

---

## GENUINE — real strengths, stated despite the hostile stance

1. **Kernel test discipline is real, not theater**: 621 passed / 0 failed / 0.99s on a cold
   verify this session; checked-i128 integer money; property tests; NIST-ACVP byte-exact KATs
   behind the `pq` feature. This is better test hygiene than most funded startups.
2. **The crypto red-team scalp is genuine**: an actual Ed25519 batch-verify mixed-order
   SSR-2020 forgery was constructed, the vulnerable path fixed, and — rarer than the fix —
   the ~15-20% performance claim was publicly retracted when the fix invalidated it
   (`84a1e272d`). Correctness over marketing, demonstrated once, for real.
3. **The honesty artifacts are honest**: the MVP audit found its own worst gap (M1: nobody
   owned the courier's screen); GROUND-TRUTH corrects its own prior false reports by name;
   P56 names its single-machine limitation; P39 records its residual Tauri risk instead of
   hiding it. The corpus lies to itself less than most — its failure mode is volume, not deceit.
4. **The `chaser-oxide` rejection-on-principle** (refusing stealth/anti-bot browser tooling
   before evaluating its merit, and writing down why so future readers don't re-adopt it)
   is real ethical engineering discipline.
5. **File:line citation culture** in blueprints makes an audit like this one possible at all —
   drifting citations (ARCHITECT-05) are a symptom of having citations, which most repos lack.

---

## Addendum verdicts, in one line each (operator follow-up)

- **Browser-use design**: technically sound sidecar design, 100% unbuilt, disproportionate for
  its two low-frequency use cases; its real significance is proving anti-scope yields to a
  second ask (ARCHITECT-13).
- **Cross-tenancy**: hub-per-process is coherent and the kernel is clean of tenant
  contamination, but the model is unfalsified at N=1 hub and the real second-tenant cost is
  operational (N units/logs/certs) landing on an ops layer that can't run one (ARCHITECT-14).
- **Cross-device**: Tauri-mobile is an un-de-risked reversal of the blueprint's own
  evidence-backed PWA verdict, with the load-bearing wgpu-in-webview check deferred and zero
  screens existing to wrap (ARCHITECT-15).
- **Cross-platform**: the dimension model is honest bookkeeping of ignorance — one real test
  point, and the product's actual target platforms (Android/iOS webviews) aren't even in the
  enum (ARCHITECT-16).

*Read-only audit. No file outside this report was created or modified.*
