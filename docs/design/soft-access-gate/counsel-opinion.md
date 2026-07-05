# Counsel Opinion — Soft Access Gate (public waitlist CTA)

> Status: ADVISORY (non-blocking unless an ETHICAL-STOP fires) · Counsel (Triad)
> Date: 2026-06-20 · Re: `docs/design/soft-access-gate/proposal.md` + `docs/adr/ADR-soft-access-gate.md`
> Human decides. Aesthetics/strategy here are friction, not vetoes. ETHICAL-STOPs below
> are friction too — they pause the council and demand a recorded human decision; they do
> not block a conscious human forever.

Verified against source: `auth.ts:138` mints `role: 'owner'` on first Google login with no
`access_requests`/allowlist check (Telegram path identical, `auth.ts:213`). So the
precondition the architect flagged is real and load-bearing — owner onboarding is **open
self-serve today**. This Opinion is read in that light.

---

## 1. Reasoning by lens (only what's load-bearing)

### Privacy / dignity (consequence + duty)
The design is genuinely PII-restrained, and that restraint is the moral spine of the doc:
hashed-IP-not-raw, claim-check (email never in queue or job-failure logs), anti-enumeration,
identical response to everyone, secrets in env. This is the *honest-UI-is-the-beautiful-UI*
principle made structural — fewer surfaces where a real person's email can leak. I endorse it.
The dignity question is not in the storage; it is in **what we tell the person we're storing it
for** (see §2, STOP-1) and **how long we keep it without a stated bound** (§2, STOP-2).

### Honesty / consent — the load-bearing tension
The whole feature is a *promise of scarcity*: "leave your email, we'll be in touch about
access." But access is **not scarce** — anyone can log in via Google/Telegram and become an
owner this minute. The architect names this precisely in §9 and refuses to *silently* claim the
gate gates access. That refusal is the difference between a defensible v1 and a dark pattern.
The risk does not live in the proposal's prose — it lives in the **microcopy that ships on the
button and the success screen**, which this doc defers to §10 without fixing the words. That is
exactly where a false-scarcity message would be born. STOP-1 grounds there.

### Aesthetics / experience (non-blocking)
"Raised my hand" is a good moment to craft — but only if the narrative is true. A "you're on
the waitlist, position #N, we'll review your request" theatre over an open door is
*seductive-elegant, not true-elegant*: it photographs well and rots on contact with a user who
discovers the side door. Conceptual integrity here means the copy matches reality: an
expression of interest, acknowledged warmly, with no implied queue we don't run. The PII
minimization is real elegance (restraint as a design value, "schema rich, runtime minimal");
the waitlist *narrative* is where elegance could turn into theatre.

### Strategy / long horizon (non-blocking)
Second-order effect: if launch messaging leans on exclusivity ("request access," "limited
early access") while the door is open, the first journalist or curious user who logs in
directly will surface the inconsistency publicly — a small, avoidable trust dent at exactly the
trigger moment (first real paid order). Reversible and cheap now; embarrassing later. The
capture itself is sound and low-regret: an email list of interested operators is an asset
regardless of whether gating ever ships. No lock-in. The one thing we may regret in a year is a
PII store that accumulated for 12 months with no retention bound and no erasure path (§2,
STOP-2) — that is a *liability* that compounds silently, the opposite of an asset.

---

## 2. ETHICAL-STOPs (grounded red lines only — 0..N)

Two. Both are friction requiring a recorded human decision, not vetoes.

### STOP-1 — Microcopy must not sell scarcity the product doesn't have (red line: soft-confirm-not-a-trap / server-authoritative truth / honest UI)
Grounded line: *dark-patterns forbidden; UI must tell the truth; consent must be informed.*
Collecting an email under language that implies **gated/exclusive/reviewed access** — while
`auth.ts:138` hands owner access to anyone who logs in — is a misrepresentation of purpose. It
is not enough that the architect *knows* the gate doesn't gate; the **person submitting** must
not be led to believe they are queuing for something scarce.
Requirement to clear the STOP: the shipped CTA + success copy (and the GDPR microcopy) must
frame this as *"register interest / keep me posted,"* **not** *"request access," "join the
waitlist," "position #N," "we'll review your application."* If the team genuinely wants a real
waitlist narrative, then the owner-onboarding-invite-gating defer-flag must ship *first or
together*, so the scarcity is true. Recorded human decision needed: **which of the two — honest
"interest" copy now, or true gating now — before any "access"/"waitlist" wording goes live.**

### STOP-2 — A PII store collected NOW with no retention bound and erasure deferred to Stage-30 (red line: anonymize-don't-delete / PII consent / lawful basis)
Grounded line: *PII gets a lawful basis, a purpose limitation, and an erasure path; we
anonymize, not hoard.* The table has **no TTL and no erasure grant** (the migration
deliberately withholds DELETE; §8 defers erasure to "Anonymizer Stage 30"). Deferring the
*admin UI* is fine. Deferring **erasure for a PII store that begins collecting on day one** is
the part that crosses the line: a person who submits today has a GDPR right-to-erasure *today*,
not when Stage-30 lands. "Schema rich, runtime minimal" is a virtue for features; it is not a
license to collect identifiable data with no exit.
Requirement to clear the STOP — the *minimum* that makes day-one collection honest (a recorded
human pick among these, not all):
  (a) a documented **lawful basis** (consent vs legitimate interest) and a **stated retention
      window** in the privacy notice the form links to; AND
  (b) a **manual erasure path that exists on day one** — even just an operator runbook +
      a one-line `DELETE`/null grant — so an erasure request can be honored before Stage-30,
      OR an explicit, recorded human acceptance that we will manually satisfy erasure out of
      band and the privacy notice says so.
This is friction, not a wall: the human may record "legitimate-interest, 12-month retention,
manual erasure on request" and proceed. What is *not* acceptable is shipping PII collection
with the erasure question silently parked.

**Not STOPs (deliberately):** RLS pattern A2, claim-check, anti-enumeration, hashed IP,
best-effort email, route naming — all sound or pure engineering taste. `ip_hash` + `user_agent`
are a *minimization* advisory below, not a red line.

---

## 3. Non-blocking advisories (aesthetic / strategic)

- **Copy is the whole ethics surface here.** Land on "register interest"/"keep me posted" +
  a warm, honest acknowledgement ("Thanks — we've got your email and we'll reach out"). Avoid
  any number, queue, position, or "approved/reviewed" language. This single choice resolves
  most of STOP-1 and the strategic-inconsistency risk at once.
- **Minimize `ip_hash` + `user_agent` to what abuse-defense actually uses.** `ip_hash` plausibly
  earns its place for rate-limit/abuse forensics; `user_agent` is a fingerprinting-adjacent
  field with no stated use in the design. Data-minimization (and GDPR) favor dropping
  `user_agent` unless a concrete need is named. Cheap to cut now, awkward to justify later.
- **Decouple the success-moment craft from any scarcity framing.** A beautiful "hand raised"
  state is welcome — make it about *being heard*, not about *being selected*.

---

## 4. Steel-man of a rejected option

**"No table at all — just fire the operator an email (B1-adjacent / minimal capture)."**
The proposal rightly rejects synchronous in-handler send (B1) on latency/PII-hot-path grounds.
But the strongest *minimal* variant is rarely stated: **don't persist PII at all in v1 — send
the operator the email and keep nothing server-side.** Its genuine merits: it is the *cleanest
possible answer to both STOPs* — no retention question because there's no store, no
erasure-path gap, no PII-at-rest liability, no RLS surface to prove, zero migration. For a
pre-launch landing page whose admission is manual and out-of-band anyway, the operator's inbox
*is* the queue. It is the most honest match to "we'll be in touch," and the most ponytail
(the best store is the one never built). Why the proposal's choice still wins: an inbox is not
deduplicated, not queryable for `status='new'`, loses the data if a best-effort email fails
(the table survives a Resend outage; an inbox-only design does not), and the architect's
claim-check design makes the PII surface genuinely small. The steel-man's real value is that it
**raises the bar**: if we *are* going to persist PII, STOP-2's retention+erasure answer is the
price of choosing the table over the inbox. Persisting is defensible; persisting *without an
erasure answer* is strictly worse than the inbox on every ethical axis.

---

## 5. Open question nobody asked

**If the door is already open, what is the email list actually for — and would we be comfortable
saying that out loud to the person typing it?**
The honest purposes are plausibly "warm a launch announcement list" and "seed future real
gating." Both are fine — but they are *different promises* than "request access," and the person
deserves the true one. The unasked question underneath: **are we collecting because we'll use it,
or because capture feels like progress?** If it's the latter, the inbox-only steel-man (§4) is
not just lighter — it's more honest. The perspective absent from the doc is the **submitter who
later logs in via the side door and realizes the "waitlist" was never a gate.** Design the copy
so that person feels informed, not tricked.

---

## R2 Re-examine (post-STOP-ETHICS human decisions, 2026-06-20)

> Re-read: revised `proposal.md` (R2 banner + §1/§4/§5/§8/§9/§10), `ethical-decisions.md`,
> `ADR-soft-access-gate.md`. Question: do the human rulings clear my R1 STOPs, and did they
> breed new ethical friction? Counsel remains advisory; the human already ruled.

### R1 STOP-1 — CLEARED.
One line: the human chose to make the gate *true* (invite-gating becomes a **blocking
prerequisite**, sequenced before launch) **and** reframed all copy to "register interest / keep
me posted" with a banned-strings list — so the narrative is no longer selling scarcity the
product doesn't have. The dark-pattern is removed at its root, not papered over.

**Residual friction (non-blocking, not a re-STOP) — sequencing honesty.** The strongest
remaining truth-question: the feature *exists in design and may ship code* before invite-gating,
but must not *launch* until invite-gating ships. That is honest **if and only if** the gate is
release-sequenced, not flag-toggled by a tired operator at 2am. The ADR/§8 call it a
"release-sequencing gate, **not** a code flag" — good, but a sequencing gate enforced only by a
checklist is exactly the kind of thing that slips under launch pressure (see the operator-fatigue
pathology). The interest-framed copy is truthful at *any* state (the doc says so, §9), so the
real failure mode is narrow: someone flips on the *banned* "waitlist/approved" strings before
invite-gating lands. **Advisory:** make the banned-strings list a *grep-able CI assertion* tied
to an invite-gating-shipped flag, not a human checklist line — turn the sequencing promise into
a test. That converts an honesty-promise into an honesty-proof. This is friction, not a STOP:
the copy ships honest today regardless.

### R1 STOP-2 — CLEARED.
One line: explicit consent (withdrawable, server-validated, evidenced per-row by `consent_at` +
`privacy_version`) + **day-one** erasure grant/script/runbook + **12-month** retention auto-erase
cron + an in-scope `/privacy` notice — the PII store now has a lawful basis, a purpose limit, a
stated bound, and an exit on day one. The erasure question is no longer silently parked; it is
answered before the first row is written. This is *more* than the minimum I asked for (I would
have accepted "legitimate-interest + manual erasure"); the human chose the more demanding,
more honest basis. I endorse it.

**Three new questions the consent decision raises — examined, none rises to a STOP:**

1. **`ON CONFLICT DO NOTHING` keeps consent pinned to the OLD `privacy_version` (N4) — is that
   still "consent"?** Yes, and it is the *correct* default. The stored `consent_at`/`privacy_version`
   are the lawful evidence of *what the person actually agreed to* on the day they agreed. Silently
   bumping them on a duplicate submit would be the dishonest move — it would forge a consent record
   for a notice the person may never have read. The doc reasons this exactly right (§4, N4). One
   honest caveat worth stating plainly in the runbook: **if the notice ever changes *materially*
   (purpose, retention, recipients), the old stored consent no longer covers the new processing** —
   at that point the deferred re-consent flow stops being "a future feature" and becomes a
   *lawful-basis requirement* for the affected rows. Non-blocking now (notice v1 is the only notice);
   **advisory:** record in the defer-flag that re-consent is promotable-to-required on a material
   notice change, so a future copy edit doesn't quietly invalidate stored consent. Not a STOP —
   it only bites on a future change that hasn't happened.

2. **Is 12-month retention proportionate for "register interest"?** Defensible, at the long-but-not-
   unreasonable end. The data is one email + a hash; the purpose ("contact about launch/access") has
   a plausibly long fuse for a pre-launch product whose own timeline is uncertain. GDPR asks "no
   longer than necessary," not "as short as possible," and 12 months is *stated, bounded, auto-enforced,
   and matches the in-repo `anonymizer.retention` convention* — proportionality is a judgment the
   human is entitled to make, and they made it explicitly with a number, not a silent default.
   **Advisory (non-blocking):** the *honest* shorter answer would be ~6 months (a launch list that
   hasn't converted in half a year is cold), but 12 is within the defensible band and consistency
   with the existing retention engine has real value. Not a STOP — it is stated, bounded, and enforced.

3. **Is withdrawal of consent *actually* designed, or only on paper?** Designed, and this is the
   part I checked hardest. "Withdraw consent = erasure" maps to the *same* day-one `DELETE` grant +
   `scripts/erase-access-request.ts <email>` + runbook (§7), with the notify worker tolerant of a
   row vanishing mid-flight (B8.1). There is no half-state "consent off but row kept." That is a
   real, exercisable right on day one, not a promise. **One honest gap (advisory, not a STOP):** the
   `/privacy` copy says the user "can withdraw consent or ask us to delete it anytime," but the
   *mechanism* offered to the user is implicitly "email the operator" — there is no self-serve
   withdrawal UI (admin/erasure UI is deferred, legitimately). That is acceptable for v1 PII-of-one-
   column **only if** the privacy notice names a concrete contact channel the person can actually use
   (an email address / contact route), so "anytime" is operable, not aspirational. **Advisory:** the
   `/privacy` page must carry a working contact for erasure requests (the §8 content list says
   "contact" — make sure it is a *reachable* address, not a placeholder). A right with no working
   channel is the paper-only failure I was probing for; the contact line closes it.

### New friction from consent-UX — examined.

- **Pre-checked checkbox = dark pattern?** Not present, and the doc is explicit the other way:
  the checkbox gates a **`disabled` submit button** (§10) — i.e. it ships **unchecked**, the user
  must affirmatively tick. A pre-checked consent box would be a GDPR-invalid dark pattern; the
  design is the opposite. **Cleared.** (Advisory, belt-and-suspenders: add a Playwright assertion
  that the checkbox is **not** checked on initial render — the proof list asserts "button disabled
  until ticked," which implies it, but an explicit `not.toBeChecked()` makes the no-pre-check
  invariant a *proven* line, not an inferred one.)

- **Is the consent text honest/readable?** The proposed `consentLabel` ("I agree to be contacted
  by email about access, and to the Privacy Notice") is plain, specific about *purpose* (contact
  about access), and links the notice — that is informed consent done correctly, not a buried
  legalese blanket. The `privacy` microcopy states the 12-month bound and the withdrawal right in
  one readable sentence. Honest and readable. **Cleared.** Minor advisory: "about access" leans
  very slightly toward the scarcity framing STOP-1 wants gone — consider "about launch" / "when
  we're ready for you" to stay fully on the interest side of the line. Cosmetic, human-polish.

- **Frictionless × consent balance.** The doc restates the invariant honestly: frictionless no
  longer means one field, it means "email + one consent checkbox + one button," and consent is a
  *deliberate, regulator-required micro-friction*. That is the right reframe — they did not try to
  smuggle consent into a frictionless story; they redrew the floor honestly. The temptation here
  would have been a pre-checked box "to keep it one-tap" (dark pattern); they explicitly refused it.
  **Balance is correct.**

### Aesthetics / strategy — is "register interest" + invite-gating-first a coherent product narrative?
Yes — and it is *more* coherent than R1. "Be the first to know" / "keep me posted" over an
honestly-bounded list, with the real gate coming as its own deliberate change, is a single true
story: we are warming a launch list now, and scarcity (if it ever comes) arrives as a real,
shipped mechanism — never as copy theatre over an open door. The success copy ("Thanks — we've
got your email and we'll be in touch") celebrates *being heard*, not *being selected* — exactly
the R1 advisory, landed. Conceptual integrity holds. One strategic note (non-blocking): when
invite-gating *does* ship and "waitlist/approved" copy becomes permissible, re-run this Counsel
pass on the *new* copy — the moment scarcity becomes real is also the moment the dark-pattern
temptation returns with permission. Flag it forward.

### No new ETHICAL-STOP.
The human rulings clear both R1 STOPs and introduce no new grounded red-line crossing. The
residual items above are **advisories** (sequencing-gate-as-CI-assertion; re-consent promotable
on material notice change; working erasure contact in `/privacy`; not-pre-checked Playwright
assertion; 12mo→6mo consider) — friction proportional to the stakes, none requiring a recorded
human decision. The design is now *more* honest and more PII-disciplined than the version that
triggered the STOPs. I endorse proceeding to build, with the proof obligations (§1044) already
correctly capturing the consent gate, retention sweep, privacy-version-no-drift, and erasure as
test lines.

**To the conductor — 1-line status:** R1-STOP-1 **cleared** · R1-STOP-2 **cleared** · **no new
ETHICAL-STOP** · top-2 advisories: (1) make the launch-sequencing/banned-strings gate a CI
assertion tied to an invite-gating-shipped flag, not a human checklist line; (2) ensure
`/privacy` carries a *working* erasure-request contact so "withdraw anytime" is operable, not
aspirational.
