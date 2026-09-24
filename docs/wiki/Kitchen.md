# Kitchen

Kitchen staff have the **Kitchen** role: they can move a round through the kitchen and nothing else.
They take no money and cannot read or change the room's tables.

## How the kitchen hears about an order

- **The Telegram bell.** When the venue connects its own Telegram bot (console, **More,
  Notifications**), every new order is sent to the kitchen's chat. If Telegram is down the message
  waits and is retried; the order is never lost because a message failed.
- **The kitchen printer.** When the venue connects a printer (console, **More, Kitchen printer**),
  every new order comes out as a ticket. The printer fetches its own tickets; the console shows each
  order's ticket as queued, printing, printed or failed.
- **The console's Orders tab** on a screen in the kitchen.

## Moving an order

A kitchen member signs in through the room app (`/room/`, with the invite code from the owner). The
room app is for the floor and says so to kitchen staff; there is **no separate kitchen display
yet** (it is on the [roadmap](../design/ROADMAP-2026-09-22.md)).

The kitchen's two moves are the same ones the console's buttons make, signed as the kitchen member:

- **Seen**: the kitchen acknowledges a round (`POST /api/staff/orders/:id/kitchen-ack`).
- **Ready**: the round moves from preparing to ready.

Anything else, such as reading the room or taking a payment, is refused for the Kitchen role.

See [Orders and delivery](Orders-and-Delivery.md) for the whole order lifecycle.
