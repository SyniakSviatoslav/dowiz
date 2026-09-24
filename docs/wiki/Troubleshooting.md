# Troubleshooting

For operators and developers, the deeper runbooks are in [operations](../operations.md).

## Guests

**The storefront says the venue is closed.**
The venue is closed when the owner set it to closed or paused it, or when it is outside the hours set
under **More, Hours**. Check both in the console.

**A dish cannot be added to the cart.**
It is marked sold out. The owner changes availability in the **Menu** tab.

**Card payment is not offered.**
The venue has not connected Stripe (**More, Payments**). Check it with **More, Integrations, prove
it**.

**An order link opens nothing on another phone.**
By design: an order opens only with the key held by the phone that placed it. Bookings are different:
their share link works on any phone.

## Owner

**The kitchen is not being told about new orders.**
1. **More, System health**: look at the message queue. A growing queue with an old first entry
   means sending is failing; entries waiting with no failures mean the rail is not configured.
2. **More, Integrations**: run the Telegram check. A bot token that was changed or revoked shows
   here, in Telegram's own words.
3. Failed messages are retried for about half an hour (six tries) and then reported on the health
   pane. The orders themselves are never lost.

**A campaign did not reach some customers.**
Only customers whose consent is currently yes receive it, and consent is checked again when each
message is sent. Someone who withdrew in between is skipped.

**Stock does not move when orders come in.**
Stock only moves for dishes that have a recipe. Enter supplies and recipes (the **Stock** tab, or
the spreadsheet import).

**An order is stuck and cannot be ended.**
An order past `PENDING` ends through a refund. `DELIVERED` and `PICKED_UP` are final.

**The console shows "This order changed while you were editing".**
Someone else changed it first (another phone, the courier, the kitchen). The order was reloaded;
check it and try again.

## Waiters

**Cash payment is refused.**
No till is open. A counter manager or the owner opens it under **Till**.

**"The kitchen already has this round; its lines no longer move."**
Lines can move between rounds only before the kitchen has the round.

**"This round is paid; it cannot be changed."**
A settled bill cannot be amended. Money comes off a paid bill only as a refund, from the console.

**The header shows actions waiting.**
The phone lost the network; the actions are saved and will send themselves. If one is refused when it
arrives (because the order changed meanwhile), the app says so.

## Couriers

**No orders come in.**
Start a shift. Off shift, nothing is offered.

**"You are offline".**
The last known job stays on screen, and taps are saved and sent later. Keep working; check the count
of unsent actions in the header once back in coverage.

**GPS paused.**
The phone paused location while the app was in the background. Bring the app to the front.

**Sign-in fails at a second venue.**
A courier account belongs to one venue's address. Use the invite code from the second venue.

## Pages look broken

**A page renders without styles, or a screen is blank.**
Reload once. If it persists, open the browser console: a Content Security Policy error from the
venue's own address is a platform bug; report it with a screenshot (see
[CONTRIBUTING](../../CONTRIBUTING.md)).
