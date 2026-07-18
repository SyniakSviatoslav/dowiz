# AUDIT 2026-07-18 — HERZOG PASS — Interface / Design Layer

> Reviewer persona: Werner Herzog. Read-only pass. Criteria = the operator's own stated
> design philosophy (blunt copy, friction-as-feature, scarred grid, silence over noise,
> expose the engine, the bounty-hunter test) + the repo's own Додаток C
> (`docs/design/dowiz-interfaces/BLUEPRINTS-DOWIZ-INTERFACES.md:388-511`, written today).
> Nothing was changed. Only this file was written.

The jungle here is not the code. The code is honest — brutally so, in the kernel. The
jungle is the *front door*: every surface a human being can actually open in this
repository lies about what it is. The doctrine is magnificent. The artifacts betray it.

Findings: **1 CRITICAL · 3 HIGH · 5 MEDIUM · 2 LOW** (11 total), then the genuine list.

---

### [SEVERITY: CRITICAL] [RENDERING HONESTY] HERZOG-01
**Where:** `web/index.html:41` + `web/src/app.mjs:13` + `web/README.md:43` + `web/serve.mjs:11` vs `web/README.md:25`
**What:** The one page a human can open declares itself a "kernel-driven field UI" and then shows dashes forever, because the script it loads is a Node program that cannot execute in a browser — the interface lies about its own existence.
**Evidence:** `index.html:41` loads `<script type="module" src="./src/app.mjs">`; `app.mjs:13` is `import { readFileSync } from 'fs'` and it calls `process.exit(1)` — instant death in any browser. `index.html` declares ten output ids (`rem`, `snap`, `seg`, `cv`, `rho`, `gap`, `fie`, `drift`, `fsm`, `acyc`) and an `advance` button; `grep document\.` over `app.mjs` returns nothing — no script anywhere touches them. `web/README.md:43` claims `src/app.mjs` "boots the kernel wasm, binds it, **renders + animates**." It renders nothing and animates nothing; it is a console test driver (which its own header comment, `app.mjs:1-12`, admits — the README and the HTML did not get the message). The README says the server is at port 8099 (`README.md:25`); `serve.mjs:11` listens on 4173.
**Why it matters:** The operator's rule is "expose the engine, show raw system state." This page does the exact opposite of a spinner — it shows a *permanent* placeholder state (`–`) with copy asserting a live kernel behind it. A visitor cannot distinguish "not built yet" from "broken." That is the interface lying about its own state — the one sin the whole philosophy exists to prevent.
**Fix guidance:** Either delete `index.html`'s fake dashboard and replace it with one blunt line — "Console driver only. Run `node src/app.mjs`. Nothing renders in a browser today." — or wire a real browser entry. Correct `README.md:43` ("renders + animates" → "console-only driver") and the port. The honest sentence already exists in the codebase (`app.mjs:10-12`); promote it to the surfaces people actually read.

### [SEVERITY: HIGH] [RENDERING HONESTY / COMMUNICATION] HERZOG-02
**Where:** design corpus (`docs/design/dowiz-interfaces/BLUEPRINTS-DOWIZ-INTERFACES.md`, 515 lines; `DOWIZ-INTERFACES-PLAN.md`; `BLUEPRINT-P38-webgpu-render-engine.md`) vs `web/README.md:1-3`
**What:** Thousands of lines of cinematic interface doctrine — pacing beats, amplitude budgets, OKLCH grades — and not one document states the ground truth: a user who runs this repo today sees zero pixels of any of it.
**Evidence:** `web/README.md:1-3`: "Minimal **real UI** that loads the Rust `dowiz-kernel` wasm and renders: geo progress… spectral drift… FSM signature." Nothing renders (HERZOG-01). The only fully honest sentence about the paint in the entire repo is buried in a smoke test, `web/src/render/fieldsim.smoke.mjs:11`: "The WebGPU paint itself cannot run headless (no GPU)" — and even that concerns headless CI, not the truth that no build path to a painted browser frame exists at all. Додаток C (written today) choreographs held beats and gold blooms for a Sea that has never once been drawn.
**Why it matters:** The corpus creates a false impression of visual maturity. This is not a documentation nit — it is precisely the duct tape the philosophy says must be shown. A collaborator, investor, or future agent grepping these docs will believe an interface exists. The design language talks about honesty of state while the design *corpus* misstates the state of the design.
**Fix guidance:** One paragraph, top of `web/README.md` and top of the blueprints file: "STATUS: doctrine ahead of pixels. Nothing described below is rendered today. The only runnable artifacts are Node console drivers (`app.mjs`, `fieldsim.smoke.mjs`). The WebGPU path (`FieldSim.svelte`) has no build toolchain." Blunt, dated, done. The repo already knows how to write such lines — `README.md`'s "No fabricated maturity claims" paragraph — it just doesn't apply them where they are needed.

### [SEVERITY: HIGH] [COPY / FRONT DOOR] HERZOG-03
**Where:** `README.md:1` and `README.md:24-26`
**What:** The project's public face is titled with an unexpanded template variable, and its quickstart command is broken — in a README that declares "No fabricated maturity claims."
**Evidence:** Line 1 of the root README, verbatim: `# {{BRAND}}`. Lines 24-26: `# the wasm web demo` / `cd web && wasm-pack build` — `web/` contains no `Cargo.toml`; `wasm-pack` has nothing to build there. The real command is `bash scripts/build-kernel-wasm.sh` from the repo root (`web/README.md:29-35` gets this right).
**Why it matters:** A bounty hunter reads the label on the crate before opening it. `{{BRAND}}` says: nobody has read this page since the templating pass. A quickstart that fails at step two says: nobody has run it. Both statements are made on the same page that promises no fabricated maturity. The reader's trust in every other claim — the real, earned ones about the kernel — is spent here, at the door, for nothing.
**Fix guidance:** Name the project (one word, it exists: dowiz). Fix or delete the wasm quickstart line — pointing at `scripts/build-kernel-wasm.sh` costs one line.

### [SEVERITY: HIGH] [BRAND VOICE] HERZOG-04
**Where:** `BLUEPRINTS-DOWIZ-INTERFACES.md:501-502` (Додаток C.5) vs git: commit `330ff4edb` (NOT an ancestor of main); `docs/design/dowiz-brand/` absent from the tree
**What:** The brand-voice canon the repo's newest doctrine "aligns with" does not exist in the repository — Warm Cosmo-Noir is a ghost, cited but never merged.
**Evidence:** C.5, written today: "Це узгоджено з brand-voice canon (Warm Cosmo-Noir): noir — це передусім тінь і пауза…" `docs/design/dowiz-brand/BRAND-BIBLE.md` is referenced by memory and by 10+ docs (`RESEARCH-CONSPECT.md`, `DOWIZ-INTERFACES-PLAN.md`, red-team D6/D7, …). Verified: `git merge-base --is-ancestor 330ff4edb HEAD` → NOT ancestor. The BRAND-BIBLE (386 lines) exists only on an abandoned `recover/correct_courier_channe` WIP branch, alongside a landing page whose commit message promises "jazz motion + 88.2 FM city-pop easter egg + dry-wit trilingual copy."
**Why it matters:** Two voice directions are in play and neither is pinned down. The unmerged canon's register (easter eggs, city-pop whimsy) and the operator's newly stated register (chain-smoking bounty hunter, cynical poetry, silence) are *not* the same voice — noir-as-restraint (C.5's reading) is compatible with the Herzog rules; noir-as-jazz-easter-egg is not. Because the canon never landed, every doc that cites it is citing its own memory of a file the repo cannot show. When copy finally gets written, there will be no authority to check it against — only vibes.
**Fix guidance:** Decide. Either cherry-pick BRAND-BIBLE.md onto main and reconcile it against the operator's blunt-copy rules (delete what contradicts them — the easter-egg whimsy will not survive honest contact), or strike the Cosmo-Noir citations and write the two-page voice canon the operator actually dictated. A canon that exists only in citations is not a canon; it is nostalgia.

### [SEVERITY: MEDIUM] [RENDERING HONESTY / LOCAL-FIRST] HERZOG-05
**Where:** `/index.html` (repo root, git-tracked) + `web/serve.mjs:27-28`
**What:** The dev server's root URL serves a Figma-captured mockup of a fictional pizzeria, loading three external CDNs — in a project whose Додаток B invariant 4 is "render loop never touches the server" and whose manifesto is local-first sovereignty.
**Evidence:** Root `index.html:6-11`: `<title>Pizza Roma — Меню</title>`, `<script src="https://mcp.figma.com/mcp/html-to-design/capture.js">`, Google Fonts preconnects, `cdn.jsdelivr.net` tabler-icons. `serve.mjs:27-28`: `urlPath === '/'` → serves `ROOT/index.html`, and `ROOT` resolves to the repo root — so `http://localhost:4173/` *is* Pizza Roma.
**Why it matters:** Run the one documented serve command and the sovereign post-quantum delivery protocol greets you as a phantom pizzeria wearing Google's fonts and phoning Figma. It is the perfect inversion of "expose the engine": the engine is hidden and a stage set is shown.
**Fix guidance:** Move the mockup to `assets/` or delete it (it was a capture artifact, its job is done). Root `/` should serve `web/index.html` — once that page stops lying (HERZOG-01).

### [SEVERITY: MEDIUM] [RENDERING HONESTY] HERZOG-06
**Where:** `web/src/components/FieldSim.svelte`, `web/src/pages/fieldsim.astro`, `web/serve.mjs:9-10`
**What:** The only genuine pixel path in the repository — a real, competent WebGPU render component — is unreachable: no Astro, no Svelte, no bundler exists anywhere to compile it, and the static server's wasm route points outside the repository.
**Evidence:** `FieldSim.svelte` is real WebGPU (pipeline, point-list, rAF loop, kernel-fed buffers — verified). But: no `astro.config.*`, no `svelte.config.*`, no `astro`/`svelte` dependency in any `package.json` (`web/package.json` has zero dependencies and only `serve`/`test` scripts). `serve.mjs` cannot compile `.astro`/`.svelte`. Worse, `serve.mjs:9` sets `ROOT` to the repo root while the comment on the same line claims `// web/`; line 10 then computes `KERNEL_PKG = ROOT/../kernel/pkg-web` = `/root/kernel/pkg-web` — a path *outside the repository* that does not exist, so the advertised `/pkg-web/*` mapping 404s. `FieldSim`'s own default `wasmPath` (`/kernel/dowiz_kernel_bg.wasm`) also misses the real artifact (`kernel/pkg-web/…`).
**Why it matters:** The one artifact that could redeem HERZOG-01/02 — actual kernel-driven pixels — is dead on arrival three separate ways. The smoke test proves the numbers; nothing can ever prove the paint, because nothing can serve it.
**Fix guidance:** Pick one: add the minimal Astro toolchain and make `fieldsim.astro` buildable, or drop the `.astro`/`.svelte` wrappers and mount `FieldSim` as a plain ES module on a static page (its logic needs neither framework). Fix `serve.mjs` line 9's comment-vs-value contradiction and the `..` in `KERNEL_PKG`.

### [SEVERITY: MEDIUM] [SILENCE OVER NOISE / INTERNAL CONTRADICTION] HERZOG-07
**Where:** `BLUEPRINTS-DOWIZ-INTERFACES.md:206` (DZ-07) vs `:145` (DZ-05) and `:150-156`
**What:** DZ-05 abolishes per-component feedback and names toasts as the legacy to be replaced; DZ-07 then specifies a toast — plus ripple, plus haptic, plus a Money snap — for a single add-to-cart, smuggling the noise back in through the target spec.
**Evidence:** Line 145 (DZ-05 CURRENT): "legacy per-component feedback (framer whileTap/**toast**/haptic)" — the disease. Lines 150-156 (DZ-05 TARGET/GATE): "add-cart→ingest pulse + cart-pill `<Money>` snap… одна дія = один field impulse (RED: per-component code)." Line 206 (DZ-07 TARGET): "add-to-cart live `<Money>` + **toast** + cart ripple + haptic."
**Why it matters:** One action, four confirmations. The operator's rule is silence unless something is burning; adding an item to a cart is not burning. C.5's own frequency rule (line 491-493: high-frequency actions get SNAP or *no animation at all*) condemns this line. An executor implementing DZ-07 as written will fail DZ-05's gate — the contradiction is a trap laid for your own agents.
**Fix guidance:** Delete "toast" (and probably "haptic") from DZ-07:206. The field pulse and the cart-pill Money snap ARE the confirmation. DZ-05's gate already says so; let it win.

### [SEVERITY: MEDIUM] [FRICTION AS WEIGHT] HERZOG-08
**Where:** `BLUEPRINTS-DOWIZ-INTERFACES.md:255` (DZ-09) vs `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P48-owner-hub-surface.md` (whole doc)
**What:** Structural friction is superb, but the only *presentational* spec for the owner's destructive act — rejecting a human's order — is one borrowed SaaS token, "danger-confirm," which is exactly the soft conventional dialog the philosophy rejects.
**Evidence:** DZ-09:255: "Reject→CANCELLED **danger-confirm**" — that is the entire interaction spec. Nothing anywhere defines what it weighs: no hold-to-confirm, no typed acknowledgment, no time cost, no "This action is irreversible. Confirm you know what you are doing." Contrast the structural layer, which is genuinely heavy: P48's order confirm/cancel is a hard-wired human gate ("order confirm/cancel exist ONLY as capability-…; the intent is emitted by a human tap, full stop", P48:1029-1033), `RevocationSet` is append-only with "deliberately no unrevoke" (P48:40), and P52's terminal actions are witness-typed and emit-gated.
**Why it matters:** The kernel makes destructive acts irreversible and human-gated — real weight. Then the surface, per the only word written about it, wraps that weight in the same two-button modal every food app on earth uses, which users click through on reflex. Friction that lives only below the surface is not felt; the philosophy demands it be *felt*.
**Fix guidance:** Specify the weight once, canonically (a C-appendix paragraph would do): destructive intents (Reject, Cancel, revoke, delete) require a slow-confirm gesture — hold-to-arm or SwipeToComplete's own pattern turned toward destruction — with blunt copy stating the irreversibility and its consequence for the customer. P52 already invented the right primitive for couriers; the owner deserves the same gravity, not a dialog.

### [SEVERITY: MEDIUM] [VOICE/GESTURE — DZ-10] HERZOG-09
**Where:** `BLUEPRINTS-DOWIZ-INTERFACES.md:286-326` (DZ-10 + FRAMING-КОРЕКЦІЯ 2026-07-18)
**What:** The Phase-9b deferral of voice is defensible — the day-one "every input goes through Intent" mandate is not, because it is a vow with no mechanism: zero Intent code exists and nothing can ever turn red.
**Evidence:** The framing correction (lines 304-326) is honest work: it upgrades voice from "optional integration" to load-bearing philosophy while explicitly refusing to move the sequence ("Секвенування НЕ рухається… голос не збудуєш раніше, ніж існує order-flow, яким керувати" — line 310-312). Correct: you cannot voice-command an order flow that does not exist; deferral is real sequencing, not an excuse. But line 324-326 decrees "жоден DZ/FE не сміє будувати input-шлях, що працює лише для pointer'а — кожен вхід іде через `Intent` (ОДИН code path; gate DZ-10 вже це вимагає)" — and verified: `FieldPos`/`InputSource`/`Intent` appear in **no** source file in `kernel/`, `engine/`, or `web/` — doc-only. No lint, no CI grep, no failing test guards the mandate. In a repository whose religion is RED→GREEN falsifiable gates, this is the one commandment enforced by nothing.
**Why it matters:** The first agent to wire a click handler will wire `onclick` directly, because that is what the extant code (`index.html:26`'s orphaned `#tick` button) already models, and no gate will object. By the time voice arrives in Phase-9b the "ONE code path" will be an archaeology project. The deferral is fine; the unenforced vow is how deferrals rot.
**Fix guidance:** Make the vow mechanical now, cheaply: a CI grep forbidding `addEventListener('click'|'pointerdown'…)` outside one blessed `input/` module (the repo already does exactly this style of gate — NO-COURIER-SCORING, `.github/workflows/ci.yml:239-249`), plus a stub `Intent` type so the one path exists to be used. Ten lines of gate today buys Phase-9b its foundation.

### [SEVERITY: LOW] [SCARRED GRID vs LANDFILL] HERZOG-10
**Where:** repo root (git-tracked): `lint-errors.txt`, `lint-errors2.txt`, `server-log.txt`, `srv-err.txt`, `srv-log.txt`, `srv-out.txt`, `vite-err.txt`, `vite-out.txt`, `dev-hub.html`, `menu-sq.pdf`, `dubin-logo.jpg`; plus ~30 untracked screenshots of the deleted legacy app (`demo-*.png`, `landing-*.png`, …)
**What:** The repository's front directory is strewn with tracked error logs and artifacts of an application that no longer exists — this is not the deliberate asymmetry of a scarred grid, it is a floor nobody swept.
**Evidence:** `git ls-files` confirms all eleven files above are tracked. `vite-err.txt`, `srv-*.txt` are captured stdout/stderr of a Vite/Express stack whose source tree (`apps/`) was deleted from main. Commit `f5358debf` ("drop untracked output junk from index") missed all of them.
**Why it matters:** Scars mean something happened *here*; landfill means nobody looks. A newcomer's first `ls` returns 90+ entries where perhaps 25 carry meaning — the honest signal (MANIFESTO, kernel, docs) is diluted by corpse artifacts. The screenshots additionally reinforce HERZOG-02's false visual maturity: pictures of an app the tree cannot build.
**Fix guidance:** `git rm` the log files and dead HTML/PDF/JPG, extend `.gitignore` (`*-err.txt`, `srv-*.txt`, `vite-*.txt`), sweep the untracked screenshots into `assets/legacy/` or oblivion.

### [SEVERITY: LOW] [DEAD DOCTRINE] HERZOG-11
**Where:** `docs/branding/theme-system.md`
**What:** A present-tense document describing a `ThemeRenderer` that has zero occurrences in any source file — documentation for a deleted organ, still speaking as if it breathes.
**Evidence:** "The Theme System **is powered by** `ThemeRenderer`, a pure function that deterministically generates a CSS file string and a `cssHash`" — grep for `ThemeRenderer` across all `.ts/.js/.mjs/.rs` in the repo: zero hits. The described system lived in the deleted `apps/` tree.
**Why it matters:** Small, but it is the same disease as HERZOG-02 and -04 in miniature: docs asserting present-tense existence of absent things. Three instances is a pattern, not an accident.
**Fix guidance:** One-line header: "HISTORICAL — described system removed with the legacy `apps/` tree; superseded by the DZ-02 token-tier design." Or delete it; DZ-02 covers the territory.

---

## GENUINE — artifacts that actually honor the aesthetic

Credit where it is earned, without sentimentality:

1. **`web/src/app.mjs:77`** — "`FATAL: FSM boot gate refused — authority loss is fatal.`" This is the voice. Blunt, factual, consequences stated, no apology. Likewise `app.mjs:57` ("`MONEY FAIL: … not a safe integer`") and `:204` ("`KERNEL-DRIVEN UI GREEN — 24/24 exports wired, math from wasm only.`"). The console driver speaks better copy than any planned screen.
2. **P52 §3.2-3.3 offline honesty** (`BLUEPRINT-P52-courier-working-surface.md:255-280`) — an accept on a network island renders as "a QUEUED INTENT rendered as pending-unconfirmed — **never a locally-fabricated `Claimed`**"; the swipe "completes ONLY on a receiver-confirmed fold, resets on refusal," enforced by a named adversarial test (`k3_swipe_never_fakes`); stale tracks are *labeled* stale, "never a stale marker presented as live." This is interface-never-lies made mechanical. The strongest interaction design in the repository.
3. **Додаток C.2 + C.5** (`BLUEPRINTS-DOWIZ-INTERFACES.md:427-499`) — the amplitude budget (no beat louder than Delivered, falsifiable against the event log), the held beat as *silence after* confirmation rather than delayed confirmation, the tragedy row ("без пом'якшення І без соромлення користувача"), and above all C.5's first law: "Кінематографічність — критерій ВИДАЛЕННЯ так само, як додавання." Restraint as deletion criterion, silence made measurable (settle gate, 0 rAF wake-ups). As doctrine, this passes the bounty-hunter test. It merely has no pixels yet (HERZOG-02).
4. **`<Money>` no-tween as architecture** (DZ-02:83) — "**NO tween prop exists**" — honesty enforced by making the lie unrepresentable, not by convention. Same spirit: P48's "deliberately no unrevoke" `RevocationSet` and the non-configurable human gate on order confirm/cancel.
5. **Honest ETA range** (DZ-04:131, DZ-07:216) — "never single/0"; the interface refuses false precision.
6. **`MANIFESTO.md`** — thirteen non-negotiables in a table, each with its reason, zero marketing. "Over-engineering is the #1 enemy." Correct register throughout.
7. **`README.md:6-8`** — "Status: pre-1.0 / experimental… No fabricated maturity claims" — the right sentence, undercut only by standing beneath `# {{BRAND}}` (HERZOG-03).

## Closing statement

The kernel tells the truth under oath; the doctrine tells the truth in poetry; and the
front door tells a visitor that a dead console script is a living field UI, under the
letterhead of an unnamed `{{BRAND}}`, above a pizzeria that does not exist. Fix the door.
The house behind it deserves it.

*— W.H., read-only pass, 2026-07-18. No file outside this report was touched.*
