# Counsel Opinion — Order acquisition-attribution `channel` (write-only metadata)

- Seat: Counsel (Philosopher / Physician) — advisory, non-blocking except a grounded ETHICAL-STOP.
- Date: 2026-07-04
- Reviews: `docs/design/order-channel-attribution/proposal.md`, `docs/adr/ADR-order-channel-attribution.md`
- Verdict: **CLEAR TO PROCEED. No grounded ETHICAL-STOP.** Two proportional recommendations (one of them
  a neighbor-field privacy finding), plus non-blocking aesthetic/strategic notes.

---

## 1. Reasoning by lens (only what is load-bearing)

**Justice / stakeholders.** Who wins: the restaurant owner — a legitimate first-party marketing insight
(which physical/social channel drove an order), squarely serving the small business DeliveryOS exists for.
Who bears cost/risk: the customer, whose arrival channel is recorded. But the cost is near-zero at the
margin — the order *already* carries far more identifying data (delivery address, phone hash, receiver
name, messenger handle). `channel` is a coarse 13-token, tenant-agnostic tag; it is **strictly less
sensitive than what the same row already holds.** The distribution is fair: real owner value, negligible
incremental customer exposure, no third-party sale, no platform-vs-user asymmetry.

**Dignity / autonomy (courier).** N/A — courier untouched. No surveillance, no coercion, no agency removed.

**Honesty / consent + dark-pattern risk.** The `?ch=` param is invisible to the customer and gathered
without an explicit prompt — but that is ordinary web attribution (a UTM/referrer equivalent), and the
codebase already collects comparably-benign query-param signals. The decisive ethical safeguard is the
**write-only, no-reader invariant**: because `channel` is structurally forbidden from reaching pricing,
status, dispatch, notifications, or authz/RLS, the design *forecloses* the real dark-pattern — "charge the
Instagram arrival more," "deprioritize the QR customer." That invariant is not merely architectural
tidiness; **it is the ethical mechanism.** Its weakness is that today it rests on a Zod enum and prose
(R5). Ethical protection that lives only in a code comment degrades silently. (See rec #1.)

**Care / harm.** The person most exposed to a bug here is not the customer but the **owner**: the silent
`unknown → 'other'` fallback means a malformed printed QR lands every scan as `'other'`, and the owner
reads "this channel underperforms" when the truth is "the link I paid to print is broken." A care failure
against the party the feature is *for*. Non-blocking (analytics/QR-kit is a later lane), but real (note A).

**Long horizon / strategy.** Reversibility is excellent (remove by omission; inert data). Lock-in: none.
The second-order watch-item is conceptual: attribution is the seed from which ad-tech profiling grows.
Today `sessionStorage` + per-order + no-reader keeps it clean and non-tracking. It would only turn
concerning if `channel` were ever joined to a **stable cross-order customer identity** — that is a
different feature with a different ethics review, and nothing here presumes it. Strategically this is
owner-growth tooling, mildly adjacent to the launch trigger (first real paid order), not a blocker for it;
cheap enough that the adjacency is fine.

**Commons / Charter.** As owner-facing first-party analytics this is pro-small-business and consistent with
the commons ethos (data serves the merchant, is not captured or sold). The one phrase to hold to account is
the proposal's "*and, later, our own analytics*." Owner-first is fine; if DeliveryOS ever **aggregates
`channel` across tenants for the platform's own benefit**, that crosses from merchant-service into
platform-level data capture and deserves its own Charter review. Horizon marker, not a stop.

**Aesthetics / integrity.** Genuinely elegant restraint, not seductive-elegant: reuse the existing
per-order sidecar, no new column/migration/dependency, write-only, reversible-by-omission, single
source-of-truth enum, mirrors the existing `otp_verified`/`client_ip_hash` pattern exactly. "Schema-rich,
runtime-minimal" is honored as intended. High conceptual integrity.

**Epistemic.** The strongest objection is not performance — it is *legibility* (see steel-man §4). The
proposal's own R4 ("a naive `SELECT metadata` exposes co-located hashed PII") is quietly an argument
against its own storage choice, and I found the neighbor gap is worse than R4 admits (rec #2).

---

## 2. ETHICAL-STOP(s): **none**

I tested this against the grounded red lines (human-in-loop / no-autoban; friction-not-verdict;
courier-completes; GPS-garbage-rejected; cash→friction; **anonymize-not-delete**; **zero-PII-in-AI**;
claim-check; soft-confirm-not-trap; server-authoritative; a11y; schema-rich-runtime-minimal; trigger =
first real paid order). None are crossed by `channel` itself:

- `channel` is not PII (a fixed tenant-agnostic enum token), so **anonymize-not-delete** and
  **zero-PII-in-AI** do not bite on it. Server is authoritative (default `web-direct`, server-side Zod
  validation) — the UI cannot lie about it. `schema-rich-runtime-minimal` is honored.

No ETHICAL-STOP. The two items below are **friction, not verdict** — proportional recommendations, freely
overridable by the human.

---

## 3. Proportional recommendations (advisory)

**Rec #1 — Make the write-only invariant a guard, not a sentence (elevate R5/R3-guard).**
The no-reader-in-price/status/dispatch/authz rule is the ethical heart of this feature. Promote it from
prose to a deterministic guardrail (a lint/grep assertion or a red→green test that fails if
`metadata->>'channel'` acquires a reader in pricing/dispatch/authz modules), matching this repo's
"guardrails decide, comments advise" discipline. Cheap, and it is what keeps a future maintainer from
silently turning acquisition analytics into channel-based price discrimination.

**Rec #2 — Grounded neighbor finding: the anonymizer does NOT scrub the metadata copy of `client_ip_hash`.**
The proposal reassures that redaction "already owns `client_ip_hash`." That is only half true, and this
change makes the gap matter more. Grounded:
- `apps/api/src/lib/order-persistence.ts:95` writes `client_ip_hash` **into `orders.metadata`** jsonb.
- `apps/api/src/lib/anonymizer/index.ts:212` on erasure sets the **column** `client_ip_hash = NULL` — it
  does **not** touch `metadata`. So after a GDPR erasure, `metadata->>'client_ip_hash'` (a hashed IP =
  pseudonymized personal data) **survives** in the row.

This is **pre-existing and not caused by `channel`** — no ETHICAL-STOP on this change. But `channel` is
precisely the new reason engineers will start reading `metadata` (to report attribution), which raises the
odds the residual hashed-IP gets surfaced/exported. Recommend (as a separate red-line-adjacent ticket for
the anonymizer/GDPR owner): have the anonymizer also strip `client_ip_hash` from `metadata`
(`metadata = metadata - 'client_ip_hash'`) so the jsonb copy honors the same erasure as the column. And,
per the proposal's own R4, any future analytics/AI read must project `metadata->>'channel'` **only**, never
`SELECT metadata`.

---

## 4. Non-blocking aesthetic / strategic advice

- **A — `'other'` silent fallback masks a broken QR (owner harm).** Because `unknown → 'other'`, a
  malformed printed QR is indistinguishable from a genuinely-unlisted channel. When the QR-kit / analytics
  lane is built, either (i) separate `'other'` (valid-but-unlisted) from a distinct `'unknown'`
  (malformed/garbled), or (ii) treat an `'other'` spike as a **QR-health alarm** the owner can see —
  otherwise the owner silently pays to print codes that report as "underperforming."
- **B — i18n the channel *labels*, keep the *tokens* stable.** The 13 enum values are correct as stable,
  untranslated keys — but never render raw tokens (`gbp`, `telegram-tma`, `web-direct`) to owners. Every
  displayed label needs `al`/`en` catalog entries (this repo's i18n SSoT + parity gate). `gbp` in
  particular is opaque; give it a human label ("Google Business Profile").
- **C — Document the taxonomy.** Publish the token→meaning map so owners understand `web-direct` vs
  `widget` vs `other`; an undocumented enum becomes folklore.
- **D — Transparency toward the attributed customer.** Consider a one-line note in the storefront privacy
  notice (`/compliance` SoT) that arrival source is recorded for the owner's own analytics. Low stakes,
  but it is the honest, commons-consistent move and costs a sentence.
- **E — Treat enum-growth (R3) as an ethical control surface, not just a release chore.** A 14th token that
  encodes something sensitive (an affinity-group venue, a health/identity-linked partner) would silently
  make this metadata more revealing. Add a one-line "is this token sensitive?" check to the
  add-a-channel process. Keeping the allowlist to benign marketing channels is doing quiet ethical work.

---

## 5. Steel-man of the rejected option (Option B — dedicated `orders.channel` column)

The proposal rejects B primarily on a **lane constraint** ("needs a migration, out of lane") rather than on
the merits — which understates B's real strength. Steel-manned on merits:

> Privacy-relevant fields are better governed as **named columns than as opaque jsonb keys, because you
> cannot govern what you cannot see in the schema.** A first-class `channel text` column is enumerable in
> `\d orders`, visible to DSAR/retention tooling, and its allowlist can be enforced by a **database-level
> CHECK** — a strictly stronger guarantee than a Zod enum that holds on exactly one write path (B's CHECK
> closes R2 permanently; A only mitigates it). Decisively, a dedicated column **separates attribution from
> the hashed-PII it is otherwise buried beside** — which is precisely the R4 smell and the rec-#2 finding
> above. The proposal's own concern that "a naive `SELECT metadata` exposes co-located PII" is an argument
> *for* B's separation-of-concerns, not against it.

Given the hard lane constraint (no migration this lane) and the field's write-only, no-reader nature, **A
remains the defensible choice for *this* change** — but B's merit is legibility/auditability, not
performance, and the proposal's deferred-first-class-column path (§5) is the correct hedge. If/when a
reader or a DSAR-tooling requirement appears, B is the right destination, and the identical wire contract
means the migration lands with zero client change.

---

## 6. The question nobody asked

**Was the attributed customer's perspective represented at all?** The proposal reasons entirely from owner
value and platform convenience; the person actually being tagged ("you arrived via Instagram," now
permanently on your order) is never given a standpoint. Nobody asked: *should they be told, and would they
object?* The honest answer is probably "it's low-stakes and disclosure in the privacy notice suffices"
(advice D) — but the fact that the customer's viewpoint had to be *added* by Counsel, rather than being
native to a design about recording facts about that customer, is the tell worth noticing for the next
attribution feature. The unexamined carrying assumption is that acquisition data is the owner's to collect
silently; that holds while the field is coarse, write-only, and merchant-first — and stops holding the
moment any of those three change.
