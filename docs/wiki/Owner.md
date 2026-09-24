# Owner: the console

The console is at `/admin/` on the venue's address. Sign in with the owner's email and password. It
works on a phone and on a desktop, opens in Albanian, and switches to English or Ukrainian from its
menu.

<!-- media: console tour, one shot per tab, 45 s (video lane fills this) -->

## The header

The chip in the header is the venue's state: **open**, **busy** or **closed**. Guests see it, and a
closed venue takes no orders. A new order rings twice where the browser allows sound.

## The five tabs

### Orders

Every live order (`PENDING`, `CONFIRMED`, `PREPARING`, `READY`, `IN_DELIVERY`) with its lines,
customer and payment.

- **Accept** or **reject** a new order. Only a `PENDING` order can be rejected.
- **Advance** it through the kitchen: preparing, ready, then out for delivery or picked up.
- **Assign a courier** to a delivery that is ready.
- **Refund** an order that is past `PENDING`: it moves to `REFUNDING` and then to
  `COMPENSATED_REFUND` when the money back is recorded.
- See each order's **kitchen ticket**: queued, printing, printed (with the time) or failed, when a
  kitchen printer is connected.
- An order taken on an aggregator such as Wolt can be entered by hand; entering it twice places it
  once.

What each status means and which moves are allowed: [Orders and delivery](Orders-and-Delivery.md).

### Menu

Dishes and categories, prices, photos, names and descriptions per language, allergens, options
(modifiers) and availability. A menu can be imported in bulk. See [Menu and stock](Menu-and-Stock.md).

### Stock

Supplies, recipes, purchases, write-offs (each names who signed it) and the waste report. The stock
ledger is built and tested but does nothing on a venue until that venue enters its supplies and
recipes. See [Menu and stock](Menu-and-Stock.md).

### Couriers

Invite couriers, activate and suspend them, and see who is on shift.

### More

Everything that is not the day's service, as tiles in groups:

| Group | Tiles |
|---|---|
| Messages | the WhatsApp and Instagram inbox |
| Room | **Bookings** (the day: confirm, seat, cancel, book by phone), **Floor plan** (rooms and tables) |
| Marketing | **Promo codes** (and the stamp card), **Posts**, **Campaigns** (WhatsApp, to consented customers only), **Social media** |
| Analytics | **Analytics** (7 and 30 days, in the venue's time zone), **Customers** (masked by default), **Staff**, **Exceptions** (voids, comps, refunds, till gaps, each with who signed it) |
| Settings | **Integrations** (prove each connection), **eBills** (dine-in sales from the fiscal till), **Kitchen printer**, **Table QR codes**, **Preview** (as a customer sees it), **Venue**, **Hours**, **Delivery** (fee, minimum), **Payments** (card, crypto wallets), **Notifications** (Telegram, WhatsApp), **Order channels**, **Agents (MCP)**, **Cloud storage** (a copy every night), **Brand** (colours, seal), **Features** (on/off), **Assistant**, **API keys**, **Activation** (ready to work?), **System health**, **Data agreement** |

## Staff

**More, Staff** invites a person and gives them a role. Each role is a fixed set of permissions; a
person is never given a score or a rank.

| Role | Can |
|---|---|
| Kitchen | move rounds through the kitchen; takes no money |
| Waiter | take orders and payments, void before the kitchen has a round; no till |
| Counter manager | everything a waiter can, voids after the kitchen, and the till |
| Owner | everything |

The invite gives the person a code; they claim it in the room app (`/room/`) and set their password.
Couriers are invited from the **Couriers** tab instead and use the courier app.

## Customers

A customer's card shows only what the order history cannot tell by itself: notes, consent, linked
phone numbers. Contact details are masked; revealing one is logged. From the card you can record or
withdraw consent, link two spellings of one person's phone number (without merging their records),
and **forget** the person. See [Privacy and consent](Privacy-and-Consent.md).

## Health and backups

**More, System health** shows each of the venue's stores against its real size limit, the queue of
messages still to send (depth and oldest entry), and recent errors. **More, Cloud storage** connects
an S3-compatible bucket the venue owns; a copy goes there every night, and one can be sent at
once. **System health** also downloads the venue's data as one file. Restoring such a file is an
API call (`POST /api/owner/restore`), described in [operations](../operations.md#restore-a-venue).

## Opening a new venue

A venue is created by the platform operator; see [operations](../operations.md#a-new-venue). Then
work through **More, Activation**: it lists what the venue still needs before it can take orders.
