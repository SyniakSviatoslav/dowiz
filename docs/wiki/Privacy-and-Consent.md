# Privacy and consent

Each venue is the controller of its guests' data; dowiz processes it on the venue's behalf (the
console's **More, Data agreement** holds the processing agreement the venue accepts). Each venue's
records live in that venue's own store, separate from every other venue's.

## What is kept about a guest

- **Orders**: what was ordered, when, the delivery address and phone number given for that order,
  and how it was paid. Card numbers never reach dowiz; Stripe holds them.
- **A customer card** (console, **More, Customers**) that holds only what the orders cannot tell by
  themselves: the venue's notes, consent, and linked phone numbers. Everything else (how often, how
  much) is worked out from the orders when asked.
- **Consent** to marketing, as an append-only log.
- **Bookings**: name, phone, time and party size.

Nobody in dowiz is scored, rated, ranked or put in a tier: not guests, not couriers, not staff.
Order feedback is a sentence to the venue, never a number.

The venue's privacy notice is served at `https://<venue>.dowiz.org/privacy` and linked from the
storefront.

## Consent

- A guest is asked, at checkout, whether they want to hear about offers. The answer is recorded with
  the channel, the time and **the exact wording they agreed to**.
- Withdrawing consent is a new record, not a deletion of the old one, so the history of what was
  agreed stays provable.
- A campaign can only be sent to someone whose consent fold says yes, and the fold is asked again at
  the moment each message is sent. A code check (`tools/gates/consent.sh`) refuses a send path that
  skips it.

## Masked contact details

Phone numbers are masked in the console. Revealing one is itself logged (who looked, and when), and
the log can be read back.

## One person, two spellings

`+355 69 123 4567` and `069 123 4567` are the same Albanian phone. dowiz links the two spellings as
aliases of one person without rewriting either record. A number from another country is never
rewritten.

## Forgetting a person

From the customer's card, **forget** removes the person in place:

1. Their card and every id it was filed under are removed.
2. Their consent records are redacted and every channel is stopped.
3. In the order log, the person's fields are emptied in place in every order, including archived
   ones. Each order keeps its id and amounts so the venue's accounts still add up, and one
   `Forgotten` event declares the erasure, so the log's chain of hashes still verifies.
4. Their bookings are erased too.

Erasures are kept in the venue's erasure register, so they can be applied again if older data is ever
restored. A restore made through dowiz re-applies them by itself; after a Cloudflare point-in-time
recovery the owner calls `POST /api/owner/customers/reforget` (see
[operations](../operations.md#point-in-time-recovery-pitr-of-a-venue-object)).

## Backups

The nightly copy goes to the venue's own bucket. Copies are kept for 21 days at most, so a copy made
before an erasure is gone within 21 days of it.

## Encryption claims

dowiz does not claim post-quantum encryption of anyone's data. The kernel contains an ML-DSA-65
signature implementation verified against NIST's test vectors, which is not on the live request
path; a sealing path for the nightly copies exists and is switched off.
