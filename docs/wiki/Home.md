# dowiz wiki

dowiz is one system for a restaurant: the guest's ordering app, the owner's console, the room app
for waiters and the counter, and the courier app, all on the venue's own address
(`<venue>.dowiz.org`). This wiki explains how to use each of them, one page per role and one page per
area of the product. It describes what is built today; planned work is in the project
[roadmap](../design/ROADMAP-2026-09-22.md).

The apps themselves speak Albanian, English and Ukrainian (the console and the room app open in
Albanian and switch language from their menu). These pages are in English.

<!-- media: a 60-second overview video, one shot per app (video lane fills this) -->

## Start here, by role

| You are | Your app | Your page |
|---|---|---|
| A guest ordering food or booking a table | the venue's address, `/` | [Guest](Guest.md) |
| The owner or manager of a venue | `/admin/` | [Owner](Owner.md) |
| A waiter or counter manager | `/room/` | [Waiter](Waiter.md) |
| Kitchen staff | `/room/` sign-in; kitchen actions go through the console's routes | [Kitchen](Kitchen.md) |
| A courier | `/courier/` | [Courier](Courier.md) |

## By area

- [Orders and delivery](Orders-and-Delivery.md): how an order moves, who moves it, how it ends.
- [Menu and stock](Menu-and-Stock.md): dishes, photos, languages, supplies and recipes.
- [Room, tables and bookings](Room-and-Bookings.md): the floor plan, table QR codes, sittings and
  rounds, reservations.
- [Payments and the till](Payments-and-Till.md): split bills, tips, currencies, the cash drawer,
  refunds.
- [Integrations](Integrations.md): Telegram, WhatsApp and Instagram, the kitchen printer, the
  nightly copy, eBills, agents over MCP.
- [Privacy and consent](Privacy-and-Consent.md): what is kept about a guest, consent, forgetting a
  person.

## Reference

- [Glossary](Glossary.md): the words dowiz uses (sitting, round, image, fold, outbox...).
- [FAQ](FAQ.md)
- [Troubleshooting](Troubleshooting.md)

## For developers

The [project README](../../README.md) and the [documentation index](../README.md) cover the
architecture, testing, code-quality rules and operations.
