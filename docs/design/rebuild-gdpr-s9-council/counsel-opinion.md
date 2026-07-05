# S9-GDPR/COMPLIANCE Port — Council Packet · COUNSEL OPINION

> Seat: **Counsel** (ethics · aesthetics · strategy) in the S9-GDPR/compliance Triadic Council.
> Advisory, non-blocking. Architect asks "will it work"; Breaker asks "how it breaks";
> this seat asks *should it exist, is it fair/honest/dignified, is it whole, what is the long
> horizon, who bears the cost, and what did nobody ask.* Docs only — no code, no design authority.
> Every load-bearing claim below was verified against live source, not trusted from the packet.

---

## OPINION: **PROCEED-WITH-REVISIONS** · **one scoped ETHICAL-STOP (friction, not veto)**

This is the reddest surface in the rebuild — the one place a defect is not money or availability but a
**broken promise to a real person that their data is gone**. I looked adversarially at every place S9
crosses a grounded line and touches a data subject: the customer who filed an Art.17 request and was told
"done," the person whose id could drive a cross-tenant erasure, the retention knob that silently keeps
PII for years, the backup that resurrects the erased. On four of these the packet's own disposition holds
the line, or fixes it — so those are Opinion, not stop. On **one**, the line is crossed **on the live
system today**, I verified it in the anonymizer's own bytes, and the packet's recommended disposition
(FIX-IN-PORT) leaves it crossed **until the 9th and last strangler surface ports**. That earns **one
scoped ETHICAL-STOP**, and it is deliberately narrow.

**The STOP is not "do not port erasure."** The erasure engine is honest, hard-won, and the packet ports
it with discipline (the #61 fail-loud backstop, the fail-closed scope, the masked-404 IDOR guard, the
DEFINER pin). Stopping a disciplined port would be verdict-not-friction, the overreach my mandate forbids.
**The STOP is on one property: an Art.17 erasure does not reach the subject's *orders* at all** — their
home address, precise home GPS, doorway photo, receiver name, and free-text feedback survive the "erase
me" request, while the system records `completed` and fires `gdpr.erasure_completed` to the dashboard.
This intersects **`анонімізувати-не-видаляти`** (the erasure obligation) and **`сервер-авторитетний` /
`UI каже правду`** (a `completed` that is not complete) — verified below in the code, not taken from the
packet. The STOP is friction: it lifts the moment the operator records **any one** of three minimal
conditions (§STOP), and its cheapest lift *fixes the live system now*. The human is final.

**Why a STOP here, when S5/S8 (also FIX-IN-PORT deferrals) got conditions, not stops.** The distinguisher
is not "is it serious" — S8's fail-open webhook was serious. It is *which* line, and *whose disposition
leaves it open*. S8's fail-open is a **security** defect (Breaker's domain, not an enumerated
counsel red-line), so it earned a strong condition ("fix on Node now"). GAP-A crosses an **enumerated
grounded red-line** (`анонімізувати-не-видаляти`) — the *exact* class the **S4** seat stopped on (the
doorway photo escaping erasure), and the packet's disposition (fix-in-port) leaves that grounded crossing
**open on the live system**, not closed. Same trigger as S4, transposed onto today's stack. This tracks
**S4**, not S5/S8. The packet itself pre-designates GAP-A as "the ETHICAL-STOP most likely" and asks
counsel to rule; ruling "fix-in-port suffices" would be the wrong answer to the question the packet asks.

**The three findings my verification sharpened (load-bearing, in two cases worse than the packet states):**

1. **GAP-B is worse than GAP-A in the long horizon, and the packet under-ranks it.** GAP-A (address /
   doorway photo) is time-**bounded** — the *retention* sweep purges those carriers by age, so on the
   live system they survive an Art.17 request "only" up to `retention_days` (default 365). **GAP-B
   (`orders.delivery_lat/lng`) and GAP-C (`order_ratings.feedback`) are purged by *neither* path** — a
   person's **precise home GPS coordinate** and their **free-text words** survive an erasure request
   **indefinitely**, forever, on an "anonymized" row. The vivid headline is GAP-A (home + face); the
   *durable* harm is GAP-B (GPS, unbounded). Both are in the STOP; GAP-B is the one that never self-heals.
2. **The #61 fail-loud backstop — the packet's proof that "silent non-erasure is structurally
   impossible" — is scoped to the customer row and structurally *cannot* catch GAP-A.** Verified: the
   backstop confirms `SELECT anonymized_at FROM customers WHERE id=$1` (`anonymizer-gdpr.ts:74-78`). The
   customer row *is* anonymized; the backstop goes **green**; `completed` is written — while the orders
   survive. So the very gate that makes N1 impossible **greenlights** a GAP-A-incomplete erasure. The
   fan-out fix must **extend the completion gate to the orders**, or the port ships a `completed` that is
   still partially false even after the fan-out lands.
3. **The Q5 "silent default to max" worry is *mitigated* by the actual default, and I say so plainly.**
   The task and the packet both flag a 7-year retention silently defaulting to the 2555 max. Verified: the
   read defaults to **365**, not the max (`gdpr.ts:268`, `retention_days ?? 365`). So the platform does
   **not** silently retain for 7 years; the residual is narrower and lawful-shaped (an owner *can* set
   2555 with no basis captured — the controller's discretion, §Q5). Right-sizing the fear is part of the
   job; Q5 is a condition, not a stop.

Everything else is Opinion or affirmation. The IDOR/masking guard, the DEFINER pin, the restore-drill
polarity, and the human-gated flip are correct and I affirm them without re-litigating the Breaker.

---

## Verification note (I read the live source behind every load-bearing claim)

- **GAP-A — the GDPR path never touches orders. Confirmed at the branch.** `anonymizer-gdpr.ts:62-65`
  calls `anonymize({scope:'gdpr', subject:{customerId, locationId}})` — **no `orderId`**. In the engine,
  that enters **only** `if (options.subject?.customerId)` → `anonymizeCustomer` (`index.ts:83-88`); the
  `orderId` branch (`:90-95`) is never entered, and the order fan-out lives **only** in the *retention*
  branch (`:97-113`, `scope==='retention'`, by age). So `anonymizeOrder` — and the #74 doorway-photo
  purge it now carries (`:260-267`) — is **unreachable from GDPR**. An Art.17 erasure anonymizes the
  `customers` row and nothing else. Confirmed.
- **GAP-B — `delivery_lat/lng` is in no null-set. Confirmed.** `anonymizeOrder`'s UPDATE (`index.ts:240-253`)
  nulls `client_ip_hash, delivery_address, delivery_instructions, customer_messenger_handle, receiver_name,
  receiver_handle, receiver_messenger_kind, delivery_photo_key` — and **not** `delivery_lat`/`delivery_lng`.
  Neither GDPR (never reaches orders) nor retention (not in the set) nulls them. The **data-map itself
  documents the intent** the code fails: row #4 says delivery_lat/lng anonymises on `retention_days →
  anonymized_at NULLs`. The code does not fulfil its own PII inventory. Confirmed.
- **GAP-C — `order_ratings` is touched by nothing.** No reference to `order_ratings` anywhere in the
  anonymizer service. `feedback` (self-identifying free-text) + `customer_id` survive both paths,
  indefinitely (data-map #8). Confirmed.
- **The fan-out fix is feasible on the live stack — the linkage and the columns exist.**
  `1780310074262_orders.ts:24` — `customer_id uuid REFERENCES customers(id)` (the subject's orders are
  selectable by `customer_id = $1 AND location_id = $2`); `:30-31` — `delivery_lat double precision`,
  `delivery_lng double precision` **exist** (so adding them to the null-set is a safe UPDATE, not a
  "gap-for-a-500" trade the packet feared). Fix-on-Node-now is viable; it is **not** port-only.
- **The live system is under-erasing *today*, under BYPASSRLS.** Pre-NOBYPASSRLS, the context-free
  `anonymizeOrder` *works* (retention purges orders today), so the doorway photo is purged up to a year
  *late* for a GDPR request, and GAP-B/C never at all — right now, on the stack serving real people. The
  N1 silent-no-op is a *future* (post-MIG-2) amplifier of an *already-live* under-erasure; GAP-A/B/C do
  not wait for the flip. Confirmed against `index.ts:220` (context-free `pool.connect()`, no `set_config`).
- **Q5 default is 365, not max.** `gdpr.ts:268` — `res.rows[0].retention_days ?? 365`; PUT accepts the
  Zod-bounded value and UPDATEs with **no basis field** (`:281-286`). Confirmed: not silently-max; no
  basis captured.
- **The IDOR/masking claims hold as the packet states** — the same-tenant proof + masked-404 +
  `cross_tenant_attempt` log, the required (never self-derived) scope, and the `maskName` status reads are
  the correct posture; this is Breaker's robustness domain and I affirm without re-deriving it.

---

## By charge

### 1. Q1 / GAP-A·B·C — the erasure that does not erase (the STOP)

**Disposition: FIX (fan the erasure out to the subject's order-graph) — and the fix must reach the *live
system*, not only the port. This is §STOP.** Weighed across lenses (plural, no single codex):

- **The erasure obligation (`анонімізувати-не-видаляти`).** The line presumes erasure *happens*. For the
  subject's orders it does **not happen** at Art.17 time — it happens up to a year later, by a mechanism
  (retention-by-age) that is not tied to the request, for *some* carriers, and **never** for GPS/feedback.
  A right exercised and not honoured is the line crossed, not bent.
- **Honesty (`сервер-авторитетний` / `UI каже правду`).** The worker writes `completed` and publishes
  `gdpr.erasure_completed`. That record is **true for the customer row and false for the erasure**. The
  #61 backstop cannot see the difference (finding #2 above). A compliance system whose *completion record*
  overstates what was erased is dishonest in exactly the dimension the surface exists to be honest about.
- **Care / who bears it.** The data subject bears 100% of the residual, and it is the most sensitive
  residual the system holds — a **photograph of their front door** and their **precise home coordinates**.
  No one gains from the survival; it is pure entropy, the clearest injustice signature (the same shape the
  S4 seat named for the doorway photo — this is that finding, one surface upstream, on the erasure *request
  path* itself).
- **Long horizon.** GAP-B/C never self-heal. An "anonymized" order row is kept forever; its lat/lng and
  feedback ride along forever. Over indefinite time this is exactly the surveillance-*shaped* corpus that
  forms by neglect, not intent — a set of precise home coordinates for people who explicitly asked to be
  forgotten. The Charter's dignity/commons spirit points the same way as the GDPR line: bound it so the
  corpus never forms.

**The fix (packet Q1a1, correct) and the one addition.** Fan the GDPR erasure out to the subject's orders
(`SELECT id FROM orders WHERE customer_id=$1 AND location_id=$2` → `anonymizeOrder` each, reaching the #74
photo purge), add `delivery_lat=NULL, delivery_lng=NULL` to the null-set (columns confirmed), and null/
re-key `order_ratings.feedback`/`customer_id` for the subject. **Addition (finding #2):** extend the
completion gate — `completed` requires the orders anonymized too, not just `customers.anonymized_at` — else
the port re-ships a partially-false `completed`. On the enforcement seat: under NOBYPASSRLS the fan-out
seats `app.current_tenant=locationId` (the `orders` RC4 arm already exists — no new arm), so the app-side
seat is preferred over a new DEFINER (Q4b), keeping the DEFINER surface minimal. **This is the §STOP.**

### 2. Q2 — the cross-tenant erasure IDOR + status masking — affirm, no stop

**Affirm (a): carry the masked-404 + `cross_tenant_attempt` log + required scope + status masking; port
the scope as a non-constructible-without-`TenantId` type.** This is the correct posture and the packet
carries it verbatim (ledger #57). A cross-tenant *irreversible* erasure is the worst class, and it is
closed by construction: the client `customerId` is proven same-tenant before it can drive the erase, the
classification (nonexistent vs cross-tenant) stays server-side, and the status reads mask the subject id.
The one thing I sharpen for the record (not a stop, a guardrail): the request row stores `subject_phone`
in **plaintext** (data-map #13) — the erasure request is itself a PII carrier; the guardrail must assert
no un-masked `customer_id`/phone/`subject_phone` in **any** gdpr-requests response, and the port must not
widen its exposure (logging it, an admin read). This is Breaker's domain; I affirm and leave it there.

### 3. Q3 — irreversibility + restore-resurrection — affirm the packet's (a)+(a1)

**Affirm: the erasure proof is the data-level re-read (`anonymized_at IS NOT NULL` under NOBYPASSRLS +
the negative `failed`/DLQ), never a restore false-green; and name restore-resurrection as a bounded, owned
accepted-risk + a re-erase-on-restore runbook.** The polarity point is exactly right and worth affirming
out loud: the restore-drill proves **fidelity** (rows come *back*); erasure needs the **opposite** (rows
*stay gone*) — conflating them (repurposing the restore-drill as an erasure oracle) is the P5 false-green
the RESOLVE-R2 rejected, and the port must keep them opposite. On resurrection: a pre-erasure encrypted
backup *does* contain the erased PII (Option A keeps full-PII dumps — correctly, since a faithful restore
must contain PII). That is a **genuine compliance property, not a bug**, and it is defensible **only** as
an explicit position: bounded backup window + R2 lifecycle expiry (erasure is durable in the live DB
immediately; the backup window is finite) **+** the restore runbook re-applies every `status='completed'`
erasure whose `completed_at` precedes the backup. This is `reversibility named honestly` — the Charter's
spirit realised as a runbook, not a silent hole. No stop; the packet's own disposition owns it. **One
sharpening:** the runbook's re-erase pass must fan out the *same* order-graph the STOP fix adds — else a
restore un-erases the very carriers GAP-A/B/C left behind. Tie the runbook to the fixed erasure, not the
old customer-only one.

### 4. Q5 — retention legal basis — condition, not stop (the fear is right-sized)

**Affirm (a) + fix GAP-B/C in the shared null-set; do not treat this as a stop.** Retention is the
**controller's** (owner's) lawful decision under their own basis; DeliveryOS is the processor (data-map
#1/#4/#13). A platform stopping an owner from setting a retention their jurisdiction permits would be
overreach. The verified facts right-size the flag: the default is **365** (not the max, `gdpr.ts:268`), so
the platform does **not** silently hoard PII for 7 years. The genuine residuals are narrower and both are
conditions: **(i)** an owner *can* set 2555 with **no basis captured** — record the accepted-risk + a DPA
clause obligating the controller to document a basis, and (ii) the UI must **not present the maximum as
neutral** (no dark-pattern nudge toward "keep everything"; the honest default and honest copy are the
`UI каже правду` line applied to retention). **(iii)** the retention sweep must not *silently* retain the
carriers the data-map marks HIGH-RISK — fixing GAP-B/C in the shared null-set means an aged-out order
loses its GPS + feedback too, not just its address. This is the owner-data-export precedent's spirit (make
the PII decision **explicit and owned**, never silent) without the export precedent's stop — because here
the decision is lawfully the controller's, and the platform's duty is legibility, not veto.

### 5. Q6 — the flip with no cleanup — affirm the human go/no-go, and pin the pre-flip P-proof to the fix

**Affirm (a): prove correctness under NOBYPASSRLS *before* the flip; the flip is a separate, explicit
operator go/no-go, alongside S5; the Rust-S9 create writes the shared `gdpr_erasure_requests` row
(worker-recoverable if the cross-stack enqueue drops); adopt the claim-before-work CAS over the
COMMIT-then-mark TOCTOU.** This is the `людина-в-петлі / нуль-автобану` line applied to the strangler flip
itself — non-negotiable on the one surface with no rollback for its defining act, and I affirm it exactly
as the S5/S8 seats named the flip as a second human act. **One sharpening tied to the STOP:** the pre-flip
N1 data-level P-proof must assert the **whole subject-graph** erased (customer *and* orders *and*
order_ratings), not just `customers.anonymized_at` — because the fan-out (finding #2) changes what
"correct erasure" means, and a P-proof scoped to the old customer-only shape would go green on a
GAP-A-incomplete port. Prove the *fixed* contract before the flip, not the *old* one.

### 6. Scope + Charter

- **Scope: disciplined.** The runtime (S8), the money folds (S5/S7), the backup pipeline (DR council), the
  CRM/reveal PII surfaces (path-owned elsewhere) are correctly excluded; draft 088 is a DEFINER the port
  *calls*, not authors; no schema change. The one FIX-IN-PORT (GAP-A/B/C) is the *honest* direction — the
  one I insist reaches the live stack. This is a clean boundary.
- **Charter: clean, with the long-horizon note that *is* the STOP.** No military/warfare, no
  surveillance-for-harm-by-intent, no commons-capture. But the survival of erased subjects' precise home
  GPS (GAP-B, indefinite) is the **surveillance-shaped asset that accretes by neglect** — nobody decides
  to build a corpus of forgotten people's coordinates; it forms because the delete was never wired. The
  Charter says never build that on purpose; the honest corollary is *do not build it by accident either.*
  Fixing GAP-B is the Charter's dignity/commons spirit realised in the null-set.

---

## Non-blocking aesthetic / strategic notes

- **The #61 backstop is aesthetics-as-leading-indicator running in reverse — the S9 twin of the S5/S8
  Potemkin.** It *looks* like a complete erasure proof (a data-level re-read, a fail-loud DLQ signal, a
  "structurally impossible" claim) — and it is genuinely elegant for the customer row. But it asserts one
  table while the erasure spans three, so its shape earns a trust its scope does not back. A beautiful
  partial proof is more dangerous than an obviously-absent one, because its polish is why no one re-checks
  it. Make the shape match the guarantee: the completion gate asserts the whole subject-graph, so the proof
  you can *see* is the erasure that *holds*.
- **The avatar-vs-doorway asymmetry the S4 seat named is *still here*, one surface up.** The GDPR path
  *does* delete the customer's chosen `avatar_key` object (`index.ts:161-177`) but never reaches the
  unconsented-by-passersby doorway photo — the **less** sensitive asset treated **more** carefully than
  the **more** sensitive one. The fan-out restores the symmetry: a whole design treats its most sensitive
  asset most carefully, and this one still has it backwards until GAP-A lands.
- **"Схема багата, рантайм мінімальний" is honoured well here — affirm the restraint.** Erasure runs on
  the S8 fleet's worker (no new runtime), the DEFINER does the visibility-independent erase, the carrier
  null-set is plain SQL, the global-singleton is flagged-for-scale-not-built. The engineering risk is
  correctly named as *completeness + isolation + irreversibility + basis*, not throughput. The rare good
  kind of rewrite: the port makes the erasure *more* complete (the fan-out) while the runtime stays
  minimal. Keep it that way — do not let a new `customers` policy arm (rejected, N2) or a companion DEFINER
  (avoidable, the `orders` arm exists) pull weight back onto the primary PII table.

---

## Steel-man of a rejected option (obligatory)

**Q1 option (a2) — "ACCEPT-RISK: retain the orders as transaction records under LI; erase the delivery-PII
on the retention TTL" — the option the packet rejects and I land against.**

Its strongest case, made fairly: **the right to erasure is not absolute.** GDPR Art.17(3) exempts
processing necessary for compliance with a legal obligation and for the establishment/exercise/defence of
legal claims. An order is a **transaction record**; tax, invoicing, and bookkeeping law in most
jurisdictions *require* retaining the substance of a sale for years — the amounts, the tax, `cash_pay_with`.
Under that reading, forcing a full delete of an order on demand would breach the *owner's* statutory
bookkeeping duty, and "keep the record, scrub the direct identifiers on the normal retention schedule" is a
recognised, defensible controller posture — not a compliance failure. The packet's own ACCEPT-RISK rows for
the pseudonymised counters and `cash_pay_with` concede exactly this. So (a2) is *right* that some order data
lawfully survives an Art.17 request, and a port that force-deleted the financial record would be *wrong*.

**Why I still land on fan-out.** (a2) is correct about the **financial/transaction fields** and I *adopt*
that — those survive under statutory basis, as an explicit, owned accepted-risk (not silence). Where (a2)
overreaches is sweeping the **home address, precise GPS, doorway photo, and free-text feedback** into
"transaction record." No tax statute requires retaining a **photograph of a front door** or a **GPS
coordinate of a home** for seven years — those are delivery-logistics PII, not bookkeeping. And even for the
carriers (a2) *would* eventually erase, it defers them to the age-based TTL (up to 365 days), whereas Art.17
requires erasure "**without undue delay**" — and the fan-out is a bounded, cheap, already-half-built change,
so "undue delay" has no cost justification here. So (a2) wins on the *financial* fields (adopted into the
accepted-risk rows) and loses on the *delivery-PII* fields (fan them out now). The honest split (retain the
statutory record; erase the delivery-PII at request time) is what the fan-out achieves and (a2)-as-framed
blurs. (a2) is right about the *destination for some data* and wrong about the *schedule for the sensitive
data* — the mirror of the S5 promotions call: adopt its legal discipline, reject its silence.

---

## The scoped ETHICAL-STOP (grounded line + why + minimal lifting conditions) — §STOP

**Grounded red-lines:** *`анонімізувати-не-видаляти`* (the erasure obligation — erasure must *happen*) and
*`сервер-авторитетний` / `UI каже правду`* (a `completed` record must be true). **Verified intersection,
in the anonymizer's own bytes, on the live stack:** an Art.17 erasure of a customer
(`anonymizer-gdpr.ts:62-65` → `index.ts:83-88`, `anonymizeCustomer` only) **never reaches the subject's
orders** — their **home address, precise home GPS (`delivery_lat/lng`, survives *indefinitely* — GAP-B),
doorway photo, receiver name, and free-text feedback (`order_ratings.feedback`, indefinite — GAP-C)**
survive the "erase me" request (GAP-A carriers up to `retention_days`; GAP-B/C forever), while the worker
writes `completed` and publishes `gdpr.erasure_completed`, and the #61 backstop — scoped to
`customers.anonymized_at` (`:74-78`) — **cannot see the difference**. This is a *verified intersection*
with two grounded lines, not taste and not the packet's word.

**Why a STOP and not an Opinion (and why S9 differs from S5/S8, where I did not stop).** S5/S8's
FIX-IN-PORT deferrals were on lines *outside* this seat's grounded set (a promotions honesty gap; a
security fail-open — Breaker's domain), and in S8's case the packet's disposition *fixed* the line. Here the
line is **enumerated and grounded** (`анонімізувати-не-видаляти` — the *exact* class the S4 seat stopped
on), and the packet's recommended disposition (fix-in-port) leaves the grounded crossing **open on the live
system** — real people filing Art.17 requests *today* are told "done" while their front door and home
coordinates survive, and the fix is deferred to the 9th and last strangler surface. A ruling that
fix-in-port suffices would be wrong (I verified the live gap it leaves open); scoped friction is the honest
answer to the question the packet explicitly asks. This tracks **S4**, not S5/S8.

**This STOP is friction, not veto.** It does not block S9 or the erasure port. It pins **one property** —
the erasure not reaching the subject's order-graph — to a recorded human decision, and it **lifts the
moment the operator records any ONE** of:

1. **Fix-on-Node-now, standalone (preferred, cheapest, the S8 shape).** Land the order-graph fan-out on the
   live Node stack as its own red→green commit — after `anonymizeCustomer`, select the subject's orders
   (`customer_id=$1 AND location_id=$2`, confirmed FK) and `anonymizeOrder` each (reaching the #74 photo
   purge), add `delivery_lat/lng=NULL` to the null-set (columns confirmed), null/re-key
   `order_ratings.feedback`, and extend the completion gate to require the orders anonymized. This works
   *today* under BYPASSRLS, closes the live crossing for real Art.17 requesters **now**, keeps the parity
   oracle clean, and the port carries already-fixed behaviour under the seated `orders` RC4 arm. *(App-side;
   no migration; no schema change.)*
2. **Fix-in-port + an explicit interim accepted-risk with an "undue delay" position + a near-term trigger.**
   If the operator keeps the fix scoped to the port, the *interim* — the live system under-erasing until S9
   ports — must be a **reviewed, owned accepted-risk row**, not a silent fix-in-port footnote: named owner,
   counsel's "without undue delay" legal note, and a trigger (`= "before the launch trigger — first real
   paid order — OR S9 ports, whichever first"`).
3. **Recorded accepted-risk (the explicit human override the mandate protects).** The operator records — on
   a reviewed register, named owner, trigger — explicit acceptance that *"an Art.17 erasure leaves the
   subject's home address + doorway photo live up to `retention_days`, and their precise home GPS + free-text
   feedback live indefinitely, while the system records `completed`."* A risk consciously accepted, owned,
   and triggered is honest; the same risk as an unread footnote is where it goes to be forgotten. The human
   is final.

My preference order is **1 > 2 > 3** (fix the live system > own the interim explicitly > name the escape).
But 2 and 3 are valid conscious human choices and I will not override them — that is the whole point of
friction-not-verdict.

---

## Explicit conditions before code (§C)

Advisory — the human decides. Counsel-weighted, ordered by ethical/epistemic load.

1. **[the STOP, §1] Bring the subject's order-graph inside the erasure, or record its escape.** Satisfy
   §STOP condition 1, 2, or 3 before the Rust S9 erasure becomes the authoritative cutover path. Preferred:
   the fan-out (GAP-A) + `delivery_lat/lng` (GAP-B) + `order_ratings.feedback` (GAP-C) + the extended
   completion gate, landed **on Node now, carried-fixed in the port**. *(The one non-optional condition;
   liftable three ways.)*
2. **[honesty/proof, §1·finding #2] Extend the completion gate and the pre-flip P-proof to the whole
   subject-graph.** `completed` requires the orders (and order_ratings) anonymized, not only
   `customers.anonymized_at`; the N1 data-level P-proof asserts the *fixed* contract (customer + orders +
   ratings gone under NOBYPASSRLS), so it can go RED on a GAP-A-incomplete port. Do not let the #61 backstop's
   customer-only scope re-ship a partially-false `completed`.
3. **[irreversibility, §3] Tie the re-erase-on-restore runbook to the fixed erasure.** The restore runbook's
   re-erase pass fans out the *same* order-graph the STOP fix adds (else a restore un-erases the GAP carriers).
   Keep the erasure proof (data-level, rows stay gone) opposite-polarity from the restore-drill (fidelity,
   rows come back); never repurpose one as the other's oracle.
4. **[retention legality, §4] Record the basis position; make the UI honest; fix GAP-B/C in the shared
   null-set.** Accept-risk + DPA clause (controller documents a basis); confirm the default stays 365 (not
   max); the retention UI must not present the maximum as neutral (no keep-everything dark-pattern); the
   nightly sweep nulls GPS + feedback too, so an aged-out order does not silently retain HIGH-RISK carriers.
5. **[cutover, §5] Name the flip as a second human act, alongside S5.** Prove the whole-graph P-proof green
   under NOBYPASSRLS *before* the flip; the flip is a separate explicit operator go/no-go, not folded into
   the S8 fleet flip; the Rust create writes the shared `gdpr_erasure_requests` row (worker-recoverable);
   adopt the claim-before-work CAS over the COMMIT-then-mark TOCTOU.
6. **[DEFINER, §6] Prefer the app-side seated `orders` arm over a new companion DEFINER.** The `orders` RC4
   `app.current_tenant` arm exists — use it for the fan-out; carry 088's `search_path` pin +
   `REVOKE PUBLIC` + `GRANT dowiz_app`; a guardrail greps for any unpinned `SECURITY DEFINER` the S9 path
   depends on. Minimise the DEFINER surface on the primary PII table.

---

## The question nobody asked (§7)

Every seat in this council — and the tasking — speaks for the **customer** as the data subject: the
customer's Art.17 request honoured completely, their orders reached, their id not driving a cross-tenant
erasure, their retention bounded, their backup not resurrecting them. That frame is correct and it is where
the whole compliance apparatus is pointed.

**Nobody speaks for the courier — the *most-surveilled* subject on the platform, who has no erasure path at
all.** The entire S9 erasure machine is customer-only: `gdpr_erasure_requests` carries `subject_phone` and
`customer_id` (data-map #13) — it cannot even **represent** a courier subject. Yet the courier is tracked
more intimately than any customer: continuous GPS (`courier_positions.lat/lng`, data-map #18, HIGH-RISK +
flagged for a DPIA), encrypted name/email/phone (#17), session and audit IP/UA hashes (#19). And the
customer's `order_ratings.feedback` that GAP-C leaves un-erased carries a `courier_id` — it is *about* a
named worker. When a courier leaves the platform, there is **no request they can file and no anonymizer that
can find them.** The system builds a careful, verified, human-gated Art.17 machine for the *owner's
customer*, and none for the *platform's worker* — the person whose location it logged every few seconds.

The unasked question is not technical and it does not block S9 (couriers are correctly out of this surface's
scope): *the reddest compliance surface guarantees erasure for the least-surveilled subject — what
guarantees it for the most-surveilled one, who is tracked by GPS and has no request path?* The honest answer
is the same shape as the customer's: the courier is a data subject with the same Art.17 right, and the
absence of a courier-erasure path is a **gap someone must own on a register with a trigger**, not a silence.
The courier cannot attend this council; naming their missing seat is the least it can do — so that the
worker the platform watched most closely is not the one subject it can never forget.

---
*Advisory only. One scoped ETHICAL-STOP (friction, liftable three ways — the human is final). Nothing here
authorizes a code change, blocks S9, or overrides a conscious operator decision.*
