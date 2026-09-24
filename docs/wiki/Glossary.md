# Glossary

**Venue.** One restaurant, with its own address (`<slug>.dowiz.org`), its own records and its own
integrations. Also called a hub in the code.

**Slug.** The short name in a venue's address, for example `sushi-durres`.

**Platform.** `dowiz.org`: the landing page, the waiting list and the creation of new venues.

**Storefront, console, room app, courier app.** The four apps: guests at `/`, the owner at
`/admin/`, waiters and the counter at `/room/`, couriers at `/courier/`.

**Role / preset.** A fixed set of permissions a staff member is given: Kitchen, Waiter, Counter
manager, Owner. Each is a signed list of capabilities (advance, take orders, take payment, void,
open the till), not a level or a score.

**Order.** One purchase: from the storefront, a table QR code, a waiter or an aggregator. Its status
is worked out from its events.

**Status.** Where an order is in its life: `PENDING`, `CONFIRMED`, `PREPARING`, `READY`,
`IN_DELIVERY`, `DELIVERED`, `PICKED_UP`, `REJECTED`, `CANCELLED`, `REFUNDING`,
`COMPENSATED_REFUND`. See [Orders and delivery](Orders-and-Delivery.md).

**Sitting.** One party at one table, from opening the table to clearing it.

**Round.** One order sent to the kitchen during a sitting. A sitting's bill is the sum of its rounds.

**Amend.** A change to a round, made as an intention (add, remove, change a count, comp) against the
version the waiter was looking at.

**Comp.** A line given on the house. It needs a reason and shows on the exceptions report.

**Void.** Removing a line after it was ordered. It needs a reason; after the kitchen has the round it
needs the Void permission.

**Till / till period.** The cash drawer, from open to close, with its float, cash in and out, blind
count and over or short.

**Minor units.** The smallest unit of a currency (whole lek for ALL, cents for EUR). Every amount in
dowiz is an integer number of them.

**Event.** One thing that happened (an order was placed, paid, advanced), appended to a log and never
changed.

**Fold.** Working out the current state by replaying the events in order. dowiz stores events and
folds them; it does not store a status that could disagree with its own history.

**Image.** One of a venue's record files (the order log, the catalogue, bookings, the outbox...). An
image is append-only and each record is chained to the previous one by a hash. The format is called
bebop.

**Durable Object.** The Cloudflare component that holds one venue's images and is the only writer
for that venue.

**Outbox.** The queue of messages a venue owes (Telegram, WhatsApp, campaigns, printer tickets),
written together with the event that caused them and sent afterwards with retries.

**Rail.** One outside service a message or payment travels on: Telegram, WhatsApp, Stripe, the
printer, the venue's AI endpoint.

**Idempotency key.** A random key an app attaches to a write. If the same write arrives twice (a
retry after a lost connection), the server answers the second time with the first answer and does
nothing again.

**Consent fold.** The current answer to "may we message this person on this channel", worked out
from the consent log.

**Forget.** Erasing a person in place: their fields emptied, the records kept so the accounts still
add up, and one `Forgotten` event declaring it.

**Gate.** A script in `tools/gates/` that checks one code rule across the whole tree and fails the
build when it is broken. Each has a proof script that shows it can fail.

**Kernel.** The Rust code (`crates/dowiz-core`) that decides every order transition and every amount.

**eBills.** Albania's fiscal invoicing platform (ebills.al), from which dowiz imports a venue's till
sales.

**MCP.** Model Context Protocol, the way an AI agent connects to a venue at `/api/mcp`.
