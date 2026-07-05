# S4-MEDIA Port — Council Packet · COUNSEL OPINION

> Seat: **Counsel** (ethics · aesthetics · strategy) in the S4-media Triadic Council.
> Advisory, non-blocking. Architect asks "will it work"; Breaker asks "how it breaks";
> this seat asks *should it exist, is it fair/honest/dignified, is it whole, what is the long
> horizon, who bears the cost, and what did nobody ask.* Docs only — no code, no design authority.
> Every load-bearing claim below was verified against live source, not trusted from the packet.

---

## OPINION: **PROCEED-WITH-REVISIONS** · **one scoped ETHICAL-STOP (friction, not veto)**

I looked adversarially at the two places this surface can touch a real human body of data — the
**customer's doorway photo** and the **cross-tenant object boundary** — and the object boundary holds
(the key is server-derived from the membership `locId`; a leaked presign is one object / one product /
≤300 s / same tenant — I verified the derivation and the confirm prefix check). The doorway photo does
**not** hold, and it fails against a *grounded* red-line, not a matter of taste. So this opinion carries
**one ETHICAL-STOP**, and it is deliberately narrow.

**The STOP is not "do not port entry-photo."** That route is live, wired, optional, and honestly
labelled — stopping a carried live feature would be verdict-not-friction, the overreach the S3 seat
correctly refused. The STOP is on **one property**: the entry-photo object is **outside the erasure
graph entirely** — neither anonymized nor deleted when its own order is erased — and the packet's
recommended disposition (Q4b, "reaper on *unattached* photos") does **not** close it, because the
photos that became orders are *attached* and an attachment-aware reaper skips them. This intersects the
**"анонімізувати-не-видаляти"** erasure obligation in a way I verified in the anonymizer's own bytes.
The STOP is friction: it lifts the moment the operator records **any one** of three minimal conditions
(§STOP). The human is final; this does not block S4, only pins the one route's erasure gap to a
recorded decision.

Everything else is Opinion, not a stop. The packet is disciplined, unusually honest about its own
deferrals, and its spike-evidence has already *removed* the imaging driver for the two-image split — a
mark of a team that tests before it argues. My value is narrow and I keep it narrow.

---

## Verification note (charge: "find the FE caller, verify the load-bearing claims")

- **The FE caller and the product goal are real and modest.** `ContactInfoSection.tsx:137-155` — the
  entry-photo is an **optional**, delivery-only field, labelled *"Entrance photo (optional)"* with the
  honest hint *"Helps the courier find your entrance."* It is uploaded by the anonymous, pre-order
  checkout client, the returned key is stored on order-create as `orders.delivery_photo_key`
  (`orders.ts:595`), and it is revealed to the assigned courier **only during** `['assigned',
  'accepted', 'picked_up']` (`assignments.ts:52`), tap-to-enlarge on `DeliveryPage.tsx:421-435`. The
  product goal is legitimate and small: *cut the last-100-metres friction of finding a door.* Consent is
  present (optional + purpose-stated), scope is narrow (one courier, one active order). **This is not a
  surveillance feature. It is a good feature with an unbounded tail.**

- **The erasure-graph gap is worse than the packet states — I read the anonymizer.** The packet says
  entry-photos are "keyed by UUID, not by customer — orphaned from the erasure graph." That undersells
  it. The key **is** reachable — it lives on `orders.delivery_photo_key`, and the order is linked to the
  customer. Yet the anonymizer's `anonymizeOrder` (`anonymizer/index.ts:237-249`) nulls
  `delivery_address`, `delivery_instructions`, `customer_messenger_handle`, `receiver_name/handle`,
  `client_ip_hash` — **and leaves `delivery_photo_key` populated** — while returning `storagePurged: 0`
  **hardcoded** (`:276`). The **only** `storage.delete` in the entire service is `avatar_key`, on the
  customer path (`:169`). So on GDPR erasure *or* retention-expiry of an order, the **text** address is
  scrubbed and the **photo of the front door** — more location-identifying than the text — survives, in
  the row and as a public-by-key R2 object, **indefinitely**. The pattern to fix it already exists
  (avatar purge); it was simply never extended to the doorway photo.

- **The object-write boundary holds.** Key = `${locId}/${pid}-${hash}.webp` built from the
  membership-resolved `locId` (`spa-proxy.ts:236`); product-media confirm rejects any `storageKey`
  outside `${locId}/${productId}/` (packet §5/§8, cross-checked). A client cannot sign or confirm into
  another tenant's prefix. Affirmed.

- **The two raw-pool writes are real and the fix is correctly inherited.** `spa-proxy.ts:252`
  (`db.query`, no GUC) and the theme-logo raw `db.connect()` are the never-copy leak class; routing all
  three media-metadata writes through the S3 `with_user` seam (Q5) is a *confirmation* of REV-2/REV-10,
  not a new decision. Affirmed.

- **The spike-evidence is honest and load-bearing.** Pure-Rust `image` + `webp` builds `cc`-only (no
  `cmake`), strips metadata by default, and matches `fit:inside` math — so imaging **no longer forces**
  the split. And the spike caught a genuine silent-wrong-output trap the two-profile framing missed:
  `image::open()` does not auto-apply EXIF orientation (sharp does), so all three profiles need an
  explicit `apply_orientation()` or they ship sideways images that still pass a "some-webp-rendered"
  assertion (REV-EXIF). That is Breaker's robustness domain and I will not re-litigate it — I only note
  it is the kind of *test-before-argue* find that earns the packet trust.

---

## By charge

### 1. Q4 — unauthenticated entry-photo: CARRY the flow, FIX the controls, and close the erasure gap

**Disposition I support: CARRY the flow + Q4b compensating controls — *plus* the erasure link (the
STOP), and give Q4c a harder look than the packet does.**

Weighed across lenses (plural, per mandate — no single codex):

- **Care / harm.** What does a careful custodian of a stranger's front-door photo do? Keep it *exactly*
  as long as the delivery needs it, and not one hour longer. The delivery's useful-life is minutes to
  hours (the courier reveal window is literally three order statuses). The current storage-life is
  **infinite**. That gap between useful-life and storage-life is the whole harm, and it is not malice —
  it is *neglect*. Care is the relevant virtue, and indefinite keeping fails it.
- **Justice / who bears the cost.** The customer whose home is photographed bears 100% of the retention
  risk; **no one gains** from the laxity — it is not a tradeoff for anyone's benefit, it is pure entropy.
  Asymmetric cost with no counterparty is the clearest injustice signature there is.
- **Consequences / long horizon.** The expected harm is (low probability of breach/misuse) × (high harm:
  a corpus of geo-linked doorway photos) + monotonically-growing orphan cost. The integral of that risk
  over indefinite time only rises. Bound the retention and you bound the integral.
- **Honesty / consent.** The FE is honest at intake (optional, purpose-stated) — good. But note the
  asymmetry three lines up in the same component: the *receiver contact* card promises *"we share this
  with the courier only to deliver this order, then delete it"* (`ContactInfoSection.tsx:132`), and the
  anonymizer **keeps** that promise (it nulls `receiver_name/handle`). The **photo** — more sensitive —
  gets **no such promise and no deletion**. We are, structurally, more careful with the text than with
  the picture of someone's home.

On the disposition menu: **Q4a (carry verbatim) is the wrong floor** — it knowingly re-ships an open
harm surface at the exact moment we hold the pen. **Q4b (sniff + global cap + `ENTRY_PHOTO_ENABLED`
kill-switch + bomb bound + reaper) is the right pragmatic floor** and I support it — the kill-switch in
particular is dignified ops hygiene (an open front door you can close instantly). But Q4b as the packet
frames it leaves the erasure gap open (its reaper targets *unattached* photos; the attached ones are the
PII-bearing ones). **Q4c (checkout-scoped anonymous token) deserves more weight than the packet gives
it** — see the steel-man; it is the only option that closes the illegal-content-hosting vector *at the
root* rather than mitigating it.

### 2. PII in the photo — metadata-strip answers metadata-PII only; **retention** is the load-bearing control

The packet treats "EXIF-GPS is stripped (good)" as most of the PII answer. Sharpen the frame:

- **Metadata-PII (GPS, camera, timestamp):** strip-by-construction + a machine assertion is **sufficient
  and correct**. The spike confirms `image`/`webp` strip by default; the DoD asserts zero-EXIF on
  output. Affirmed — this class is closed.
- **Content-PII (the front door itself, an incidental face, a plate, a house number):** these are **in
  the pixels**, and strip-metadata does **nothing** for them. You cannot machine-scrub them without
  destroying the photo's only purpose (a blurred doorway is useless to the courier). Therefore, for
  content-PII, **the sole available control is retention** — bound how long it exists and who can reach
  it. Machine perception is *not* an alternative to a retention bound here; it is orthogonal. This is why
  the erasure gap is not a compliance footnote but the *primary* PII control for this asset, and why it
  earns the STOP rather than an Opinion note.

### 3. Q1 — the two-image split: the recommendation is honest restraint, with one aesthetic caution

**Affirm (C): build the seam, defer the runtime.** The YAGNI read is clean: the spike removed the
imaging driver, the back-of-envelope is single-digit uploads/sec, and standing up a second always-on
Fly app for that workload would be exactly the Prime-Video "we recombined the monolith" over-
provisioning the packet cites. Deferring the *runtime* (not just the *decision*) is the honest form of
"schema rich, runtime minimal" — it does not conserve latent complexity, it *refuses* it until a second
surface (OCR, or presign-there) makes the split pay across two consumers instead of one.

**One caution, non-blocking.** The `trait ImageProcessor` seam is genuinely schema-rich: it is
exercised, typed, load-bearing, and it is where the split's option-value actually lives. Ship it. But
the packet also proposes to *author the `media-worker` Dockerfile now, leave it unbuilt.* A Dockerfile
with no image built, no service consuming it, and no test touching it is **not schema in the
load-bearing sense** — it constrains nothing and is exercised by nothing; it is a doc that will drift
out of sync the first time a base image bumps. That is schema-*cruft*, not schema-*rich*. Prefer to
author it **lazily** — at the moment Q2→(a) or the OCR slice commits — so the artifact is born with a
consumer. The seam is the honest anticipation; the unbuilt Dockerfile is speculative scaffolding wearing
the seam's clothes. Minor, and it does not block approval.

### 4. Q2 — hand-rolled SigV4 presign: prefer the option that *removes the class*, not the one that guards it

This is the sharpest strategy/security call in the packet, and I read it through "the capability that
cannot be misused is the one not present in the artifact" (the same principle the S2 seat praised).

- **(b) hand-rolled query-SigV4** puts a **crypto surface in the scratch `api` image**. The packet's
  failure analysis is honest — a signing *failure* is fail-safe (loud); a signing *looseness* is the
  hazard. But "roll your own SigV4" looks smaller than it is: the danger is not the happy path, it is
  R2-vs-AWS **canonicalization edge cases** — URI-encoding a key path that contains slashes,
  `UNSIGNED-PAYLOAD`, signed-header casing, query-param ordering, a `productId` with an odd character.
  This is where the *seductive-elegance* flag fires: a "~1 canonical-request function" is exactly the
  kind of thing that reads as clean and hides its sharp corners. Acceptable **only** behind the offline
  byte-fidelity test vector the packet names — and even then, it buys a crypto surface the system spent
  effort avoiding once already (the `aws-sdk-s3` "works-in-Docker-broken-locally" rejection).

- **(c) server-proxied upload** is, on the value axes, the **cleanest** option, and I would elevate it
  above the packet's framing. It **deletes an entire failure class** (leaked-presign *and* hand-rolled
  crypto) rather than mitigating it, and it keeps the crypto surface to the one **already-working**
  header-signed path. There is also a **conceptual-integrity dividend the packet does not name**: *two
  of the three upload paths already proxy through the API* — product-image (`spa-proxy.ts:213`, sharp →
  `storage.put`) and entry-photo (`spa-proxy.ts:268`) are both proxy-through. The **presign path is the
  odd one out.** Option (c) does not *add* a pattern; it *removes the one divergent pattern* so all
  three uploads share a single trust boundary. Fewer distinct boundaries is fewer places to leak — the
  aesthetic doing its ethical job. The honest cost: (c) sacrifices the direct-to-R2 offload for large
  rich-media (≤25 MB clips buffer/stream through the API), which the back-of-envelope absorbs at this
  scale but which becomes a real memory/bandwidth line if rich-media adoption ever scales.

**My reading for the operator:** land **(c) [proxy, removes the class]** or **(d)→(a) [defer on Node,
then let presign ride the Debian-slim `media-worker` *if* OCR stands it up anyway — a clean `aws-sdk-s3`
presigner, zero hand-rolling]**. Treat **(b) as the last resort**, chosen only if the operator
specifically wants presign in the scratch image with no second runtime, and only behind the test vector.
The through-line: at single-digit QPS, do not buy a crypto surface you can *delete*; if scale later
demands the offload, *that* is the moment to add presign — via a real presigner in a real second
runtime, never hand-rolled. Same schema-rich/runtime-minimal discipline, applied to crypto.

### 5. Scope, ordering, Charter

- **Scope is restrained, not bloated.** The NOT-S4 boundary is drawn correctly and generously: backup
  multipart → the DR ops-binary; OCR → its own post-S4 slice (S4 only *enables* the `media-worker`
  image); brand-extractor as a non-🔴 rider on the shared crate (Q9 — fine, one pure imaging call is
  cheaper as a rider than a re-import). The Q5 raw-pool fixes are inherited, not invented. No schema
  change, no order-total (correctly S5). This is a disciplined surface.
- **Ordering (S4→S5) has one honest wrinkle the packet should record.** The entry-photo *spans*
  surfaces: it is **created** in S4 (the upload route) but **stored, revealed, and erased** in S5+ (the
  order row, the courier assignment, the anonymizer). The erasure fix I require (purge
  `delivery_photo_key` + object on order-anonymization) therefore lands **partly in a future surface**,
  not in S4 itself. That is not an escape hatch — it is a scope-honesty point: S4 owns the *route-level*
  controls (retention TTL/reaper reachable to entry-photo objects, global cap, kill-switch, sniff, bomb
  bound); the *erasure-cascade* fix is owed by the order/anonymizer port. The STOP's lifting conditions
  are written to respect that boundary (see §STOP).
- **Charter: clean, with one long-horizon note.** No military/warfare, no surveillance-for-harm, no
  commons-capture. The entry-photo is consented and single-purpose — it is **not** surveillance today.
  But an *indefinitely-retained* corpus of geo-linked customer doorway photos is precisely the
  surveillance-*shaped* asset that forms by neglect rather than intent: nobody decides to build it, it
  simply *accretes* because deletion was never wired. The Charter's dignity/commons spirit and the
  surveillance-creep pathology both point the same way — **bound the retention so the corpus never
  forms.** That is the strategic reframe of the STOP: it is not merely GDPR hygiene, it is *refusing to
  accidentally build the thing the Charter says we must never build on purpose.*

---

## Non-blocking aesthetic / strategic notes

- **The avatar-vs-doorway asymmetry is an integrity flaw worth naming.** The anonymizer *deletes* a
  customer's chosen `avatar_key` (`:169`) but *keeps* their unconsented-by-passersby doorway photo. The
  **less** sensitive asset (a picture the customer picked of themselves) is treated **more** carefully
  than the **more** sensitive one (a picture of where they live). A whole design treats its most
  sensitive asset most carefully; this one has it backwards. Fixing the erasure gap also *restores the
  aesthetic* — symmetry between the two purge paths.
- **Q2(c) unifies three upload paths into one** (see charge 4). Conceptual integrity is a leading
  indicator of both quality and safety here: one upload boundary is fewer bugs and fewer leak sites than
  two. If the operator lands (c), the surface gets *simpler* by porting, which is the rare and good kind
  of rewrite.
- **The `ENTRY_PHOTO_ENABLED` kill-switch (Q4b) is dignified restraint** — an open unauthenticated front
  door you can close in one flag-flip is honest ops posture. Keep it default-on but *reachable*, and pair
  it with the global cap so a botnet cannot fan out under the per-IP limit.
- **Content-addressing honesty (Q7).** Carrying the client-declared, never-re-verified sha256 as a
  *name* (not an integrity proof) is fine — but keep the confirm-side mime re-sniff as the real gate,
  and do not let the "content address" language in the code imply an integrity property the field does
  not have. A name that looks like a proof is a small dishonesty that a future reader will trust. One
  comment ("naming scheme, not integrity — mime-sniff is the gate") keeps it honest.

---

## Steel-man of a rejected option (obligatory)

**Q2 option (b) — hand-rolled SigV4 query-string presign — the option I land *against*.**

Its strongest case, made fairly: (b) is the *only* option that preserves **both** things (a) and (c)
each sacrifice — a **single runtime** (no second always-on Fly app; it honors the monolith-first ADR and
the Prime-Video lesson *more completely* than (a) does) **and** the **direct-to-R2 offload** (the client
PUTs bytes straight to R2; the API never buffers the 25 MB clip, which (c) reintroduces). And the usual
"never roll your own crypto" reflex is **mis-aimed here**: that maxim warns against *inventing a
scheme*. SigV4 query-presign is not an invention — it is a **fully-specified, deterministic, public
standard**, and R2's implementation is a known-good reference you can pin an **offline byte-fidelity test
vector** against. Implementing a published spec with a golden-vector oracle is categorically safer than
designing a primitive; it is closer to "port a hash function against its test vectors" than to "invent a
cipher." By that light, (b) is the option that keeps the system *whole* (one image) and *cheap* (offload
preserved) while making the crypto *testable to the byte*. That is a genuinely strong position and I do
not dismiss it.

**Why I still land elsewhere.** The maxim's *spirit* survives even when its letter doesn't: the hazard
is not "will the happy path sign correctly" (the vector proves that) — it is **canonicalization
divergence at the edges** (slash-bearing key paths, `UNSIGNED-PAYLOAD`, header casing, odd `productId`
characters), where a *looseness* yields a URL more permissive than intended, silently. A golden vector
proves the *cases you thought to write*; it cannot prove the case a future R2 quirk introduces. Against
that, (c) does not *guard* the class — it *removes* it, at a bandwidth cost this scale absorbs, and it
*unifies* the upload paths as a bonus. (b) is not wrong on the engineering; it is a heavier *ongoing
liability* (an owned crypto surface in the hot image) than a single-digit-QPS surface should carry when
a class-removing alternative exists. So: (b) is acceptable **behind the vector, as a last resort** — it
loses to (c) on *what it must keep alive*, not on whether it can be made to work.

---

## The scoped ETHICAL-STOP (grounded line + why + minimal lifting conditions) — §STOP

**Grounded red-line:** *"анонімізувати-не-видаляти"* — the erasure obligation. The line presumes
erasure *happens*, in anonymized form. A photograph of a front door **cannot be anonymized in place** —
the pixels *are* the PII; there is no scrub-that-keeps-the-row for an unblurrable image. So, applied
honestly to this asset, the erasure obligation collapses to **delete the object**. The current system
does **neither** — it does not anonymize the photo (impossible) and does not delete it (verified:
`anonymizeOrder` leaves `delivery_photo_key` set and returns `storagePurged: 0`, `anonymizer/index.ts:
237-276`; the only object-delete in the service is `avatar_key` at `:169`). The doorway photo therefore
**escapes the erasure system entirely**, and Q4b's recommended reaper (unattached-only) does not reach
it. This is a *verified intersection* with a grounded line — not taste, not the packet's word, but the
anonymizer's own bytes.

**Why a STOP and not an Opinion (and why S4 differs from S3, where I did not stop):** the S3 seat
declined a stop on a property that was *carried, unwired, and safety-neutral in the port.* Here the
property is **carried but live and wired** (real checkout, real courier reveal), and the port is
**actively re-opening this surface's controls** (Q4b) — so "we didn't touch it" does not apply; we *are*
touching it, and choosing what to leave open. The threat-model and open-questions both pre-designate
this as the ETHICAL-STOP candidate and ask counsel to rule. A ruling that Q4b-as-framed is *sufficient*
would be wrong (I verified the gap it misses); a ruling of scoped friction is the honest answer to the
question the packet actually asked.

**This STOP is friction, not veto.** It does not block S4 or the entry-photo port. It pins **one route's
erasure gap** to a recorded human decision, and it **lifts the moment the operator records any ONE** of:

1. **Erasure link (preferred, cheapest — the pattern already exists).** Extend the anonymizer's existing
   `avatar_key` purge to `delivery_photo_key`: on order anonymization (GDPR *and* retention), null the
   column **and** `storage.delete` the object. This is a ~5-line symmetry with code that already ships,
   and it restores the avatar/doorway integrity. *(Lands in the order/anonymizer surface, not S4 —
   acceptable to record now as an owned, triggered debt if that surface has not yet ported.)*
2. **Retention TTL bound (S4-owned alternative).** A hard TTL on entry-photo objects tied to the
   delivery window + a grace period (e.g. object reaped N days after the referencing order reaches a
   terminal state, or after upload if never attached). This bounds the storage-life to the useful-life
   and closes both the attached and unattached cases without waiting for the order surface. This *is*
   S4's own reaper, generalized past Q4b's unattached-only framing.
3. **Recorded accepted-risk (the explicit human override the mandate protects).** The operator records —
   on a reviewed accepted-risk register, with a **named owner** and a **trigger** (`= "the
   order/anonymizer surface ports, OR the launch trigger (first real paid order) fires, whichever
   first"`) — explicit acceptance that *"customer doorway photos persist publicly-by-key indefinitely
   and survive GDPR/retention erasure of their own order."* A risk consciously accepted, owned, and
   triggered is honest; the same risk left as an unread footnote is where it goes to be forgotten. This
   is the S3 register pattern, and it is a legitimate lift — the human is final.

Any one of the three lifts the STOP. My *preference order* is 1 or 2 over 3 (close the gap rather than
name it), but 3 is a valid conscious human choice and I will not override it — that is the whole point of
friction-not-verdict.

---

## Explicit conditions before code (§C)

Advisory — the human decides. Counsel-weighted, ordered by ethical load.

1. **[erasure, the STOP] Bring the doorway photo inside the erasure graph, or record its escape.**
   Satisfy §STOP condition 1, 2, or 3 before the Rust entry-photo route becomes the authoritative
   cutover path. Preferred: extend the `avatar_key` purge to `delivery_photo_key` (symmetry with
   shipping code) **or** a delivery-window-scoped retention TTL. Fallback: a reviewed accepted-risk row
   with owner + trigger. *(This is the one non-optional condition; it is liftable three ways.)*
2. **[Q4 controls] Adopt Q4b as the floor and give Q4c a serious look.** Ship the sniff-before-decode,
   the global cap, the `ENTRY_PHOTO_ENABLED` kill-switch (default-on), and the pre-decode bomb bound.
   Separately, put Q4c (checkout-scoped anonymous token) on the operator's desk as the *stronger* control
   — it is the only option that closes illegal-content-hosting-on-brand-domain at the root; the token
   seam is a modest cost the port is the natural time to pay.
3. **[Q2 crypto] Prefer the class-removing path; treat hand-rolled crypto as last resort.** Land (c)
   [proxy — deletes the leaked-presign + hand-rolled-crypto class and *unifies* the three upload paths]
   or (d)→(a) [defer on Node, then a real presigner in the `media-worker` if OCR stands it up anyway].
   Choose (b) only if the operator specifically wants presign in the scratch image, and only behind the
   offline byte-fidelity SigV4 test vector, signed as a 🔴 crypto item.
4. **[Q1 restraint] Ship the seam, defer the Dockerfile.** Build and test the `trait ImageProcessor`
   seam (real schema-rich). Author the `media-worker` Dockerfile *lazily* — when Q2→(a) or the OCR slice
   commits a consumer — so the artifact is born with something that exercises it, not as scaffolding that
   drifts.
5. **[honesty] Keep the code's language honest.** One comment on the client-declared sha256 ("name, not
   integrity — the mime re-sniff is the gate"), and preserve the `delivery_photo_key`/receiver-contact
   symmetry once the erasure link lands, so the system is not — in code or in UI copy — more careful with
   the text than with the picture of someone's home.

---

## The question nobody asked (§7)

The entire packet frames entry-photo risk from **two frames the platform can see**: the *abuse* frame
(DoS, cost, illegal-content hosting on the brand domain) and the *customer-PII* frame (the doorway, the
GDPR link). Both are correct. Both are about people the system has a *row* for — the platform, the brand,
the customer who checked out.

Nobody in this surface speaks for the **person who is in the photo but is not the customer.** A
1024×1024 doorway shot taken by an anonymous phone at checkout will, sometimes, contain a **neighbour on
their stoop, a face at a window, a licence plate, a child in the yard.** That person **consented to
nothing** — they are not the customer, who at least ticked "optional" and read "helps the courier."
Strip-metadata does **nothing** for them, because they are in the pixels, not the EXIF. And they are
reachable by **no erasure mechanism in this system** — they are not a customer, not a `subject_id`, not a
row; there is no request they can file and no anonymizer that can find them. Every control in this packet
— consent, key-unguessability, the GDPR graph, the reaper — is keyed to the *account-holder*. The
incidental third party falls through all of them.

The unasked question is not technical and it does not block the port: *the system carefully bounds harm
to the people it has rows for — what bounds it for the person it photographed by accident, who has no
row, no consent, and no erasure path?* The honest partial answer is the same as the customer's:
**retention.** You cannot get the incidental face's consent and you cannot find them to erase them — but
you *can* make sure the photo they are in does not outlive the one delivery it was taken for. That is one
more reason the retention bound (§STOP condition 2) is the load-bearing control on this surface, and one
more reason it should not be the last thing anyone thinks about. The person whose face is in the frame
cannot attend this council; the retention TTL is the only seat we can offer them.

---
*Advisory only. One scoped ETHICAL-STOP (friction, liftable three ways — the human is final). Nothing
here authorizes a code change, blocks S4, or overrides a conscious operator decision.*
