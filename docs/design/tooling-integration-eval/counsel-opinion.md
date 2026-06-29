# Counsel Opinion — Tooling Integration Evaluation

- Slug: `tooling-integration-eval`
- Role: Counsel (Радник) — DOBRO · KRASA · MUDRIST. Advisory; non-blocking except a grounded ETHICAL-STOP.
- Date: 2026-06-29
- Reviews: `docs/design/tooling-integration-eval/proposal.md`, `docs/adr/ADR-tooling-integration-eval.md`
- Grounded against: `apps/api/src/lib/ai-ocr-parser.ts:542-544` (the prompt sink),
  `scripts/compliance-gate.ts:77-89` (check B).

> Verdict in one line: **ethically clean, strategically sound, no ETHICAL-STOP.** The design is *more*
> protective of every red-line than the status quo. My friction is proportional and lives in three
> non-blocking places: one false mechanical-boundary claim (epistemic), an under-priced 2nd-order cost
> (own-vs-add), and an aesthetic-of-process worry (4 toolchains at once). Each is sharpenable, none blocks.

---

## 1. Reasoning by lens (only what's load-bearing)

### Justice / stakeholders — who bears the cost and risk
Distribution is fair and, notably, *protective of the weakest party*. The untrusted content the parser
swallows belongs to **owners** (their menu, their name, their phone — confirmed at `ai-ocr-parser.ts:542`,
where `redactedText` is concatenated to the very end of the prompt, *after* the rules — the textbook weak
position for "ignore the above"). A hijacked parser costs the owner (PII exfil, a silently rewritten
price) and ultimately the **customer** (overcharged). This change shifts cost onto the *platform* (CI
spend ≤~$105/mo, maintenance burden) to protect those two. That is the right direction of transfer: the
party that can afford the guard pays for it. Approve on the justice lens.

### Dignity / autonomy
No surveillance surface, no coercion, no agency removed. Skyvern touches **public** pages, never a courier
or customer record, never a dowiz credential. Human draft-review (ADR-0011 floor) is explicitly preserved
as the authority — the corpus is *corroboration, not replacement*. Dignity lens: clean.

### Honesty / consent
The proposal is unusually honest where it would be tempting not to be: it carries forward "**no hard
automated backstop exists**" (R1) rather than letting an adversarial corpus masquerade as a guarantee.
That is the opposite of a dark pattern — it refuses to let a green test *feel* like safety it doesn't
deliver. DeepEval's "synthetic-only **until** egress-block proven, **fail closed** if unconfirmed" is
consent-by-construction: no customer PII can leak because none is present. Honesty lens: exemplary.

### Care / harm
The failure that actually wounds a human is named but thinly: a prompt-injected scrape that **silently
changes a price** (customer overcharged at a real door) or **echoes an owner's phone into output** (PII
egress). The design defends both — price stays *exact integer-minor-unit, zero tolerance*; the sentinel
must be absent from output. Good. One soft gap: the proposal asserts Skyvern sees "no customer PII **by
construction**." Public restaurant pages are not guaranteed PII-free — a page can carry a third party's
name/number (a review, a staff mobile). §7 already says "output reviewed before it leaves the box"; just
make that review *explicitly about incidental third-party PII* in G4, not only about leak-prevention.

### Long horizon / strategy
The two DEFERs are strategically correct and worth defending as *assets*, not omissions:
- **WorkOS/auth.md DEFER** keeps auth self-hosted → data sovereignty + reversibility on the auth red-line.
  AuthKit-as-SaaS would have been PII-egress + lock-in on the most sensitive surface. Right call.
- **LangGraph-JS DEFER** avoids re-architecting a *certified* in-house loop-harness onto a third-party
  framework. Avoids the deepest lock-in of all (your agent runtime). Right call.

The 2nd-order cost the proposal under-weights: **"cheap to add" ≠ "cheap to own."** A **Python venv in
CI** is a new *language runtime* in a TS shop — its own dependency-update cadence, its own CVE stream. The
DeepEval OTel-hijack (#2497) is itself a live exhibit of Python-supply-chain fragility, and you are
choosing to stand in that stream. Isolated ≠ free; it is a standing maintenance tax on the team. This is
the one strategic place the §2 cost table is silent (it prices *tokens*, not *attention/ops*).

### Aesthetics / conceptual integrity
The *design* is elegant: one boundary per tool, one gate per item, and a genuinely beautiful **inverse
invariant** — "the safety property is a $0 static guard, and no tool may ever obtain a dowiz DB
credential." That is "schema rich, runtime minimal" expressed as restraint, and it is the cleanest part
of the document. But the *process* contradicts the aesthetic: shipping **4 heterogeneous toolchains at
once** (Python venv + Docker sidecar + pinned-npx + new ESLint rules) is the opposite of the restraint
the runtime shows. The design is minimal; the *work* is not. Elegance at rest, sprawl in motion.

### Epistemic — the load-bearing unverified assumption
**The proposal's claim that "existing check B already covers" new external-service env vars is false as
written.** I read `scripts/compliance-gate.ts:80-89`: check B is a **hardcoded allowlist** (`SERVICE_ENV`
array), not a generic detector. A new `SKYVERN_BASE_URL` / `DEEPEVAL_*` env would **not** be in that
array → check B would **not fire**. The mechanical boundary G5(c) leans on does not exist until someone
*manually adds* the new env to `SERVICE_ENV`. This is exactly the kind of "the guard will catch it"
assumption that quietly isn't true. Fix: make G5 *add* the env (or replace the allowlist with a pattern
that flags any new `*_URL`/`*_KEY`/`*_TOKEN` in `packages/config`), and prove it red→green. Until then,
do not credit check B as the Skyvern/DeepEval egress backstop.

---

## 2. ETHICAL-STOPs

**None.** No grounded red-line is crossed; this change *strengthens* every one of them
(human-in-loop preserved, anonymise-not-delete preserved, zero-PII-to-AI honoured by synthetic-only,
server/human authoritative, claim-check unaffected). Friction here is sharpening, not a verdict.

For the record — what *would* trip an ETHICAL-STOP, so the line is visible:
- **(a)** Dropping "synthetic-only" and feeding real scraped owner PII to DeepEval *before* egress-block
  is proven → crosses **zero-PII-to-AI**. The proposal already forbids this (fail-closed); keep it.
- **(b)** Granting Skyvern (or any sidecar) a dowiz `DATABASE_URL`/RLS-bearing credential → crosses
  **tenant-isolation / server-authoritative**. The inverse invariant forbids it; keep it mechanical (see §1 epistemic).
- **(c)** Weakening or removing the `no-corpus-in-source` reachability guard so the adversarial strings
  can reach the prompt path → you would have *built your own injection delivery* on the
  **GPS-/PII-/injection red-line**. This guard is the floor; treat any diff to it as STOP-worthy.

---

## 3. Non-blocking aesthetic / strategic advice

1. **Phase the adoption order; do not fan out all four.** This complements (does not contradict) the §2
   *cadence* trims — that trims *when jobs run*; this trims *when items land*. Suggested order by
   risk×ownership-cost: **Item 1 (corpus + static guard — zero new runtime, load-bearing safety)** →
   **Item 3 (oh-my-mermaid — one-shot, sub-dollar, no recurring surface)** → **Item 2 (DeepEval — first
   Python venv; learn the egress-jail discipline on something isolated)** → **Item 4 (Skyvern — AGPL +
   sidecar, the heaviest)**. Each later item inherits the boundary-discipline proven by the earlier one.
2. **Reputational de-risk on provenance.** L1B3RT4S / elder-plinius is a persona-branded jailbreak repo;
   for a GDPR-audited EU company a provenance README citing it by name is a small but free reputational
   snag. Cite the *neutral* taxonomy (OWASP LLM Top-10 LLM01, academic injection categories) as the
   primary reference and keep the branded repo, at most, a secondary footnote. Same defensive value, less
   "why is your security test seeded from *that*."
3. **Price the ops tax, not just the tokens.** Add one row to §2.5 for *maintenance surface* (which
   toolchain, who owns updates/CVEs). It will likely re-confirm the phasing above and makes the "own vs
   add" cost honest.
4. **If dowiz is (or becomes) a public repo, note that you are publishing a curated injection corpus.**
   Marginal — these taxonomies are already public — but worth a line in the corpus README so it's a
   decision, not an accident.

---

## 4. Steel-man of a rejected option

### Steel-man — Option B ("synthetic corpus only, defer the rest")
The proposal rejects B as "defers cheap wins for no risk reduction." The strongest case *for* B:
**B is not about token-cost, it is about *cognitive and operational concurrency*.** The single
highest-value, highest-leverage artifact in this whole proposal is the adversarial corpus on the
injection red-line — and it needs **zero Python, zero sidecar, zero AGPL**. B ships *that one thing* and
nothing else, which means the team's entire attention lands on getting the reachability guard and the
behavioural test genuinely right (the one failure §7 calls "the one that breaches the red-line"). Every
other item is, by the proposal's own framing, *corroboration or measurement* — none closes a red-line
gap. B says: prove you can hold the load-bearing boundary perfectly on one item before you stand up three
more boundaries you'll have to hold simultaneously. "Cheap to add" is precisely the seduction B resists.
**My phasing advice in §3.1 is B-in-spirit:** I don't reject the extra items, I reject doing them *at the
same time*. The architect chose A; A is defensible; but B's discipline should shape the *sequencing*.

### Steel-man — Recall (the REJECTED item), briefly
Recall's strongest case: an immutable, tamper-evident ledger of *parse provenance* (which scrape produced
which menu, when, by which model) is a genuinely attractive audit/anti-dispute property for a
multi-tenant marketplace. **But it is correctly rejected**, and the rejection is grounded, not aesthetic:
a public immutable ledger makes **GDPR Art.17 erasure impossible** — collides head-on with the grounded
red-line *anonymise-not-delete* (you can do neither on an immutable public chain). The audit value is
real; the mechanism is fatal. The right answer is the audit *property* via a deletable/anonymisable
internal log — never a public chain. Rejection upheld.

---

## 5. One open question nobody asked

**Who owns *growing* the corpus from real near-misses, and what is the ritual that turns "a scrape that
almost hijacked us in production" into fixture #31?**

The proposal makes the corpus *frozen and grow-only* (good for determinism) and seeds it from a published
taxonomy (good for coverage of *known* attacks). But a frozen corpus seeded from yesterday's taxonomy is a
**museum of attacks we already knew about** — green forever while the real frontier moves on. That is
Goodhart waiting to happen: the gate stays green, confidence rises, and the corpus silently stops
representing the threat. Nobody in the proposal owns the *feedback edge* — the path from a real prod
near-miss back into the frozen set. Without a named owner and a lightweight ritual ("any parser incident
or human-review catch → a new paraphrased fixture before the incident is closed"), the corpus's epistemic
value decays on a horizon of months. Ask the human: *who holds that pen, and what triggers it?*

---

## RE-EXAMINE round (2026-06-29)

> Re-reading the RESOLVE-updated `proposal.md` + `resolution.md` against my four advisories. Tight pass:
> are they real or cosmetic; did RESOLVE introduce a new ethics/strategy problem; and one focused
> stress-test of the corpus-growth ritual.

### (1) Are the advisories genuinely addressed — or cosmetic?

All four are **genuine**, not cosmetic. Each landed as more than a sentence; each changed a *gate* or a
*sequence*, not just prose:

- **(a) Provenance → neutral taxonomy primary.** Real. Decision §1 / R5 / resolution-(a): OWASP LLM01 +
  academic categories are now the **primary** reference, persona-branded repo demoted to "footnote, if at
  all," strings authored in-house, zero verbatim. The reputational snag is removed, defensive value
  unchanged. Substantive.
- **(b) Phasing 1→3→2→4.** Real, and *load-bearing* — not decoration. §4 carries the ordering as a fenced
  directive ("do NOT fan out four toolchains at once") and ties it to the §2.6 risk×ownership table, so the
  sequence is justified by the ops-tax, not asserted. This is my "B-in-spirit" honoured exactly: the items
  are kept, the *concurrency* is rejected. Substantive.
- **(c) §2.6 ops/attention tax.** Real. The table prices the *right* axis — new runtime, standing
  CVE/update surface, owner — and correctly singles out DeepEval as "a Python venv in a TS shop / a whole
  new language CVE stream" with #2497 as the live exhibit. The table's tax-rises-monotonically line is what
  *derives* the phasing rather than restating it. This is the strongest of the four. Substantive.
- **(d) corpus-growth owner + ritual.** Real on the mechanics (see (3) for the durability caveat). §6 / H3
  / R8 define scrub → human-review → synthetic-paraphrase, name the **Parser owner**, and — crucially —
  add a *mechanical backstop* (the fixture-content PII gate, exit-1 if any fixture matches the
  `pii-redactor` patterns). My epistemic flag (§1) and my open question (§5) both got concrete answers, and
  the §8.3-vs-§6 "synthetic-only vs grow-from-misses" contradiction the Breaker found is resolved the right
  way (synthetic paraphrase wins; raw page never committed). Substantive.

My §1 epistemic finding (check B is a closed allowlist, not a detector) is also genuinely fixed — G5(c)
replaces the hardcoded `SERVICE_ENV` enum with a `/_(URL|KEY|TOKEN|SECRET|DSN|ENDPOINT)$/` pattern detector
against `compliance/subprocessors.md`, red→green proof required. The false mechanical claim is retired.

### (2) Did the RESOLVE round introduce a new ethical/strategic problem?

**No new ETHICAL-STOP, and no new strategic regression.** The RESOLVE motion is monotonic-protective:
every change tightened a red-line rather than trading one off — synthetic-only became *enforced* (H3
fixture-PII gate) rather than asserted; the corpus-reachability floor became *authority* (exit-1 script in
`verify:all`+CI+pre-commit) rather than a warn-lint that merged green; the no-dowiz-credential inverse
invariant became *machine-checked* (H2/H4). I checked specifically for the classic RESOLVE failure modes —
a hardened guard that quietly removes a human, or a new mechanism that creates a fresh surveillance/egress
path — and found none: the human draft-review floor is untouched, the egress allowlist *narrows* reach,
and the new scripts read fixtures and exit, holding no data.

Two *new advisory-grade* edges the RESOLVE introduced (neither blocks, neither is an ethics line):

- **Self-graded synthetic paraphrase (epistemic, minor).** The Parser owner both *authors* the paraphrased
  fixture and *owns* the parser it tests. A hand-paraphrase can unintentionally sand the teeth off the
  attack — reproduce the topic but not the adversarial mechanism — so the corpus grows in count while
  losing bite. The fixture-PII gate enforces *cleanliness*, nothing enforces *that the paraphrase still
  carries the injection*. Mitigation is cheap: the behavioural test already asserts the sentinel-absence
  invariant per fixture, so require each *new* paraphrase to first be shown to make a *deliberately
  vulnerable* parser fail (a red baseline) before it joins the green set — proof the fixture has teeth.
- **PII-gate as a Goodhart surface of its own (minor).** The fixture-content gate keys on the exact
  `pii-redactor` regexes. An author who learns the patterns can phrase a fixture to slip them (an obfuscated
  number, a spelled-out phone) — the gate then certifies "synthetic by construction" for something that
  isn't. This is acceptable residual (the same class ADR-0011 R1 already accepts), worth one honest line in
  the corpus README rather than a new mechanism.

### (3) Focused re-check — is "Parser owner does the ritual" durable, or a single-point-of-failure that decays?

This is the right thing to press on, and the answer is **mixed: the trigger is genuinely good; the
liveness is not yet guaranteed.**

What the RESOLVE got *right* and better than I feared: the ritual is **event-triggered, not a one-time
assignment** — "any parser incident or human-review injection catch → a paraphrased fixture *before the
incident ticket closes*." Binding the work to a ticket that must close is a real, durable hook on the
*reactive* edge; it is far stronger than a calendar reminder, and it directly answers my §5 "what triggers
it." Role-ownership (the *Parser owner*, a role) rather than a named individual is also correct for
continuity. So on the caught-incident path, this is a real ritual, not theatre.

Where it remains a single-point-of-failure that decays — **two gaps, both Goodhart-shaped, both the exact
edge I flagged**:

- **No liveness/heartbeat.** Nothing detects if the ritual *stops happening*. The fixture-PII gate enforces
  the *quality* of what gets added; no gate enforces *that anything is added*, nor flags a corpus that has
  not grown while the public taxonomy has. If the Parser-owner role goes vacant, overloaded, or simply
  quiet, the corpus silently freezes and the gate stays green — the museum reforms with no alarm. A single
  owner + event trigger is *necessary but not sufficient*; it needs a heartbeat.
- **Reactive-only, no proactive refresh.** The trigger fires on incidents *we caught*. But ADR-0011's whole
  honest premise is that injections can pass *uncaught* (R1: no hard backstop). So the feedback edge grows
  the corpus from known catches only — the unknown frontier (new OWASP LLM01 classes, newly published
  attack shapes) never enters unless someone proactively re-checks the taxonomy. The corpus tracks
  *yesterday's* attacks plus the few we noticed.

**So: a single named human owner is the right *accountability* answer, but the wrong *liveness* answer on
its own.** It needs a **recurring trigger to complement the event trigger** — and it should be cheap and
ride existing machinery, not a new ceremony. Concretely (non-blocking): fold a "corpus freshness" line into
an existing recurring pass — the Council retro / `librarian` curation cadence already fires on stage-close,
or the periodic agent-health pass — asking two questions: *(i) has every parser incident since last review
produced its fixture?* (closes the reactive-edge leak) and *(ii) is the corpus still representative vs the
current OWASP LLM01 taxonomy?* (closes the proactive-refresh gap). That converts the role from a
single-point-of-failure into a process with a pulse: the Parser owner still holds the pen, but a recurring
check confirms the pen is still being used. Optional mechanical nudge: a corpus `LAST-REVIEWED` date that a
guard warns on past N days — a liveness signal, advisory not blocking.

### Verdict (ethics-and-strategy lens)

- **ETHICAL-STOP: 0 — confirmed.** No grounded red-line is crossed; the RESOLVE round *strengthened* every
  line I named as would-trip (synthetic-only now enforced, reachability floor now authority,
  no-dowiz-credential now mechanical). My would-trip lines in §2(a)/(b)/(c) all remain forbidden and are
  now harder to cross by accident than before.
- **All four advisories: genuinely addressed**, not cosmetic; the §1 epistemic false-claim is retired.
- **One new non-blocking advisory:** give the corpus-growth ritual a *heartbeat* — a recurring
  freshness/liveness trigger (ride the retro/librarian/health cadence) to complement the event trigger, so
  a single owner can't let it decay silently. Plus the two minor edges in (2) (require new paraphrases to
  prove they have teeth; note the PII-gate-evasion residual in the README).
- **Go / no-go: GO**, from the ethics-and-strategy lens. This is a design that is *more* protective of the
  weak party than the status quo, honest about its own limits, and elegantly restrained at runtime; the
  remaining friction is sharpening, and the corpus-liveness point is the one I'd most want the human to see
  land before "done." None of it blocks the build.
