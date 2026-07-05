# S10-PLATFORM-ADMIN / PROVISIONING Port — Council Packet · COUNSEL OPINION

> Seat: **Counsel** (ethics · aesthetics · strategy) in the S10-platform-admin/provisioning Triadic
> Council — the **highest-privilege plane** (platform-admin *above* owner) and the **LAST** strangler
> surface (its flip is the Phase-D decommission trigger). Advisory, non-blocking. Architect asks "will it
> work"; Breaker asks "how it breaks"; this seat asks *should it exist, is it fair/honest/dignified, is it
> whole, what is the long horizon, who bears the cost, and what did nobody ask.* Docs only — no code, no
> design authority. Every load-bearing claim below was verified against live source, not trusted from the
> packet.

---

## OPINION: **PROCEED-WITH-REVISIONS** · **NO ETHICAL-STOP**

This is the most-privileged surface in the system — the one plane whose principal reads and acts *across
every tenant*, holds the DR triggers, and provisions net-new tenants from scratch — and the packet is the
most disciplined of the ten. I looked adversarially at every place S10 could cross a grounded line and
touch a real person at this tier: the owner who could silently become an admin; the tenant whose data is
read cross-tenant with no visibility; the backup key one log-line from the whole database; the
non-consenting restaurant whose storefront a claim token could grant to the wrong party; the "temporary"
Node front-door that never gets cut. **On every grounded line I can verify, the packet either holds it,
fixes it, or names it as an owned residual with a trigger — so the friction here is Opinion and
conditions, not a stop.** Issuing a STOP on a packet that already installs the honest disposition would
be verdict-not-friction, the overreach my mandate forbids. This tracks the **S5** posture (a disciplined
red-line port, closed lines, conditions instead of a stop), not the **S9/S4** posture (a grounded line
left crossed on the live system).

**The three things my verification changes or sharpens (load-bearing):**

1. **The single sharpest correctness fact of the packet is TRUE in the live rebuild, not aspirational — I
   read the bytes.** The Rust `Claims` enum is genuinely 3-role (`claims.rs:151-156`, Owner/Courier/
   Customer), there is no `PlatformAdminClaims` variant, and `unknown_role_is_rejected` (`:390-394`)
   already refuses a `"superadmin"` token *today*. The highest privilege in the system is **structurally
   un-forgeable** — an owner cannot become an admin by minting a claim, because the claim shape cannot
   *represent* an admin. This is the anti-capture control the whole surface rests on, and it is real. I
   affirm it without addition; it is the design-language high point of S10.

2. **The Phase-D "record now" disposition has ALREADY been checked off empty once — so Q5b needs teeth,
   not affirmation.** REV-C10 (my own prior finding on the cutover-harness) recorded, in that packet's
   resolution (`resolution.md:66-68, 94`): *"Phase-D cut-trigger + owner recorded now."* I read it: **no
   actual name and no actual date were written** — the act of recording the trigger was itself deferred to
   a placeholder. That is the un-cut vine reproducing in miniature: even the commitment-to-commit got
   softened to an intent. So on the LAST surface, whose flip *is* the trigger, "record it now" is not
   enough — the operator must write a **concrete named owner and a concrete date**, and the S10
   approval/flip must gate on the slot being *filled*, not merely acknowledged again. This is the one
   place I strengthen hardest (§5, §C-5).

3. **The claim/Art-14 path is an ethical high point the port must *protect*, not just carry.** The system
   builds a preview of a restaurant *without its consent* (hostile recipient, Art-14). Verified in the
   live code (`claim.ts:174-200`, `public/claim.ts:49-83`): the honest response is an equally-prominent,
   one-click, **no-account** decline-and-erase, alongside an honest notice that names the source, purpose,
   and rights. That is anti-dark-pattern design doing ethical work — the non-consenting party's dignity is
   protected by making the *exit* as easy as the *claim*. The port must keep decline **at least as
   prominent and at least as frictionless** as accept; a rebuild that made "erase me" harder than "claim
   it" would be a dark-pattern regression on the surface where consent is most fraught (§3, §C-3).

Everything else is Opinion, affirmation, or a condition. The axum plane-gate mechanism (Q1a), the
`/internal/*` reachability (Q2a), the cross-tenant read B3 blocker (Q4a), the DR-drill hardening — these
are Breaker's robustness domain and/or the packet's own named cutover prerequisites; I affirm them and add
the ethical/strategic dimension without re-deriving the Breaker.

---

## Verification note (I read the live source behind every load-bearing claim)

- **The admin authority is an allowlist fact, not a token role — confirmed in the gate itself.**
  `platform-admin.ts:20-26` — `isPlatformAdmin` is `SELECT 1 FROM platform_admins WHERE user_id = $1 AND
  revoked_at IS NULL`, a plain non-tenant point-read; `:33-54` `requirePlatformAdmin` maps no-userId→401,
  miss→403, and a **DB throw→503 fail CLOSED** (`:44-48`, "never fail-open at the top tier"). The
  plane-gate (`:63-83`) keys on `request.routeOptions.url` (the **matched pattern**, not the raw URL) and
  excludes the `/api/administrators` lookalike (`:66`, boundary `=== '/api/admin' || startsWith
  '/api/admin/'`). Confirmed: the authority cannot be forged in a token and cannot fail open.
- **The 3-role enum structurally forbids a 4th — confirmed in the Rust port that already exists.**
  `rebuild/crates/api/src/auth/claims.rs:151-156` is a 3-variant enum; `:391-394`
  `unknown_role_is_rejected` asserts `{"role":"superadmin"}` fails to parse. There is no admin variant and
  no mint site. The packet's central authz claim is not a promise — it is live and tested.
- **No restore-to-prod endpoint exists — confirmed by what the routes *are*.** The three admin backup
  routes are list / drill / drill-report; the drill (`runRestoreVerify`) targets a sandbox. The real DR
  restore is a manual runbook. The packet's disposition is to **refuse to build** an authenticated
  restore-over-prod trigger. Confirmed as the honest posture: the port does not invent a new irreversible
  capability the system deliberately lacks.
- **The backup key is env-only and fail-loud — confirmed in the crypto module.** `encrypt.ts:44-72`
  `resolveBackupKey` reads `BACKUP_KEYRING`/`BACKUP_ENCRYPTION_KEY` from `process.env` directly (not the
  Zod schema, `:42` comment) and **throws on an unknown keyId** (`:66-70`, "Refusing to restore with an
  unverified key"). This is the restore-to-wrong-target control. Confirmed. (One observation, not a
  finding: the writer stamps `keyId: 'primary'` as a hardcoded literal, `:25` — key *rotation* is a
  read-side keyring lookup only; a genuine rotation would need a writer change. Out of S10 scope; noted so
  the port does not assume rotation is exercised.)
- **The claim theft guard is live — confirmed at the branch.** `claim.ts:107-115` — `acceptClaim` selects
  `invited_contact_hash` and **throws `CONTACT_REQUIRED`** when it is NULL (`:113-114`), refusing a
  token-only invite on the web path; `public/claim.ts:36` maps it to 403. The `claim_transfer` DEFINER is
  the sole transfer authority with org/location derived in-fn (`claim.ts:97-117`). Confirmed: the R3-1
  theft vector (a leaked token-only invite binding ownership to any account) is closed on the web path.
- **The Art-14 notice + equally-prominent decline are real — confirmed.** `claim.ts:174-200` builds a
  notice written for the *hostile recipient* ("You did not ask us to do this") with an EQUALLY-prominent
  one-click delete; `public/claim.ts:69-83` `POST /claim/decline` is **token-only, no auth, no account**.
  Confirmed as the consent-respecting exit.
- **Phase-D was recorded as an intent, not a filled commitment — confirmed.** `cutover-harness/
  resolution.md:66-68` records the REV-C10 direction ("dated cut-trigger + named owner NOW") and `:94`
  lists it as done ("recorded now") — but **no name and no date are written** anywhere in the resolution.
  The slot is still empty. Confirmed: Q5b is not a fresh ask, it is a *second* attempt to fill a slot the
  first "record now" left blank.

---

## By charge

### 1. Concern 1 — platform-admin = power over ALL tenants: is it held narrow, and does the rebuild silently widen it? (Q1/Q1a)

**Held narrow, verified — and the packet closes every widening vector I can find. Affirm; add one
aesthetic-as-ethics note.** The four controls that keep this power narrow are all present and (where
already ported) live: authority is an **allowlist fact, not a JWT role** (so it cannot be forged or
self-minted — verified `claims.rs`, the 4th variant is structurally impossible); the gate is
**B3-independent** (a plain non-tenant no-RLS point-read, no GUC, no DEFINER — so it reads identically
under either pool posture); revocation is **immediate** (`revoked_at` re-read per request); the
`platform_admins` table is **SELECT-only** to the operational role (self-serve escalation is structurally
impossible). This is the correct shape for the top privilege tier and I affirm it.

**On "does the rebuild silently widen this power?" — the answer is: only in two places, and the packet
names both.** (a) The **axum plane-gate (Q1a)** is the one non-verbatim carry: Fastify's root
`onRequest` hook keyed on the matched pattern has no axum equivalent, so a future admin route registered
*outside* the gated `Router` would silently escape the gate — a cross-tenant breach at this tier. The
packet's disposition (nest ALL admin routes under one `route_layer`ed `Router` + a clippy/test tripwire +
a re-proven sibling-closure test) reproduces the *property* with a different mechanism. This is Breaker's
robustness domain; I affirm it and add only that the sibling-closure test is the **coverage authority**,
not the router-nesting alone — the property "no admin route can escape the gate" must be *proven by an
attack* (a throwaway ungated sibling → 403), because a structural guarantee that is never adversarially
tested is a comment, not a control. (b) The subtler widening vector is **aesthetic**: the pull to "clean
up" four different gates into one unified auth middleware. See §Non-blocking note 1 — this is the place a
*simplification* would be a silent power-widening, and the packet's "do not merge the two gates"
(Q-PROVISION-SECRET) is correct.

**Charter tie (commons / anti-capture).** The Charter says AI is a commons, "never captured for the
exclusive benefit, control, or enrichment of any narrow group." The highest privilege in this system is a
short server-side allowlist, revocable in one request, un-forgeable in a token, and audited. That is the
*mechanical* expression of anti-capture — power that is narrow, legible, and removable. Affirm; the port
must not trade any of the four for elegance.

### 2. Concern 2 — backup/DR + encryption keys: is the restraint enough, or will the rebuild add an authenticated restore-over-prod? (Q3)

**The restraint is not just enough — it is the *point*, and it is verified. Affirm strongly; the one risk
is temptation, and I give it a forcing function.** Three facts hold the line: (a) **no restore-to-prod
endpoint exists**, and the packet *refuses to build one* — an authenticated "restore over prod" HTTP
trigger would be a net-new, weaponizable, **irreversible** capability the system deliberately does not
expose, and building it as a *side effect of a port* would be the worst possible way to introduce it
(reversibility lens: the one operation with no undo should never arrive by inertia); (b) the drill targets
a **sandbox**, and the port must assert the smoke pool's connection string is the sandbox, not prod (the
LC7 fix-2 bug is the canonical breach); (c) the **backup key is env-only and fail-loud** (verified
`encrypt.ts:44-72`), so a wrong/only key refuses rather than silently producing garbage — this is the
restore-to-wrong-target control, and the secrets-exposure-incident is the standing reason the key must
never reach a log or a commit.

**The only real risk here is future temptation, and it deserves a forcing function, not just a "no."** The
steel-man for building the endpoint is genuinely strong (see §Steel-man below) — a real 3am DR event
served by a panicked human running `pg_restore` with raw superuser creds is *itself* a risk (human error,
no double-confirm, no audit trail, no target-pin). The honest disposition is therefore **not** "the
runbook is fine, move on" but: keep restore-to-prod out of S10 (a filled S10 coverage-assertion that the
namespace exposes no restore-over-prod route), AND own the runbook's own risk explicitly — it must be a
*genuinely good* runbook (double-confirm, target-pin, audit line, drilled at least once), not a hand-wave,
because "we don't have the dangerous endpoint" is only honest if the manual path it defers to is actually
safe. Any future confirmation-gated restore endpoint is its **own** council (blast-radius, double-confirm,
target-pin), never an S10 side effect (§C-2).

### 3. Concern 3 — provisioning + ownership transfer: is "needs work" an owned residual or a silent hole? Real people (a new business owner). (Q2b)

**The live theft guard is real and the exit is dignified — this is a genuine high point. The residual is
owned in *shape* but under-specified in *content*; name what P6 deferred, or "owned residual" is a label
over an unexamined gap.** Verified: the web claim path refuses token-only invites (`CONTACT_REQUIRED`,
`claim.ts:113`), so the R3-1 theft vector (a leaked token binding a whole tenant to any authenticated
account) is closed on the public surface; the transfer is one atomic DEFINER statement with org derived
in-fn (no IDOR); and — the part the packet under-celebrates — the *non-consenting* restaurant gets an
**equally-prominent, no-account, one-click erase** (§Verification, `claim.ts:174-200`). That last is the
dignity control that makes the whole build-a-preview-without-consent posture defensible: the person whose
storefront was created without asking is given a first-class way to say no. **Protect it in the port**
(§C-3): decline must stay at least as prominent and frictionless as accept.

**Where I press.** "Ownership transfer needs work" (P6) is disposed as an accepted-risk row + owner
(Q2b(a)) — the right *shape*. But the packet does not enumerate *what* P6 deferred, and an accepted-risk
row that says only "needs work" is where a real gap goes to hide behind a checkbox. The residual must name
the concrete deferred hardening (from the P6 memory: the recipient-binding model, the invite lifecycle,
what happens when the "contact on file" is a shared `info@` inbox or a stale address, whether a
mis-directed invite can be *revoked* before acceptance). The real people here are two: the small-business
owner who could receive a storefront that isn't cleanly theirs, and the one who *loses* the claim window
because the invite went to a dead address. Neither is a red-line crossing (the theft guard holds), but
both deserve a *specific* owned line, not "needs work" (§C-3). And, per the mandate, this hardening is its
own future council — do NOT couple a security port to a net-new recipient-binding design at the worst
moment (that would repeat the S5 lesson: right about the destination, wrong about the vehicle).

### 4. Concern 4 — the cross-tenant reads (Q4a) and `/internal/*` reachability (Q2a): the isolation questions

**Both are correctly held as owned/gated, not silent. Affirm; sharpen the honesty of each residual.**

- **Q4a (the cross-tenant admin reads).** `fallback/health`/`r2-check` read across ALL tenants and work
  today only because the pool is BYPASSRLS; post-B3 they return **0 rows** unless a platform-read path is
  built (B4 R1, unbuilt). The packet makes building it a **cutover prerequisite** and gates the flip on
  it — the right disposition. My additions are two. First (care/ops-reality): a cross-tenant recovery read
  that silently *empties* at the flip is a broken recovery tool **at the exact moment it is needed most**
  (an incident) — so the proof must be that `fallback/health` returns *fleet rows* under NOBYPASSRLS
  *before* the flip, not merely that the fn compiles. Second (the deeper counsel point): "works only via
  BYPASSRLS" is not just an availability bug — it is a **masked isolation question**. The reads have never
  been forced to state, in a policy, *who* is allowed to see every tenant's config and public phones;
  BYPASSRLS answered "the pool" by accident. Building the platform-read path is the chance to make that
  answer *explicit and legible* (a named DEFINER/role whose grant is auditable) rather than an emergent
  property of a superuser pool. Do it as legibility, not just as a 0-rows fix.

- **Q2a (`/internal/*` externally reachable).** A cross-tenant *write* front door bounded by one shared
  secret on a public interface. The packet carries the secret-gate (timing-safe, fail-closed-404,
  decoupled from dev-login, rate-limited) and **defers network isolation as a named-threshold residual** —
  acceptable *only* as an explicit, owned defer, never by silence. My sharpening: the threshold must be a
  **real trigger with an owner** (e.g. "first non-founder ops hire OR N tenants ⇒ segment `/internal/*`"),
  not "when it feels right" — the same anti-vagueness discipline as Phase-D, one tier down. And the port's
  invariant "does not widen `/internal/*` reachability" must be *proven* (a test that the acquisition
  routes are not exposed on any additional interface the port introduces), because "we didn't widen it" is
  the kind of claim that is true until a convenience edit quietly makes it false.

### 5. Concern 5 — Phase-D un-cut vine (Q5b): the closing act. Strengthen it — it has been checked off empty once already.

**This is the packet's own required-human-decision gate — which is *why* no separate ETHICAL-STOP is
needed — but it must be strengthened so it cannot be satisfied by a placeholder, because it already has
been.** S10's flip means all ten surfaces are Rust; the Node front-door shim has then done its job and
must be cut (Phase D). The strategic risk is the oldest in infrastructure: the "temporary" shim becomes
the permanent load-bearing incumbent — and a rebuild whose *entire point* is to escape a stack it no
longer wants to be captive to would, by never cutting the vine, plant a **new** permanent incumbent (the
Node front-door): the exact lock-in shape it set out to leave. This is the handoff's open item
(`терпіння↔прив'язаність`) realized: from the inside, "not yet time to decommission Node" and "I never
actually intend to" are indistinguishable **without a pre-committed, dated trigger and a named owner.**

**Why I strengthen rather than merely affirm.** The packet's disposition (Q5b(a): record a dated trigger
+ named owner NOW) is correct — and it is, in effect, a friction gate the packet already installed (a
recorded human decision the human must make; an agent cannot self-assign the owner). A STOP would be
redundant with it. But my verification found the load-bearing fact: **this exact disposition was already
recorded once, and left empty** (`resolution.md:94` "recorded now" — with no name, no date). So affirming
"record it now" a second time risks the same empty checkbox. The strengthening (§C-5):

1. The Phase-D line must be written with a **concrete named owner** (a person the operator names — I
   cannot, and no agent can, self-assign it) and a **concrete date or a concrete dated condition**
   ("S10 flipped + all-ten stable ≥ N days ⇒ front-door role migrates to Rust, Node shim deleted, by
   `<named person>` before `<date>`"). "Recorded now" as an intent does not satisfy it; a filled line
   does.
2. **S10 approval — and the S10 flip — gate on the slot being *filled*, not acknowledged.** Because S10's
   flip *is* the trigger, an S10 that flips with the slot still blank fires the trigger into a void and
   makes the vine permanent by exactly the mechanism REV-C10 named. The DoD item "Phase-D trigger + owner
   recorded" (§12) must assert a *non-placeholder* value.

This is the single most important thing this seat can contribute to the last surface: not a new argument,
but *teeth* on the packet's own closing item, because the evidence shows the honest disposition can be
checked off in name only.

### 6. Scope + Charter

- **Scope is disciplined.** The backup CRON worker (S8), the GDPR export/erase DEFINER fns (S9), the
  `cutover_flags` mechanism (the harness's ADR), the row-180 two-writer and the DR-drill orchestration
  (permanent-Node carve-outs) are all correctly excluded or explicitly carved out — no schema change, no
  net-new capability. The one non-verbatim change (the axum plane-gate, Q1a) is a mechanism port, not a
  scope expansion. Clean boundary; the port owns the *authority*, not the heavy pipelines.
- **Charter: clean, and mechanically expressed.** Orders across tenants, provisioning, DR — no military/
  warfare, no surveillance-for-harm-by-intent, no commons-capture. The anti-capture spirit is realized in
  code: the top privilege is a narrow, revocable, un-forgeable, audited allowlist. The one Charter-adjacent
  long-horizon note that is *not* caught by any per-surface gate is the un-cut vine (§5) — a rebuild that
  never cuts the Node front-door recreates the captivity it set out to escape. That is why Phase-D is the
  Charter's closing test, not a footnote.

---

## Non-blocking aesthetic / strategic notes

- **Four gates, deliberately un-unified — the anti-elegance that is the ethical high point.** S10 has four
  auth mechanisms (allowlist / ops-secret / owner-JWT / claim-token). The rebuild's strongest aesthetic
  pull will be to "clean this up" into one unified auth middleware — fewer seams, more elegant. **Resist
  it.** Merging the ops-secret gate into the platform-admin allowlist is exactly what B4 rejected (enabling
  provisioning must not require an owner-JWT-minting admin session; a cron POSTs the retention sweep with a
  secret, not a human token); folding the claim-token into the admin plane would couple ownership-transfer
  to platform privilege. Here, **four seams are more honest than one**, because collapsing them *couples
  authorities that must stay decoupled* — and coupled authority is concentrated authority, the capture
  shape. This is the rare case where my usual "aesthetics as leading-indicator of ethics" runs the other
  way: the *simpler* design would be the *less* ethical one. Name the boundary so no one refactors it away.

- **The B3-independent plain point-read is the correct beauty, and it looks ugly.** The gate is a bare
  `SELECT 1 FROM platform_admins WHERE user_id = $1 AND revoked_at IS NULL`. It is tempting to "improve"
  this into a principled-looking GUC/DEFINER path — but B4 (RA2-3) killed exactly that, because it only
  *relocates* the BYPASSRLS dependency to the fn owner. The plain read is B3-independent *because* it is
  plain. The port must not beautify it into a trap. Elegance here is the restraint of leaving the ugly
  thing ugly.

- **"Schema-rich, runtime-minimal" done right — the DR-drill stays on Node.** Re-porting a 400-line
  `pg_restore`/decrypt/stream/sandbox pipeline to Rust for two cold-path endpoints would be the *evil
  twin* of the doctrine (rewriting runtime you do not need, onto a large new subprocess/crypto/superuser
  attack surface). The thin Rust trigger (gate + audit + uuid + rate-limit → invoke the Node drill) is the
  *good twin*. Affirm the restraint; the Rust surface owns the *authority*, not the pipeline.

- **The Art-14 notice is aesthetics doing its ethical job — protect it, don't just carry it.** A notice
  written for the person who *did not ask*, with an erase as prominent as the claim and needing no
  account, is a whole, honest surface — no dark-pattern seam for a false promise to leak through. In the
  port, the decline path's prominence and frictionlessness is a *design invariant*, not a cosmetic detail:
  the moment "erase me" is one tap harder than "claim it," the surface has quietly become a growth-hack.

---

## Steel-man of a rejected option (obligatory)

**Q3a option (b) — "build a confirmation-gated restore-to-prod endpoint now" — the option the packet
rejects and I land against.**

Its strongest case, made fairly: **the manual runbook it defers to is not obviously safer — it may be
*more* dangerous.** A real DR event is a worst-case moment: production is down or corrupted, the clock is
running, and the current "control" is a human with raw superuser DB creds (`DATABASE_URL_ADMIN`) typing
`pg_restore` against prod from a shell — under pressure, with no double-confirm, no target-pin, no
enforced audit line, and every opportunity for the exact wrong-target/wrong-key error the fail-loud key
control was built to catch on the *drill* path but that nobody wired on the *manual* path. An in-app,
confirmation-gated restore — double-confirm, backup-target-pinned, keyId-verified, write-ahead-audited,
rate-limited, platform-admin-gated — could be **safer** than a panicked human with psql, because it moves
the irreversible operation from tribal-knowledge-under-stress into a control with the same discipline S10
already applies to the *drill*. "We don't expose the dangerous endpoint" is only honest if the path it
defers to is genuinely safe, and a manual runbook is precisely where safety erodes silently between
drills. That is a strong position and I do not dismiss it.

**Why I still land on no-endpoint-in-S10.** Two things the steel-man underweights. First, the **vehicle**:
introducing the one operation with no undo *as a side effect of a port whose DoD is byte-parity* is the
worst possible moment and process — a net-new irreversible capability deserves its own council with a
blast-radius model, a double-confirm/target-pin design, and a threat review, not a rider on the last
surface's flip. Second, and decisively: I **adopt** the steel-man's real insight — that the manual runbook
is a live risk, not a safe default — but route it to the *right* fix. The honest response is not "rush an
endpoint into S10" but "keep restore-to-prod out of S10 *and* make the runbook genuinely safe now"
(double-confirm, target-pin, an audit line, drilled at least once), so the deferral is honest rather than a
hand-wave (§C-2). So (b) is **right about the danger** (the runbook is not automatically safe) and **wrong
about the vehicle** (not as an S10 port side-effect) — the same shape as the S5 promotions call: adopt the
urgency, reject the coupling.

---

## Explicit conditions before code (§C)

Advisory — the human decides. Counsel-weighted, ordered by ethical/strategic load.

1. **[authz, §1] Prove the plane-gate closure by attack, and keep the four gates un-unified.** The
   sibling-closure test (a throwaway ungated `/api/admin` sibling → 403; the `/api/administrators`
   lookalike NOT gated) is the coverage authority for Q1a, re-proven **in Rust**, not assumed from the
   router nesting. Carry all four gate mechanisms **distinct** (allowlist / ops-secret / owner-JWT /
   claim-token); do NOT merge any two for elegance (Q-PROVISION-SECRET). Keep the B3-independent gate a
   plain point-read — do not "improve" it into a GUC/DEFINER path (B4 RA2-3).

2. **[DR/irreversibility, §2] No restore-to-prod in S10 — and own the runbook's own risk.** A filled
   coverage-assertion that the S10 namespace exposes no restore-over-prod route (Q3a). Carry the key
   posture verbatim (env-only, fail-loud on unknown keyId, `redactPII` on every drill line, a grep-gate
   that no log prints the key / R2 secret / `DATABASE_URL_ADMIN`) and assert the smoke pool targets the
   **sandbox**, not prod (Q3c/S10-T5). **Adopt the steel-man:** the deferred-to manual runbook must be
   genuinely safe (double-confirm, target-pin, audit line, drilled once) — "we lack the dangerous
   endpoint" is only honest if the fallback path is. Any future restore endpoint is its own council.

3. **[provisioning/consent, §3] Name what "ownership transfer needs work" *is*, and protect the decline's
   prominence.** The Q2b accepted-risk row must enumerate the concrete P6-deferred hardening
   (recipient-binding, invite lifecycle, shared/stale contact, mis-directed-invite revocation) with a
   named owner and a future-council trigger — not the word "needs work." Carry the `CONTACT_REQUIRED`
   theft guard verbatim (Q2b). **Design invariant:** the decline-and-erase path stays token-only,
   no-account, and **at least as prominent and frictionless as accept** (the Art-14 dignity control); a
   port that makes "erase me" harder than "claim it" is a dark-pattern regression.

4. **[RLS/legibility, §4] Build the platform-read path as legibility, not just a 0-rows fix; name the
   `/internal/*` threshold.** Prove `fallback/health` returns *fleet rows* under NOBYPASSRLS **before** the
   flip (Q4a), and build the platform-read DEFINER/role so *who may read every tenant* is an explicit,
   auditable grant, not an emergent property of a superuser pool. For Q2a, the network-isolation defer must
   be a **real named-threshold trigger with an owner** (not "when it feels right"), and the port's
   "does-not-widen-`/internal/*`-reachability" invariant must be proven by a test.

5. **[long-horizon, §5 — the closing act, strengthened] Fill the Phase-D slot with a name and a date, and
   gate S10 on it being filled.** Q5b must be satisfied by a **concrete named owner + concrete dated
   condition** in the cutover-harness ADR — *"S10 flipped + all-ten stable ≥ N days ⇒ front-door role
   migrates to Rust, Node shim deleted, by `<named person>` before `<date>`."* "Recorded now" as an intent
   does **not** satisfy it (it already was, and left blank — `resolution.md:94`). The S10 DoD item and the
   flip gate on the slot holding a **non-placeholder** value. The named owner is an **operator decision**
   — no agent (this one included) can self-assign it; this seat surfaces the empty slot and refuses to let
   the last surface flip into it blank.

---

## The question nobody asked (§7)

Every seat in this council — and the tasking — speaks for constraining the platform-admin's power *from
the platform's side*: the allowlist keeps it narrow, the plane-gate keeps it un-escapable, the audit trail
records what it does, the fail-closed keeps it from failing open. That frame is correct and well-built, and
it is the right way to bound the highest privilege in the system.

**Nobody speaks for the tenant as the *subject* of the platform-admin's cross-tenant reads — the person
watched by the watchman, who has no way to know she was watched.** When a platform-admin runs
`fallback/health` or `r2-check`, the read crosses **every** tenant's `locations` and
`owner_notification_targets` — including **public phones** (asset A2, verified in the packet's own scope,
`fallback.ts:14`). The small-business owner whose config and contact are read has **no signal** that the
platform looked at her tenant. The only check on the admin's cross-tenant reach is the
`platform_admin_audit_log` — a trail the platform-admins *themselves* write and *themselves* read
(`platform-admin.ts:116-141`). At the highest privilege tier, the sole accountability mechanism is a log
the watched party cannot see and the watching party controls. That is *quis custodiet ipsos custodes* in
its literal form: **who audits the auditor?**

The unasked question is not technical and it does not block S10 (the audit trail is a real control, and
the reads are cross-tenant *by design* — that is the job): *the whole surface works to keep the
platform-admin's power narrow and legible **to the platform** — what makes it legible to the **people whose
data it reads**?* The honest answer is the same shape the Charter points to — "AI... belongs to and must
serve everyone... never turned against the people it was learned from": the audit trail should be
**tamper-evident** (append-only, so an admin cannot quietly erase their own read of a tenant) and
**reviewable by someone other than the admins who write it** (an operator/oversight role, or in the long
horizon, a tenant-facing "your data was accessed by platform operations on `<date>` for `<reason>`"
signal). None of that is S10's to build, and it is correctly out of scope. But naming the missing seat is
the least this council can do: the surface that most carefully bounds the watchman's power has, so far,
made him accountable **only to himself** — and the tenant he reads, the one person the design is ultimately
for, is the one it never tells. On the last surface, whose whole ethic is anti-capture, that is the
question worth leaving on the table for whoever owns the next horizon.

---

*Advisory only. NO ETHICAL-STOP — the packet holds every grounded line I can verify; Q5b is the packet's
own required-human-decision gate, which I strengthen (fill the slot with a real name + date; gate the flip
on it) rather than duplicate. The human is final. Nothing here authorizes a code change, blocks S10, or
overrides a conscious operator decision.*
