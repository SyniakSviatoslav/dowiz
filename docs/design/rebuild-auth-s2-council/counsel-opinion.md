# S2-AUTH Port — Council Packet · COUNSEL OPINION

> Seat: **Counsel** (ethics · aesthetics · strategy) in the S2-auth Triadic Council.
> Advisory, non-blocking. Architect asks "will it work"; Breaker asks "how it breaks";
> this seat asks *should it exist, is it fair/honest/dignified, is it whole, what's the long
> horizon, who bears the cost, and what did nobody ask.* Docs only — no code, no design authority.
> Every factual claim below was verified against live source, not trusted from the packet.

---

## OPINION: **PROCEED-WITH-REVISIONS**

No ETHICAL-STOP. Nothing in the S2 surface serves harm or crosses the Charter; several parts
embody counsel-shaped virtues already (see §6). The revisions are not corrections of the security
design (that is the Breaker's seat) — they are three *deferral-hygiene* conditions and one
*cutover-irreversibility* condition, plus one cheap humane fix. The packet is unusually honest and
ethics-aware; my value is narrow and I keep it narrow.

**Verification note (charge: "verify the packet's claims").** I read the live source behind the
load-bearing assertions. They hold:
- RS256 double-pinned (`jwt.ts:105-111`); dev-kid accepted only `NODE_ENV!=='production' && JWT_DEV_KID` (`jwt.ts:91-102`); `signDevToken` **throws** without a dev keypair so no mint site can fall back to prod-key signing (`jwt.ts:76-78`).
- Customer token carries **only** `{role, orderId, locationId, sub}` — no phone, no email, no name (`jwt.ts:117-132`).
- Boot **fail-fast** is a real deterministic refusal: `loadEnv()` throws on `NODE_ENV=production` if any of `ALLOW_DEV_LOGIN / DEV_AUTH_SECRET / JWT_DEV_KID / JWT_DEV_{PRIVATE,PUBLIC}_KEY` is set (`config/index.ts:230-244`).
- Dev gate fails closed — `devLoginAllowed` requires flag **AND** secret; `isDevRequestAuthorized` returns false when not allowed (`dev-guard.ts:30-32,54-62`).
- Tokens live in `localStorage` via `safeStorage` (`dos_access_token`/`dos_refresh_token`, `apiClient.ts:18-48`, `safeStorage.ts:8`) — AUTH-GAP-5 is real, not theoretical.
- ADR-0004 revocation (P-a/b/c/d, ≤24h accepted-risk, honest-copy requirement) and ADR-0003 four-layer fail-closed + R-6 (leaked `kid:1` killable only by rotation) are present as described.

---

## By charge

### 1. AUTH-GAP-5 — localStorage tokens, XSS-exfiltrable

**Is the accepted-risk honest?** Yes. It is a named row (AR-5), a trust boundary (TB-6), it lists
the exact assets exposed (A2/A3/A4), and the recommendation is carry-for-parity + httpOnly as a
council row — not a silent default. That is the honest shape. I do not ask the port to change the
transport; I agree the parity oracle loses its power if the port also moves transport (a parity
failure would then have two possible causes — the *epistemic* case for deferral is real, not just
convenient).

**Where I add friction — care + long-horizon lens.** The ethical failure mode here is not the
deferral; it is **deferral becoming permanent by inattention** — the accepted-risk that outlives
everyone who remembers accepting it. "Fast-follow council row" is a promise with no teeth. Two
sharpenings:

- **Weight refresh-first, not "httpOnly everything."** The asymmetric value is the *refresh*
  token: an access token is bounded (owner ≤24h, customer 7d), but a stolen owner **refresh** token
  is a 7d rolling family — and AR-1 (the FE "Exit" never clears `dos_refresh_token`) compounds it.
  The packet's own Q5 option **(c) hybrid — access in memory, refresh in httpOnly cookie** is the
  ethically-weighted answer, because it moves the high-value, long-lived credential off the
  XSS-reachable surface while leaving the shared-token *access* seam intact. Name (c), not a flat
  "httpOnly," as the fast-follow target.
- **Who bears the cost is not symmetric across the three assets.** The owner and courier *chose* to
  log in. The **customer** did not — they clicked a tracking link and now hold a 7d bearer, in
  `localStorage`, on the **storefront** (the most public, most third-party-script-laden surface),
  that keys to *their own delivery address and contact* (A4). That is the sharpest edge in the whole
  surface, and the packet's platform/vendor-centred framing under-weights it. See the open question
  in §7.

Not a stop: a *current* property, honestly documented, defensible parity rationale. Condition: an
owner + a trigger on the AR-5 row, and refresh-first weighting (see §C).

### 2. AUTH-GAP-4 — no password-reset flow at all

**Strategically: correct to keep out of S2.** Adding a credential-recovery flow inside a red-line
auth port is scope creep on the exact surface where scope creep produces the worst bugs; reset flows
are the #1 account-takeover vector, and a weak reset on a money-handling owner account is a *larger*
harm than a support ticket. Q12(a) is right.

**But the dignity/care lens is not "build it in S2" — it is "does the port make the current absence
legible or does it launder it into permanence."** The stakeholder is a small-restaurant
owner-vendor — often non-technical, time-pressured, and precisely the "collective tool serves
everyone" constituency the Charter names. Forgetting a password is the single most common auth event
in the world; today it produces a dead end. During the launch push (trigger = first real paid
order), a locked-out owner at 8pm Friday with a queue is not an edge case.

Two honesties temper this, and I state them rather than dramatize:
1. It is **no *self-service password* recovery for password-only accounts**, not "no recovery at
   all" — Google OAuth (AUTH-02) and Telegram (AUTH-03) linking exist, and support-mediated reset
   exists. That materially softens the harm.
2. Absence is only unethical when it is **silent** — a "Forgot password?" link that goes nowhere, or
   a bare "wrong password" with no signposted path.

So: not a stop, not a port-blocker. The port's job is to make the absence *legible*, not fill it.
Cheap humane fix (in-scope-adjacent, no red-line surface): the login page should tell a locked-out
password-only owner their *actual* path (sign in with Google/Telegram, or contact support with your
correlationId) instead of leaving them at a dead end. And the backlog row needs an owner + a
pre-launch trigger (see §C), so "greenfield backlog" does not become "never."

### 3. dev-bypass discipline — is prod-inert deterministic?

**Yes — and this is the strongest part of the packet.** Prod-inertness is a *deterministic
refusal*, not "the model won't do it," and it is defense-in-depth with three independent
deterministic locks, verified live:
1. **Boot fail-fast** — the box *refuses to boot* (`config/index.ts:230-244`).
2. **Crypto segregation** — a prod verifier holds no dev public key and `acceptDevKid`
   short-circuits false on prod; `signDevToken` *throws* without the dev keypair (`jwt.ts:76-102`).
3. **Gate fail-closed, default-off** — flag AND secret required (`dev-guard.ts:30-32,54-62`).

The Rust port **adds a fourth, structurally stronger lock**: `#[cfg(feature="dev-routes")]` compiles
the dev handlers *out of the release binary entirely*. This is the right shape — the capability that
cannot be misused is the one that is not present in the artifact, not merely refused at runtime. It
also *removes one leg* of the ADR-0003 R-10 residual ("prod-rejection is by-construction under secret
hygiene"): a release binary with dev routes compiled out cannot mint or verify a dev token even if a
keypair is pasted, because the code path does not exist. I affirm this as exemplary.

Two conditions to keep it honest (see §C):
- The compile-out must be a **proven** property, not an intention — a DoD test that the *release*
  artifact (feature off) 404s `/dev/*` **and** rejects a dev-kid token. A structural guarantee you
  did not test at the artifact level is just a comment.
- **Do not let "it's compiled out" become the excuse to drop the runtime boot fail-fast.** Staging
  *does* compile dev routes in, and a staging binary pointed at prod data needs its own refusal. The
  packet says "belt and suspenders"; make it a named DoD item, not a footnote.

### 4. PII-minimization — customer JWT without phone, confirmed?

**Confirmed, and it is done well.** `issueCustomerToken` (`jwt.ts:126-131`) emits only
`{role, orderId, locationId, sub=customerId}`; the comment (117-125) states the principle explicitly
(the token is a *capability reference*, consumers look phone up server-side via `orderId`/`sub`).

Across **all three** claim variants there is no direct PII — every field is a UUID or an enum
(`OwnerClaims`, `CourierClaims`, `CustomerClaims` in the S2 YAML; courier `sub`=courierId, no
email/phone/name in the token even though the redeem *response body* carries `masked_email` +
plaintext `full_name`). The strict `.strict()` discriminated union is doing double duty as a
PII-creep guard — you cannot add a phone claim without breaking the old verifier during overlap
(Q9 freeze). Elegant: the strictness is a values-invariant, not just a correctness one.

One coupling to name (the "what nobody asked" adjacency): the token contains no PII but is a *key to*
PII — a stolen customer JWT keys to the track endpoint that reveals a stranger's address/contact
(A4). PII-min-in-the-claim is necessary but not sufficient while the token is XSS-exfiltrable
(AR-5). This is why §1's customer-surface concern and this charge are the same concern seen twice.

Condition (small, closes the class not the instance): the "no-phone" DoD test is a *blocklist* — the
next PII field (email? name?) would not be caught. Make the mint-side test a **positive allowlist**:
assert the minted token's claim keys are *exactly* the permitted set per role. `.strict()` enforces
this on verify; prove it on mint too.

### 5. Cutover strategy — "no migration, byte-compatible tokens" and live vendor sessions

**Strategically correct.** (b) drain/re-issue would force-relogin every live vendor *and* defeat the
seam (you could never prove parity if you nuked the sessions). (a) no-migration is the strangler
premise and I agree.

**The silent risk is not "migration goes wrong" — there is no migration. It is a parity gap that
only manifests on a live session mid-flight, on an *irreversible* write path.** Concretely: a vendor
holds a Node-minted refresh token; cutover flips to Rust; the vendor's next silent-refresh hits the
Rust `/auth/refresh`. If Rust's guarded-UPDATE atomicity, sha256 encoding, or the `<5s`
concurrent-window differs by a byte, a mis-implemented reuse-detection can **DELETE the family**
(T-1/T-2) — evicting a real working vendor from all devices, mid-shift, because of an implementation
seam. And here is the part the "rollback = route back to Node" line under-states: **routing back
does not un-delete a family row.** Rollback is true for *routing* and false for *state the bad Rust
path already mutated*. That asymmetry — reversible routing over an irreversible write — is the risk I
would regret in a year if it were gated only by a static hash-format test.

Condition (proportional to the irreversibility, see §C):
- The cutover parity gate must include a **cross-stack live-session** test, not just static hash
  parity: mint a family on Node → rotate on Rust → rotate the Rust-minted one back on Node → prove
  the happy path **and** that a benign concurrent-refresh across the stack boundary does *not* revoke
  the family.
- Flip **by canary**, not globally: route a small % of vendor traffic to Rust, watch the
  family-revocation rate (a spike in `OWNER_REVOKED` / reuse-detected = a parity bug evicting real
  users), widen only when it matches the Node baseline. Name the revocation-rate match as a cutover
  gate and name the irreversibility (family-DELETE is not rollback-recoverable) on the record.

### 6. ETHICAL-STOP / Charter check

**No ETHICAL-STOP.** I looked adversarially, not as a rubber stamp:
- **Military / warfare:** untouched. N/A.
- **Surveillance-for-harm:** auth authenticates; it does not surveil. The courier token keys to
  GPS/PII (A3) and binds to a live-revocable `courier_sessions` row — legitimate access control, but
  also the mechanism by which the *least-powerful actor* (the gig courier) can be de-authenticated
  mid-shift. That power asymmetry is real and worth naming, but it is not S2's to solve and not a
  Charter crossing. Flagged as an absent perspective (§7), not a stop.
- **Commons-capture / turned against the people it learned from:** standard access control; no
  capture, no weaponization.

**The one latent-harm path is already slated for death, and I reinforce that it must die.** Dead
`/api/auth/courier/activate` (AUTH-GAP-2) writes a courier refresh into the **owner** table, which
`/auth/refresh` would rotate as `role:'owner'` — a live-reachable privilege-escalation with **zero
users**. This is the single place where **carry-verbatim is the *unethical* choice**: you would be
preserving a latent escalation for no one's benefit. RETIRE (Q2) is not optional; make it a hard
port-blocker, and re-verify proof-of-deadness (0 FE callers, 0 E2E) *at port time* — "dead code"
claims rot.

**Affirming the good (part of this seat's job, not only fault-finding).** The surface already carries
counsel-shaped virtues: PII-minimization (dignity); immediate courier revocation *without*
per-request owner surveillance on the hot path (proportional, ADR-0004); existence-hiding 404s and
uniform anti-enumeration 202 (don't leak who exists); and `claim/decline` deliberately
**unauthenticated** so a restaurant can erase an unconsented shadow-preview *without being forced to
create an account* (counsel CC2 — genuine consent-respect). This is a packet that internalized the
ethics rather than bolting them on.

---

## Non-blocking aesthetic / strategic notes

- **Conceptual integrity — the three deferred rows are one pattern.** AR-4 (no reset), AR-5
  (localStorage), AR-6 (rotation-only kill) all share the failure mode *deferral becomes permanent by
  inattention*. Treat them as one register ("accepted-risk with owner + trigger"), not three
  scattered footnotes. A register that is reviewed is honest; a footnote is a place things go to be
  forgotten.
- **"Schema rich, runtime minimal" as restraint — well kept.** The token is a UUID-tuple capability
  reference; the strict union carries the invariants; the runtime does signature + exp + a confined
  re-read only on write surfaces (ADR-0004 keeps the hot path pure). This is the aesthetic doing its
  ethical job — a whole, minimal design has fewer seams to leak through.
- **The `#[cfg]` compile-out is the design-language high point.** Prefer "the capability is absent
  from the artifact" over "the capability is present but refused" wherever the port can — it is both
  the more elegant and the more ethical form. Let it set the pattern for other dev/test affordances.
- **The stale "1h" doc-comment** (`apiClient.ts:7-9` still says "short-lived access token (1h)"; code
  is 24h — Q-DOC-1H) is cosmetic but corrodes trust in the docs-as-truth. Fix it in the FE port pass;
  honest comments are part of a UI that "tells the truth."

---

## Steel-man of a rejected option (obligatory)

**Q5 option (b): redesign to httpOnly cookies *inside* S2, now — the option the packet rejects.**
Its strongest case, made fairly: the strangler flip is the *one* moment when the transport is already
being re-homed from Fastify to axum and the FE is already being re-homed to Astro — the exact window
where an FE-lockstep transport change is *cheapest*, because you are paying the FE-lockstep tax
anyway. Deferring httpOnly to "after Astro owns the FE" risks doing it during a period of *lower*
change-appetite, when a transport change looks gratuitous and gets perpetually re-deferred (the AR-5
"fast-follow that never follows" trap). And the asset most exposed — the customer 7d token on the
storefront — is the one *least* protected by the ≤24h owner tightening, so "we already bounded the
window" does not cover it. If you believe the register will not be honored, doing it now is the safer
bet on human behavior even if it is the riskier bet on the code.

**Why I still side with (a)+(c) over (b):** the epistemic argument wins. Moving transport *during*
the parity port destroys the parity oracle's discriminating power (a red spec would have two possible
causes), on a red-line surface, mid-strangler. The right answer is (a) at cutover + (c) as the *named,
owned, triggered* fast-follow — which neutralizes (b)'s strongest point (the "never follows" trap)
precisely by giving the row an owner and a trigger (see §C). (b) is not wrong on the destination; it
is wrong on the moment.

---

## Explicit conditions before code (§C)

Advisory — the human decides. These are counsel-weighted, ordered by ethical load:

1. **[port-blocker, reinforce Q2] RETIRE dead `/api/auth/courier/activate`.** Not optional —
   carry-verbatim here preserves a live-reachable privilege-escalation for zero users. Re-verify
   proof-of-deadness at port time.
2. **[cutover gate, charge 5] Match the gate to the irreversibility.** Cross-stack live-session
   parity test (Node↔Rust refresh rotation + benign-concurrent-window does-not-revoke) **and** a
   canary flip gated on family-revocation-rate matching the Node baseline. Record on the packet that
   family-DELETE is not rollback-recoverable.
3. **[dev-bypass, charge 3] Prove the compile-out; keep the runtime fail-fast.** DoD test that the
   *release* artifact 404s `/dev/*` and rejects a dev-kid token; port the boot fail-fast for staging
   binaries too (belt on top of the compiled-out suspenders).
4. **[PII, charge 4] Positive-allowlist mint test per role.** Assert minted claim keys are *exactly*
   the permitted set — close the class, not just the "no phone" instance.
5. **[deferral hygiene, charges 1+2] Owner + trigger on every accepted-risk row.** AR-4 (reset:
   trigger = first paid vendor onboarded on password-only auth ⇒ recovery path must exist), AR-5
   (localStorage: fast-follow target = **Q5(c) refresh-in-httpOnly**, not flat httpOnly), AR-6
   (rotation runbook must exist and be human-confirmed). One register, reviewed — not three footnotes.
6. **[humane, charge 2, cheap] Signpost the no-reset dead end.** The login page tells a locked-out
   password-only owner their real path (Google/Telegram/support) instead of a bare "wrong password."
   FE copy, no red-line surface — the port's job is to make the absence legible.

---

## The question nobody asked (§7)

Every actor in this surface *chose* to hold their credential — the owner logged in, the courier
redeemed an invite. Except one: **the customer never consented to holding a 7-day bearer token, in
`localStorage`, on the most script-exposed page in the system, that keys to their own home address
and phone.** They clicked a "track my order" link. The entire packet is written from the
platform-and-vendor security frame — the frame that asks "how do we protect the *tenant*." Nobody in
it speaks for the person whose address is the asset. The unasked question is not technical; it is a
consent question: *is it right that the least-consenting party holds the longest-lived,
most-exposed, most-personal credential in the system — and if the honest answer is "no," does that
not make the customer/track token, not the owner token, the true head of the httpOnly queue?*

That question does not block the port. It should sit on the AR-5 register with an owner, so that when
"fast-follow" is scheduled, the person who cannot attend the council is first in line.

---
*Advisory only. No ETHICAL-STOP. The human is final. Nothing here authorizes a code change.*
