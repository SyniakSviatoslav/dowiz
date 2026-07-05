# S3-CATALOG Port — Council Packet · COUNSEL OPINION

> Seat: **Counsel** (ethics · aesthetics · strategy) in the S3-catalog Triadic Council.
> Advisory, non-blocking. Architect asks "will it work"; Breaker asks "how it breaks";
> this seat asks *should it exist, is it fair/honest/dignified, is it whole, what is the long
> horizon, who bears the cost, and what did nobody ask.* Docs only — no code, no design authority.
> Every factual claim below was verified against live source, not trusted from the packet.

---

## OPINION: **PROCEED-WITH-REVISIONS**

**No ETHICAL-STOP.** I looked adversarially at the one place this surface could touch a human body
— the allergen gate — and it does not cross a grounded red-line: the port *carries the gate verbatim
and adds a machine guardrail*, the route is *unwired* (zero FE callers), and the residual food-safety
property is *pre-existing, not introduced by the rewrite*. An ETHICAL-STOP on a carried, unwired,
neutral-to-safety port would be verdict-not-friction — the exact overreach my mandate forbids. So the
friction here is Opinion, not a stop: five revision conditions (§C), ordered by ethical load. The
packet is disciplined, honest about its own deferrals, and — unusually — its central security finding
is *self-confirmed by the Rust code it critiques* (see below). My value is narrow and I keep it narrow.

**Verification note (charge: "verify the load-bearing claims").** I read the live source behind every
load-bearing assertion. They hold, and one holds harder than the packet claims:

- **The Q1 seam is real and the Rust code already admits it.** `rebuild/crates/api/src/db.rs:62`
  pins `SET_TENANT_STATEMENT = "SELECT set_config('app.current_tenant', $1, true)"`, binds a
  `TenantId` (a location uuid), and its own module doc (`db.rs:12-17`) states: *"The live schema
  actually uses two tenant GUCs… This helper implements `app.current_tenant` only, per this build's
  brief. Reconciling the two (or exposing a second `with_user` helper) is explicitly deferred."* The
  finding is not the Breaker catching a hidden bug — it is the packet honoring a debt the previous
  build *wrote down and left open*. That is the healthy shape; I affirm it.
- **The owner path really seats a different GUC.** `packages/platform/src/auth/tenant.ts:11` seats
  `app.user_id` from a `userId`; every S3 owner route calls `withTenant(db, userId, …)`
  (`locations.ts:48`, `menu-confirm.ts:18`, `menu-import.ts:253`). Two roots, two values, confirmed.
- **menu-confirm mutates exactly one column.** `menu-confirm.ts:20`:
  `UPDATE products SET allergens_confirmed = true WHERE id=$1 AND location_id=$2 RETURNING id`.
  The write-set is `{allergens_confirmed}`; `source` untouched; 0 rows → 404. Confirmed.
- **The C2 read-gate is narrower than the packet says — and that matters (charge 1).** The strip
  lives in *one* function, `read_preview_menu` (`migrations/…070_provision-products.ts:90-91`):
  `CASE WHEN p.source = 'place' AND p.allergens_confirmed = false THEN p.attributes - 'bom' ELSE …`,
  and that function *admits only shadow tenants* (`owner_id IS NULL AND status='closed' AND
  published_at IS NULL`, `:106-108`) — it "can never serve a real tenant." So the gate's storefront
  power is confined to the **pre-claim preview** of place-scraped rows. This sharpens where the
  guardrail must bite; it does **not** soften the harm (see charge 1).
- **The deferred public-OCR path is local-inference, not third-party egress (charge 3).** The
  `/anonymous` route (`menu-import.ts:173-228`) is public + rate-limited (`max:5, timeWindow:'1
  minute'`), ≤10MB, and stashes venue contact PII in Redis with a **30-min TTL** (`setex 1800`),
  **nothing written to the DB**. The parser is local: Tesseract.js OCR (`ai-ocr-parser.ts:7`) + an
  Ollama LLM (`memory.ts:3,35` — `provider:'ollama'`, `localhost:11434`), with a `PiiRedactor`
  (email/phone/card/iban/name, `pii-redactor.ts:10-40`) and a `menu-region` allowlist *before* the
  LLM. The "zero-PII-in-AI" red-line is satisfied **by architecture**, not by promise.

---

## By charge

### 1. Food-safety — is "write-set = exactly one column" enough friction for a life-safety column?

**For the port's job: yes, and the packet has it right — but prove the outcome, not the SQL string.**
The guardrail's real job is *provenance integrity*: keep the confirm route from broadening its write
beyond `{allergens_confirmed}` and silently touching `source`. Corrupting `source` away from `'place'`
defeats the gate's `source='place'` predicate and would leak AI-guessed allergens into the preview
(threat S3-T5). The write-set assertion is the correct, necessary guardrail and I affirm it.

**Sharpening (proportional to life-safety, not scope creep):** an assertion on the *emitted SQL text*
rots the moment someone refactors to a query builder — the exact drift the port introduces. For a
column that gates what an allergic customer sees, make the guardrail a **post-state behavioral
assertion under the NOBYPASSRLS probe**: after confirm, `allergens_confirmed` flipped to true **and
`source` is byte-identical to its pre-value**. Prove the *effect on the row*, not the intent of the
statement. This mirrors the S2 counsel move ("prove the compile-out at the artifact level, not just
intend it"). Cheap, and it closes the class rather than the instance.

**The deeper food-safety property the guardrail cannot reach — and must not pretend to (name it,
don't fix it in S3).** The confirm route flips `allergens_confirmed=true` **without checking that the
owner authored any allergens**. Combined with the comment at `menu-confirm.ts:9` ("bulk is a client
loop over this"), the live shape is: one owner "confirm all" pass converts *AI-uncertainty-stripped-
to-blank* into *owner-warranted "safe"* across a whole menu. A `source='place'` dish that once carried
an AI-guessed nut warning, stripped and never re-authored, renders after confirm as a dish with **no
allergen badge, marked owner-confirmed**. The customer with the allergy sees a blank and reads it as
cleared.

I state this carefully, because the design here is *considered, not careless*: migration 068's own
comment (`…068_acquisition.ts:20-21`) — "the pipeline NEVER asserts allergens as fact" — chose
**blank-over-guessed** deliberately, and a false "contains no nuts" is more dangerous than a blank.
That is an ethically-defensible tradeoff, not a bug. So this is **not** an S3 port-blocker and **not**
an ETHICAL-STOP: it is pre-existing, carried verbatim, and the route is unwired today. The port's job
is to make it **legible**, not fill it. Condition (§C-1): put "confirm-of-blank publishes owner-
warranted 'safe'" on the accepted-risk register with an owner and a trigger = **the moment the FE
wires confirm-allergens** — because that is the moment a one-click "confirm all" becomes reachable by
a time-pressured owner, and the moment the customer-facing question below (§7) goes live.

### 2. Cutover honesty — the double-write window and the "green becomes flip" trap

**The packet is honest about *rollback* but under-names two cutover facts.**

**(a) The irreversibility asymmetry — and why DEFER of menu-import is the best cutover-safety
decision in the packet, not just scope hygiene.** Almost every S3 catalog write is *reversible by the
owner re-editing* (last-writer-wins upserts on human-edited fields). There is exactly **one**
rollback-un-recoverable catalog operation: menu-import `mode='replace'` mass-DELETE
(`menu-import.ts:452-472`, guarded 409 on historical `order_items` at `:442-451`). Routing the proxy
back to Node does not un-delete rows the Rust path already destroyed — the same asymmetry the S2
counsel named for the refresh-family-DELETE. The packet defers menu-import for "heavy pipeline / wrong
crate" (true), but the *deeper* reason is stronger and should be on the record: **DEFER keeps the only
irreversible catalog write on the proven Node stack through the entire S3 cutover window.** That is
the single most cutover-protective choice in the packet. Affirm it, and name *why*.

**(b) The second gate the DoD does not name: the cutover flip is a separate human act.** §8 lists
"council sign-off + rollback plan (flag flip back to Node)" — good — but it does not distinguish
**build-approval** (this packet + operator 🔴 signs) from **the flip that makes Rust authoritative for
real tenant catalog writes**. Under Ship Discipline, "deploying dark to verify is fine; launching is a
separate, explicit act." The failure mode is *green-DoD → agent concludes "done" → flips the proxy as
part of shipping*, with no distinct operator go/no-go on the **cutover** as opposed to the **build**.
This is the "human-in-the-loop / zero-autoban" red-line applied to the strangler flip itself.
Condition (§C-2): name the cutover flip as a **second, explicit operator go/no-go**, distinct from
DoD-green and from packet-approval, and — because catalog is a human *edit session*, not a stateless
per-request path — flip the S3 namespace **atomically per-surface** (whole namespace to one stack,
fast-rollback), **not** a per-request traffic-% canary that could split a single owner's edit session
across two stacks and leave a customer looking at split-brain menu state. (This inverts the S2 canary
recommendation on purpose: auth was per-request and the risk was family-DELETE; catalog is per-session
and the risk is intra-session consistency.)

### 3. DEFER menu-import — does the deferral conserve a known PII risk without an owner?

**Yes, it conserves a risk — but a *well-mitigated* one, held steady on Node, and the honest fix is
legibility + a regression pin, not new code.** My initial worry was the grounded "zero-PII-in-AI"
red-line: a public unauthenticated endpoint where a user can upload *anything* (not just a menu) into
an OCR/LLM path. I chased it to source. The path is **local inference end to end** — Tesseract.js +
Ollama at `localhost:11434` (`memory.ts:3`), no cloud provider — with a `PiiRedactor` and a
`menu-region` allowlist *before* the model, a **30-min Redis TTL**, a **5/min/IP** rate-limit, and
**no DB write**. The red-line is satisfied by construction, and the extracted PII is venue self-
uploaded business contact (name/phone/address), not third-party or customer PII. The packet's
threat-model slightly over-dramatizes "PII intake" by omitting these four present controls; the true
residual is modest and DoS-shaped (compute-amplification abuse), not exfiltration-shaped.

**So the compensating control for the deferral window is not a new gate — it is preventing the
existing gates from silently rotting while attention is on Rust.** The real danger in a long deferral
is *drift*: someone bumps the TTL "to reduce re-uploads," disables the rate-limit "for a demo," or —
the one that would actually cross the red-line — swaps Ollama for a cloud model "to improve
extraction," turning a local pipeline into a third-party PII egress on a public endpoint. Condition
(§C-3), cheap and durable: a **regression pin** on the Node `/anonymous` route asserting (i) Redis TTL
≤ 30 min, (ii) rate-limit present, (iii) the ai-ocr provider stays local (Ollama/Tesseract, not a
cloud SDK) — plus one owner and a trigger = **"menu-import ports OR the launch trigger (first paid
order) fires, whichever first."** That converts "DEFER without an owner" (the real concern) into
"DEFER with an owner, a trigger, and a fence against regression."

### 4. Q1 `with_user` seam — strategically the right cut; graduate it from open-question to ADR

**This is the authoritative seam of the entire tenancy story, and its blast radius is every future
authenticated surface, not just S3.** Owner-root (`app.user_id`→memberships, ~34 sites) vs
courier/service-root (`app.current_tenant`, ~102 sites) is the load-bearing distinction S3, S5, S6,
S7 all inherit. Q1(a) — add `with_user(UserId)`, reserve `with_tenant(TenantId)` for later — is
correct, and the operator 🔴 sign-off is appropriate. Two strategic sharpenings:

- **The value is not the second function — it is un-confusable id types that make the trap
  un-compilable.** Q1(c) ("reuse `with_tenant`, pass the userId as a `TenantId`") is only *possible*
  if a `UserId → TenantId` conversion exists. The strongest structural guarantee is not a naming
  convention ("remember to call the right one") but **type segregation**: `with_user` accepts only
  `UserId`, `with_tenant` only `TenantId`, and the two are not interconvertible, so the wrong-family
  call is a *compile error*, not a post-flip 0-rows silent outage. This is the same principle the S2
  counsel praised in the `#[cfg]` compile-out — "the capability that cannot be misused is the one not
  present in the artifact." A naming convention is a comment; a type is a guarantee.
- **Settle it once, at ADR altitude — do not re-litigate per surface.** Because S4–S7 all inherit
  this ruling, it should graduate from a packet open-question into a durable **ADR** (the tenancy-GUC
  contract), so the monotonic-ratchet holds and the next surface *inherits* rather than *re-decides*.
  A ruling that lives only in an S3 packet is a footnote the S6 courier port will re-argue.

### 5. Scope, ordering, Charter

- **Scope is restrained, not bloated.** Four 🔴 concerns + a disciplined non-🔴 CRUD lane, menu-import
  deferred, no error-envelope normalization, no schema change, no order-total (correctly S5). The one
  arguable inclusion — porting the *unwired* menu-confirm (Q4) — is cheap insurance on a safety
  column: machine-enforce the invariant *before* the FE wires it, so the guardrail is already locked
  when the "confirm allergens" button is built. That is prudence, not creep. Affirm Q4(a).
- **Ordering is coherent.** S3 stores pricing inputs; S5 computes totals; the value flows through the
  frozen DB schema (the SSOT), not through stack-coupling — so Rust-writes-fees / Node-computes-totals
  during the interleave window is safe, and the packet names the right cross-surface assertion ("a
  fee/tax edit changes the next order total via the S5 path, not S3"). menu-import correctly waits for
  the S4 media stack.
- **Charter: clean.** Catalog CRUD. No military/warfare, no surveillance-for-harm, no commons-capture,
  no weaponization. Local inference keeps the AI path off third-party egress. The only human-wellbeing
  vector is the allergen care-lens concern (charge 1), which is carried-not-introduced and does not
  cross a grounded line. **No ETHICAL-STOP. No Charter violation the port introduces.**

---

## Non-blocking aesthetic / strategic notes

- **Conceptual integrity — "one way to touch a tenant table" is the whole aesthetic, and it is the
  ethics.** The packet's insistence that the raw pool stay unreachable (`Pools` `pub(crate)`) and that
  `with_user`/`with_tenant` be the *only* sanctioned paths is the design-language high point: a whole,
  minimal seam has fewer places to leak a tenant boundary. Push it one notch further than the packet
  does — a *single* GUC-seating choke point (see the steel-man) makes "there is one way" a compiler-
  checked fact rather than a two-function convention someone can route around with a third ad-hoc
  `set_config`.
- **"Schema rich, runtime minimal" as restraint — well kept.** The catalog write is a single-statement
  `with_user` txn; the invariants live in the frozen schema + RLS + the explicit `WHERE location_id`
  belt; the runtime is a thin, confined seat-then-write. The one long-held txn (menu-import commit) is
  correctly *excised* into its own future slice. This is the aesthetic doing its ethical job.
- **The `read_preview_menu` gate is elegant and honest** — the strip is co-located with the data it
  guards (`source='place'` rows only ever render there), it is `SECURITY DEFINER` with a pinned
  `search_path`, and it *cannot* serve a real tenant by construction. When the FE finally wires
  confirm, keep the confirm UI as honest as this SQL is: a confirm of a blank allergen set should
  *look* like what it is (publishing "no allergens, owner-verified"), not like a formality.
- **The nested-`BEGIN` quirk (`menu-import.ts:253-254,509`) is the right thing to refuse, not carry.**
  Deferring menu-import lets that structural rewrite get its own council instead of riding the first-
  writes surface — the packet's instinct is correct and aesthetically clean (don't smuggle a txn-model
  change into a tenancy-seam change).

---

## Steel-man of a rejected option (obligatory)

**Q1 option (b): one `with_ctx(GucCtx::User | GucCtx::Tenant, …)` combinator — the option the packet
leans away from in favor of (a) two functions.**

Its strongest case, made fairly: (b) is the *more conceptually-integral* design, and by exactly the
standard the packet itself invokes. Two separate functions make "there are exactly two tenancy roots"
**tribal knowledge** — you must know both exist and when each applies, and nothing stops a future dev
from adding a *third* ad-hoc `set_config` path that touches neither function, because there is no
single place that owns GUC-seating. A single `with_ctx` whose argument is a compiler-checked enum
makes the two-roots fact **encoded in the type**: every call site must name its root, a new root (if
S8 ever needs one) becomes a new variant the compiler flags at *every* match site, and — decisively —
there is **one auditable choke point** where every tenancy GUC is seated. That is the "one way to
touch a tenant table" virtue the packet champions, delivered *more completely* than (a) delivers it:
(a) gives you two named doors; (b) gives you one door with a labeled dial and no way to build a third
door without the compiler noticing.

**Why I still land where the packet does — with a condition.** (a) and (b) are both acceptable; the
packet is right that (a) is simpler and ships faster on a port where simplicity is a virtue. But the
*load-bearing* property is neither "two functions" nor "one function" — it is **type-segregated,
un-confusable id types that make Q1(c) a compile error** (charge 4). (a) achieves that only if `UserId`
and `TenantId` are genuinely non-interconvertible newtypes; (b) achieves it *and* centralizes the
audit surface. So: choose (a) for speed, but adopt (b)'s *guarantee* — no `UserId→TenantId` footgun,
and treat the GUC-seating sites as an auditable set even if they are two functions. (b) is not wrong
on the destination; it is a heavier lift than a port wants — which is the only reason it loses.

---

## Explicit conditions before code (§C)

Advisory — the human decides. Counsel-weighted, ordered by ethical load:

1. **[food-safety, charge 1] Prove the outcome, not the SQL string; register the confirm-of-blank
   property.** The menu-confirm guardrail is a **post-state** assertion under the NOBYPASSRLS probe:
   after confirm, `allergens_confirmed=true` **and `source` byte-identical**. Separately, put
   "confirm-of-blank publishes owner-warranted 'safe'" on the accepted-risk register with an owner and
   trigger = **FE wires confirm-allergens**. Port dark now (Q4a); make the human property legible for
   when it goes live.
2. **[cutover, charge 2] Name the second gate and the irreversibility.** The proxy flip that makes
   Rust authoritative for real tenant catalog writes is a **separate, explicit operator go/no-go**,
   distinct from DoD-green and packet-approval. Flip the S3 namespace **atomically per-surface** (not a
   per-request canary that splits an edit session). Record on the packet that (a) menu-import's
   `replace` mass-DELETE is the one rollback-un-recoverable catalog write, and (b) deferring it keeps
   that write on proven Node through the whole cutover — the DEFER is a *safety* decision, name it so.
3. **[deferral hygiene, charge 3] Pin the anonymous-import controls against drift; owner + trigger.**
   A regression pin on the Node `/anonymous` route: Redis TTL ≤ 30 min, rate-limit present, ai-ocr
   provider stays **local** (Ollama/Tesseract, never a cloud SDK — that swap would cross zero-PII-in-
   AI). Owner + trigger = "menu-import ports OR first paid order, whichever first."
4. **[tenancy seam, charge 4] Un-confusable id types; graduate Q1 to an ADR.** Whichever of Q1(a)/(b)
   ships, the requirement is that the wrong-GUC-family call (Q1c) is a **compile error** —
   `UserId` and `TenantId` non-interconvertible. Settle the ruling as a durable ADR (the tenancy-GUC
   contract) so S4–S7 inherit it rather than re-litigate.
5. **[deferral hygiene, charges 1+3] One register, reviewed — not scattered footnotes.** The confirm-
   of-blank property (§C-1) and the anonymous-import controls (§C-3) share the failure mode *deferral
   becomes permanent by inattention*. Treat them as one reviewed accepted-risk register with owners
   and triggers, same posture as the S2 AR-register. A register that is reviewed is honest; a footnote
   is where things go to be forgotten.

---

## The question nobody asked (§7)

The entire packet is written from the **platform-and-tenant** frame: the `source` provenance column is
a *liability audit* — it protects the **platform** by recording that the human owner, not the AI, is
the author of record for what the storefront claims. That frame is correct and well-built. But it
answers "who is *responsible* if an allergen claim is wrong," not "who is *harmed*." Those are
different people.

Nobody in this surface speaks for the **customer with the allergy**. When the FE finally wires
confirm-allergens, one time-pressured owner clicking "confirm all" (the bulk client-loop the code
already anticipates, `menu-confirm.ts:9`) converts a menu's worth of *AI-uncertainty-stripped-to-
blank* into *owner-warranted "safe"* — and the customer reads a blank allergen field as "cleared,"
not as "never authored." The provenance column will faithfully record that the owner did this. It will
not have made the owner *look*. The unasked question is not technical and it does not block the port:
*the system carefully protects the platform from the liability of a wrong allergen claim — what, if
anything, protects the customer from a careless bulk-confirm of an empty one?* The blank-over-guessed
tradeoff (068's "never assert allergens as fact") is the right call on the *system's* side of the
line; the open question is whether the *human* side — the confirm UX, when it exists — will make the
owner understand that "confirm" is a warranty, not a checkbox.

That question sits on the §C-1 register with an owner, so that when confirm-allergens is wired, the
person who cannot attend this council — the one whose body is the asset — is not discovered last.

---
*Advisory only. No ETHICAL-STOP. The human is final. Nothing here authorizes a code change.*
