# Legal decisions for the privacy texts (2026-09-24)

`legal-decisions v1 2026-09-24`

**Why this file exists.** `BLUEPRINT-GDPR-AND-MCP-2026-09-24.md` §6 said nine questions needed a lawyer
before the privacy notice (P8) and the DPA (P9) could ship. On 2026-09-24 the operator lifted that gate:
"do it yourself instead of the lawyer, he gave consent". This file answers each question from the text
of the law, cites the article, and says plainly where the answer is a judgement or depends on a fact
not in the repository. It is not advice from a lawyer.

**Sources read.** Law No. 124/2024 "On Personal Data Protection", the Commissioner's official English
text (https://idp.al/wp-content/uploads/2025/04/Law-no.124-2024-DP.pdf), read directly for Art. 4, 7,
8, 12, 13, 14-20, 25, 26, 29, 33, 39-42, 46, 86, 101. The GDPR (Regulation (EU) 2016/679). Karanovic &
Partners' summary of the Commissioner's Decision no. 1 of 30 April 2025 (the decision itself was not
read). The code at the working tree of this date, cited by file.

**Texts this decides.** `privacy-notice v1 2026-09-24` (`workers/api/src/privacy/notice*`, served at
`/privacy`), `dpa v1 2026-09-24` (`docs/privacy/DPA-v1-2026-09-24.{sq,en,uk}.md`, served at `/dpa`),
`processors v1 2026-09-24` (`docs/privacy/PROCESSORS.md`).

## Corrections to the blueprint found while reading the law

- The 16-year line for consent online is **Art. 8(6)**, not Art. 7(6). The notice cites 8(6).
- The complaint right is **Art. 86** (confirmed). The Commissioner's full title in the official English
  text is "Commissioner for the Right to Information and Personal Data Protection".
- **Art. 41(2)(a): standard contractual clauses need the Commissioner's authorisation** in Albania. They
  are not, by themselves, a self-executing safeguard as under GDPR Art. 46(2)(c). Decision 4 accounts
  for this.
- **Art. 46(2)** allows direct marketing on legitimate interest. dowiz keeps the stricter rule (consent
  only); see decision 7.

## 1. Where dowiz is established; GDPR; a representative in Albania

- **Fact not in the repository:** the legal entity that operates dowiz and its country. Nothing in the
  tree names it, so the texts say "dowiz, the operator of the dowiz.org platform" and give
  `privacy@dowiz.org`.
- **Law 124 applies to the venues** (controllers established in Albania, Art. 4(1)(a)) and therefore to
  everything dowiz does for them as their processor.
- **If the operator is established outside Albania**, its processing for Albanian venues is
  continuous, not "occasional", so the Art. 25(2)(a) exemption does not fit, and **Art. 25(1) requires a
  representative located in Albania, notified to the Commissioner in writing.** This is an action for
  the operator, recorded as OPEN below; no text claims a representative exists.
- **GDPR:** if the operator is established in the EU, GDPR Art. 3(1) applies to its processing. The
  design already meets GDPR where it is stricter; where Law 124 is stricter (30 days, not one month) the
  texts use Law 124's number. Nothing in the texts claims GDPR does or does not apply.

## 2. Controller / processor split and the DPA

- **The venue is the controller** of its customers', guests', staff's and couriers' data: it decides to
  sell food, deliver, book tables and market. **dowiz is its processor** (Art. 26) and processes only on
  the venue's documented instructions — the settings the venue chooses in its console.
- **dowiz is a controller** only for owner/staff platform accounts and the waiting list. Both texts say
  so (`dowiz_role` in the notice; §1 of the DPA). The apex `dowiz.org/privacy` renders dowiz's own
  notice from the registry's platform rows.
- **The DPA** covers every item Art. 26(3) lists: subject, duration, nature, purpose, data types,
  categories of people, instructions (a), confidentiality (b), security (c, Art. 28), sub-processor
  rules (ç, Art. 26(2)/(4)), assistance with rights (d), assistance with breach duties (e, Art. 29),
  deletion or return at the end (ë), audits and information (f).
- **Sub-processors, general written authorisation (Art. 26(2)):** Cloudflare (hosting, database,
  storage, email) and Stripe (card payments through the platform's Stripe keys; Stripe also acts as an
  independent controller for its own legal duties). 30 days' notice before a change, with a right to
  object.
- **Services the venue chooses itself** (Telegram, Meta, its S3 bucket, its AI endpoint, ebills.al) are
  the venue's own recipients or processors under the venue's own contracts; dowiz transmits only what the
  venue switched on. This is stated in DPA §6 and each appears in the notice only when switched on.
- **Acceptance:** a click at hub creation (`POST /api/platform/hubs` refuses 400 without `"dpa": "dpa v1
  2026-09-24"`) and in the owner console (`POST /api/owner/dpa/accept`), stored on the venue's `loc`
  record (`dpa_version`, `dpa_accepted_at_ms`, `dpa_accepted_by`).

## 3. Retention where accounting and tax law override erasure

- **Money is kept, the person goes.** Art. 15(3)(b) exempts processing required by a legal obligation.
  The till, fiscal documents and wallet ledger are kept (`Eraser::Retain`, each with its reason in the
  registry). They carry a staff id or a pseudonymous key, not a customer's name or phone.
- **A customer's name, phone and address on an order are not part of the accounting record** and are
  redacted on erasure (`hubdo/forget.rs`), while the order's money and status stay. This is a
  judgement: the accounting record needs the amount, the date and what was sold, not who ate it.
- **How long orders keep contacts without a request:** there is **no automatic limit yet** (P6 will add
  one). Art. 13(1)(ç) allows stating "the criteria used to determine such period" when a period cannot
  be given, so the notice says "no fixed limit yet; kept while the venue keeps its records" and that
  erasure on request reaches it. That is true today; it becomes a number when P6 lands.
- **The Albanian accounting-law retention period in years was not read** and no text states one.

## 4. Transfers (Cloudflare, Meta, Stripe, Telegram)

- **Primary ground: adequacy (Art. 40)** via the Commissioner's Decision no. 1 (30 Apr 2025), which, per
  the Karanovic & Partners summary, recognises EU/EEA states, Convention 108 parties with working
  authorities, and third countries with an EU adequacy decision. For the United States the EU decision
  covers organisations certified under the EU-US Data Privacy Framework; Cloudflare, Stripe and Meta
  state such certification. **Residual uncertainty:** whether the Commissioner reads the partial US
  decision as covering the US; the decision's own text was not read. If it does not, the providers'
  SCCs need the Commissioner's authorisation under Art. 41(2)(a) — OPEN.
- **Fallback ground: Art. 41(3)(b)** — a transfer necessary to perform a contract with the data subject.
  It fits sending an order to the venue's kitchen and taking a card payment.
- **Telegram** offers no DPA and is in the UAE (not in the adequacy list as summarised). Decision: it
  stays allowed as a venue-chosen kitchen tool **only because the ticket is minimised** (first name and
  initial, the order, the address, the phone only for a delivery — `notify::ticket_contact`, P11) and
  the transfer is necessary to fulfil the order (Art. 41(3)(b)). The notice says Telegram offers no
  agreement, in those words. Recommendation to venues: prefer the console and the printer rail.
- **The United Kingdom** (OpenStreetMap Nominatim, reached from the diner's browser) has an EU adequacy
  decision.

## 5. Courier location

- **Basis:** Art. 7(1)(b), necessary to dispatch the delivery the courier accepted. Only the latest fix
  is kept (`ops` image, `Retention::Latest`).
- **DPIA:** Art. 31 enters into force two years after publication (Art. 101(2)), about 17 January 2027.
  P13 (the screening) should be done before then. Not done in this lane.
- **Worker information duty (Art. 13):** a courier-facing notice was not written in this lane — OPEN.

## 6. Wallet balance on erasure

- Decision: **keep the pseudonymous ledger and leave the balance claimable.** The ledger is keyed by the
  customer key (an HMAC of the phone). Erasure redacts the person from orders, people and consent; the
  ledger keeps money the venue owes (Art. 15(3)(b)). If the same person later orders with the same phone,
  the same key reaches the same balance — a re-link the person causes by coming back, which the notice
  covers under "kept, because a law requires it; it holds a pseudonym". A refund-first rule is the
  venue's commercial choice, not required by Law 124.

## 7. Marketing and the 16-year line

- Offers are sent **only on consent**: a separate, never pre-ticked box, recorded with its exact wording
  (`consent_log`), withdrawable at any time (Art. 8(1)-(3)). This is stricter than Art. 46(2), which
  would allow legitimate interest; kept because WhatsApp templates are unsolicited electronic messages.
- **16 years (Art. 8(6)):** the notice tells people under 16 they cannot agree themselves. The checkout
  does not ask for an age; this is a disclosure, not a verification.
- The objection right to direct marketing (Art. 46(4), Art. 19(2)) is covered by withdrawal.
- Albania's electronic-communications law was **not read** — OPEN.

## 8. Data protection officer

- Art. 33(1): required for public bodies, for core activities of large-scale regular and systematic
  monitoring, or large-scale sensitive data. Decision: **not required today** for dowiz or for a venue.
  Order processing is not monitoring; courier location is limited to active deliveries of a handful of
  venues; no sensitive categories are asked for. Revisit when the platform reaches large scale.
  `privacy@dowiz.org` is the contact point.

## 9. Wording in three languages

- The notice and the DPA are written in Albanian, English and Ukrainian by this lane, versioned, and
  served from the code. Rules applied: every sentence true of the tree on this date; the backup window is
  computed from `cloud::KEEP_WEEKLY_MS` (21 days, so a copy survives at most 22); the answer deadline is
  Art. 12(4)'s 30 days (+60 with reasons within the first 30); no claim of post-quantum encryption; no
  mention of funding or investors.
- **The breach runbook (P12) was not written in this lane** — OPEN.

## OPEN (actions this file cannot close)

1. Name the operator's legal entity and country; if outside Albania, appoint and notify an Albanian
   representative (Art. 25).
2. Make `privacy@dowiz.org` deliver: add the address in Cloudflare Email Routing for `dowiz.org` (the
   texts name it; this lane did not touch production).
3. Read the Commissioner's Decision no. 1 itself for the US reading; if needed, seek authorisation for
   SCCs (Art. 41(2)(a)).
4. Measure the live venues' Durable Object locations (P9 CHECK; needs production).
5. Courier notice (Art. 13) and the courier-location DPIA screening (P13) before ~17 Jan 2027.
6. Breach runbook (P12). Retention jobs (P6). Albania's electronic-communications and accounting-law
   retention periods.
