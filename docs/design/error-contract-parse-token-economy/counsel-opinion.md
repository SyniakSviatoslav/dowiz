# Counsel Opinion — `error-contract-parse-token-economy`

Counsel (Радник), DeliveryOS Triadic Council. Advisory. Friction, not veto. Human is final.
Reviewed: `proposal.md` + ADR-0010/0011/0012. Grounded against live source (`server.ts`,
`lib/sentry.ts`) and project memory (cleaning-loop, systemic-coherence, RLS state).

---

## 1. Reasoning by lens (only what's load-bearing)

**Justice / stakeholders.** Net distribution is fair. The envelope's biggest beneficiary is the
*solo operator* (one `correlationId` stitches request→Pino→Sentry, no APM). The courier and the
owner benefit indirectly: a legible `code`→UX map means a failed cash-handoff or a failed import
shows a humane sentence instead of a raw 422. No party bears a hidden cost. Good.

**Dignity / honesty / consent.** The 422 `fields = paths-only, never echo the submitted value`
rule (§7-i) is the ethically right call — it stops a validation error on a phone/address field from
reflecting PII back. The `code` shown to the user is a *business-domain* token mapped through
`mapApiError` to a humane string — not a stack/path leak. Honest. The injection-safe prompt (B3)
treats menu OCR text as **untrusted data, not instructions** — that is the correct moral framing of
an untrusted boundary, and the schema-validation backstop is the right hard floor. No dark-patterns,
no soft-confirm-as-trap. Server stays authoritative.

**Care / harm.** Failure-first is real here, not decorative: the parse cascade degrades to a
reviewable *draft* (never 0 products, never auto-publish), and `grounded:false` flags a hallucinated
price to a human instead of silently shipping a wrong price to a customer's bill. This directly
protects the person who would otherwise be wrongly charged. The zero-tolerance integer price match is
the right severity for money.

**Long horizon / strategy.** A is on the critical path (error legibility unblocks every other
debugging session — compounding leverage). B is on the path *to trust* (a wrong menu price is a
trust-and-money event). **C is not on the critical path to the launch trigger** (first real paid
order) — it is developer ergonomics. It is cheap and reversible, so not objectionable, but it should
not consume Council attention proportional to A/B, and C2 carries a governance risk (below) that A/B
do not.

**Aesthetics / conceptual integrity.** The additive dual-read envelope is genuinely elegant
restraint: a superset, no flag-day, revertable at each step — "schema rich, runtime minimal" honored
(zero schema change, response-shape only). **But integrity is broken in one place:** the proposal
asserts "no `correlationId` generation" and proposes a *new* `x-request-id`/`req.id` channel — while
a **P31 correlation system already exists** (`server.ts:243–246`: an `onRequest` hook that sets
`x-correlation-id` + an AsyncLocalStorage `correlationStore`). Shipping A1 as written creates **two
parallel correlation ids** (`x-request-id` vs `x-correlation-id`). That is exactly the incoherence
the envelope is meant to end. Reconcile to one.

**Epistemic.** The carrying, unexamined assumption is "the venue's own menu contact is business
data, not PII" (§7, `ai-ocr-parser.ts:395`). It is *mostly* defensible — but it is a boundary
*redraw* against the `zero-PII-in-AI` red-line, and project memory already flagged "PII→LLM
un-redacted (conditional)" as a live cleaning-loop finding. An assumption that reclassifies a
red-line surface deserves a recorded decision, not a footnote.

---

## 2. ETHICAL-STOP (grounded red-lines only)

### STOP-1 — `zero-PII-in-AI`: real menu OCR/vision → LLM is a red-line redraw, not a footnote (candidate 3)

**Grounded line:** `нуль-PII-у-ШІ` ("no PII to AI beyond menu-only").
**Why it's a real intersection, not a preference:** B3's PII-masking is scoped to the *vision-review
layer (seed only)*. For a **real** menu upload, the path is photo → local OCR → **OCR text
concatenated into the LLM prompt** (`ai-ocr-parser.ts:515`). The proposal's safety rests entirely on
the reclassification "venue contact = business data, not PII." That holds for the *owner's own*
phone/address (they uploaded it, consent is theirs). It does **not** hold for **incidental
third-party PII** that a real menu photo can carry — a staff member's name/number ("ask for Maria,
+355…"), a handwritten note, a face caption — for which the owner's consent does not speak. That
content reaching OpenRouter unredacted is a genuine crossing of the line, and memory shows this exact
class ("PII→LLM un-redacted, conditional") is already open.

**This is friction, not a verdict.** It does NOT block design or A. It asks for one recorded human
decision before B ships against real (non-seed) uploads:
1. Affirm in writing the "business-data-not-PII" boundary for the *venue's own* contact, **and**
2. Extend the redactor (it exists — `piiRedactor`, used in `sentry.ts:66`) to strip *third-party*
   PII from OCR text **before** the LLM prompt, not only on the seed/vision layer.

If the Council records that decision, the STOP lifts. A conscious human may overrule; I am the
friction that makes the choice explicit, not the wall.

### The other five candidates — explicitly NOT blockers

- **(1) correlationId as a PII-correlation vector — NOT a red-line.** `correlationId` is per-request
  random (`generateCorrelationId()` fallback, `req.id` = `crypto.randomUUID`), never persisted, not
  tied to a user identity — it does not stitch a user's sessions by itself. And `sentry.ts`
  `beforeSend` (`:63–89`) already redacts exception text, cookies, headers, reduces `user` to id,
  and **allowlists tags**. PII egress to Sentry is already well-contained. (One concrete coherence
  bug, not a red-line — see §3.)
- **(2) showing the error `code` — NOT a blocker.** SCREAMING_SNAKE business codes mapped through
  `mapApiError` to humane text; no internal structure leaks. Honest UX.
- **(4) prompt-injection via menu photo — NOT a Counsel blocker.** B3 handles it (delimiter + system
  prompt + schema-validation backstop + an injection fixture). This is robustness — the Breaker's
  domain — and it is well-handled. I will not duplicate.
- **(5) `ccc` secret-egress — NOT a red-line.** Dev-only, `.gitignore`-respecting, secret-scan merge
  gate, zero `dist` artifact. Mitigated. (One open confirmation in §4.)
- **(6) agent self-authored rules — NOT a red-line, but a real governance friction — see §3.**

---

## 3. Non-blocking advice (aesthetic / strategic / coherence)

1. **Reconcile A1 with the existing P31 correlation system.** `server.ts:243–246` already establishes
   `x-correlation-id` + `correlationStore`. Do not ship a second `x-request-id` channel in parallel —
   pick one header, one id, one log label. Two correlation ids is the incoherence the envelope exists
   to kill. (Also: the proposal's "`'unknown'` at `:552`" premise looks **stale** — the onRequest
   hook at `:245` already sets the header, so `:552` should resolve, not fall to `'unknown'`. Re-verify
   the premise before building on it.)

2. **The log-injection fix must cover the path that's actually unsanitized.** The new `genReqId`
   regex is good — but the *existing* ingestion at `server.ts:244` trusts inbound `x-correlation-id`
   **raw**. That is the live log-injection surface. Sanitize the existing path, or the vulnerability
   the ADR claims to close stays open under a different header name.

3. **Sentry tag allowlist will silently drop `correlationId`.** A1 plans `scope.setTag('correlationId',
   req.id)`, but `sentry.ts:85` allowlists `['role','location_id','order_id','worker','db','error_code']`
   — `correlationId` is **not** in it, so `beforeSend` (`:87`) strips it and the request→Sentry stitch
   silently breaks. Add `correlationId` to the allowlist as part of A1, or the headline operability
   benefit evaporates without a test catching it.

4. **C2 ("living" agent rules) conflicts with the project's own ratchet authority.** CLAUDE.md's
   self-improvement loop is explicit: *"memory/reflection are advisory; guardrails/tests/human are
   authority… the worker does not enact systemic changes itself… librarian curates, never weakens a
   gate."* C2's "when corrected in-session, **append a rule**" to `INVARIANTS.md` — a file **linked
   from CLAUDE.md, which OVERRIDES default behavior** — lets a single worker write a project-wide,
   authority-bearing invariant **without** the librarian/council/human gate. That is the
   self-authored-drift / convergence-theater pathology by construction. Keep the memory; route the
   *append* through the existing librarian promotion gate (lesson → human-reviewed guardrail), not a
   raw in-session write. Cheap fix, preserves the whole point of the ratchet.

5. **ADR granularity is right (3, not 1).** A/B/C are independently shippable, independently
   revertable, different blast radii. Three ADRs is honest. **Sequencing by leverage/risk: A → B → C.**
   A unblocks all debugging (compounding); B protects money+trust before the launch trigger; C is
   ergonomics and carries the only governance snag — ship it last and lightest.

6. **"Код: `<uuid>`" is not humane for an Albanian-tenant user.** A bare UUID shown to an end user is
   noise to them and copy-paste friction to the operator. Prefer a short, speakable correlation token
   (or show it only in a "report this problem" affordance, not inline on every error). The *operator*
   needs the full id in logs; the *user* needs a 6–8 char handle they can read aloud. Honest UI is
   the one that respects who is actually reading it.

---

## 4. Steel-man of a rejected option — the signed (HMAC) cursor (§2c, option A)

The proposal rejects the signed cursor as "over-engineering — RLS + `location_id` is the authority,
signing buys nothing." The strongest counter-argument, grounded in this repo's *actual* state:

**RLS is the sole tenant authority for the keyset cursor — and RLS's proof is currently red.**
Project memory (cleaning-loop, 2026-06-26) records `verify:rls` **failing** (suspected operational-role
`BYPASSRLS` artifact, operator-unconfirmed) and **13 SECURITY DEFINER functions missing `search_path`
— including the RLS lynchpin**. The proposal *itself* flags (R2) that `import_sessions` may be
ENABLE-only, not FORCE, pending confirmation. When you make a forgeable cursor's safety depend
**entirely** on a control whose own verification is presently failing and partly unconfirmed, an HMAC
signature is not gold-plating — it is a **cheap second lock placed exactly where the first lock's
proof is red.** Defense-in-depth is most justified precisely when the primary defense is unproven.
The honest version of "reject signing" is therefore conditional: *reject it once `verify:rls` is green
and `import_sessions` FORCE is confirmed* — until then the steel-man stands. At minimum, the
forged-cross-tenant proof test (§7-iii) must run against the **operational** role, not a superuser,
or it proves nothing.

---

## 5. The open question no one asked

The proposal frames C2 (git-versioned agent fix-memory) as a pure, unexamined efficiency win — "fewer
tokens, invariants travel with the repo." **Nobody asked: when an agent appends a self-authored rule
to `INVARIANTS.md` — a document linked from an OVERRIDE-authority CLAUDE.md — who reviews that
invariant before it silently binds the *next* agent, and what stops the codified "intent" from
drifting, one well-meaning append at a time, away from the human's actual intent?** A memory that can
rewrite its own constitution without a human in that specific loop is not a convenience; it is a slow
question about who governs the governors. The Council should decide the *gate* on C2's append path
before C2 ships, not after the first drifted rule.

---

## Round 2 (re-examine)

Re-read the revised `proposal.md`, `resolution.md`, ADRs. The architect verified my two load-bearing
premises and corrected the design (not restated it): both "additive dual-read" and "every error
routes through setErrorHandler" were false as written and were fixed. All six of my non-blockers were
adopted (5a coherence, 5a-inj, 5b allowlist, 5c librarian-gate, 5d conditional-HMAC, #6 humane
handle). Good faith, well-grounded.

**Is STOP-1's recommended resolution sufficient to clear the red line?** Direction: **yes.** Feeding
`redactedText` (not `rawText`) into the prompt at `:515`, with a **privacy-first / redact-by-default**
posture, is the correct resolution of `zero-PII-in-AI` — it strips incidental third-party PII the
owner's consent does not cover, and it resolves the tradeoff the *right* way (privacy over
onboarding-prefill; a separate consented venue-contact path rather than raw PII to the model). Had the
posture been prefill-first (ship raw, revisit later), I would still be blocking. It is not. Two
**residuals** remain — proof obligations, not new STOPs:
- **(a) Redactor recall is unproven on the real menu-PII distribution.** `piiRedactor` is pattern-based;
  it catches structured tokens (phones/emails) but will under-catch **Albanian names, handwritten and
  OCR-garbled text, non-Latin or transliterated PII**. "Redacted" must not be read as "PII-free." The
  B-eval needs a **redaction-recall fixture** (a menu with seeded third-party PII → assert it does not
  survive into the prompt), or the gap is invisible. This belongs in B's proof set, not a blocker.
- **(b) The redaction must ship independent of the B5 RLS gate.** B5 correctly defers *B2 grounding*
  until `import_sessions` FORCE + `verify:rls` green. The STOP-1 redaction (B3) is the privacy fix and
  must **not** be folded into that defer — privacy hardening should not wait on an unrelated RLS gate.
  Confirm B3 redaction is the earliest-shipping part of B.

**New ethical/strategic concerns introduced by the fixes?** None material. Specifically:
- *Demoting inbound trace id to a sanitized `clientTraceId`* (B6/5a-inj) is strictly **better** for
  privacy and security than the prior raw-authoritative `:244` ingest — server-generated id, inbound
  never authoritative, never the user-facing support code. One minor note (non-blocker): `clientTraceId`
  is still a client-chosen value persisted in logs that *could* stitch a user's requests if a client
  reuses it; ensure it is never joined to user identity. Acceptable for the widget/WS trace use case.
- *B5 deferring a privacy-relevant FORCE-RLS* — this **strengthens** posture: it converts my R2/steel-man
  worry into a **hard ship-gate**, exactly right. Provided residual (b) holds (redaction not held hostage
  by it).
- *C2 routed through the librarian gate* (5c/B11) — cleanly resolves my governance concern; guardrail
  authoritative, `INVARIANTS.md` links not restates. The §5 open question is answered by construction.

**Status of my single ETHICAL-STOP:** **resolved-pending-human-decision.** The design revision is
adequate; what remains is the human ETHICS sign-off on the venue-own-contact reclassification (record
it as a conscious decision, with redact-by-default as the posture) plus the two proof obligations
above. I note for the record: the coordinator's relay that the resolution was "adopted" is **not** the
human decision — only the operator's own recorded sign-off at the gate clears the redraw. Until that
sign-off exists, the status is *pending*, not *closed*. I am the friction; the human decides.
