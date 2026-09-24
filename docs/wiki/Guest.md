# Guest: ordering and booking

The storefront is the venue's own address, for example `https://sushi-durres.dowiz.org`. There is
nothing to install and no account to make. On a phone it can be added to the home screen and then
opens like an app.

<!-- media: phone recording, menu to tracking, 20 s (video lane fills this) -->

## The six tabs

| Tab | What it does |
|---|---|
| **Menu** | the venue's dishes by category, with photos, allergens and options |
| **Table** | book a table |
| **Search** | find a dish by its name or description |
| **Cart** | what you are about to order, and checkout |
| **Orders** | the orders placed from this phone, and live tracking |
| **Info** | the venue's hours for the week, address, map and phone |

The language (Albanian, English, Ukrainian) is chosen in the header and remembered on the phone.

## Ordering

1. Add dishes from **Menu** or **Search**. A dish that is sold out cannot be added.
2. Open **Cart**, then **Checkout**. Choose **delivery** or, where the venue offers it,
   **pickup**. For delivery, give the address (pick it on the map or type it) and a phone number.
   (Ordering at the table is below.)
3. The total and the expected waiting time are shown before you send. Both come from the server:
   the price from the venue's menu, the wait from the venue's own kitchen and its current queue, in
   whole minutes. What you see is what you are charged.
4. Choose how to pay. What is offered is what the venue has set up:
   - **cash**, always;
   - **card, Apple Pay, Google Pay** when the venue has connected Stripe (the card goes from your
     browser to Stripe; the venue's server never sees the number);
   - **crypto** when the venue has published a wallet address: you pay after ordering, to the
     address shown.
5. If the venue asks, you can agree to hear about offers. This is optional, the wording you agreed
   to is recorded, and you can withdraw at any time.

After sending, the order appears under **Orders** with its status (see
[Orders and delivery](Orders-and-Delivery.md)), a map for deliveries, and the waiting time. The order
can only be opened from the phone that placed it: the link carries a key, and the order number
alone opens nothing.

You can send the venue a sentence about an order (for example "the rice was cold"). There are no
stars and no scores.

If the venue runs a **stamp card**, the order shows your stamps, and a full card takes money off the
next order.

## Ordering from the table

Each table can carry a QR code. Scanning it opens the storefront for that table. The round you send
joins the table's open bill and waits for a waiter to confirm it before it goes to the kitchen. The
screen shows the table's line and the bill so far.

## Booking a table

1. Open **Table**, choose the day, the time, the number of guests and, where the venue has
   several rooms, the room.
2. Leave a name and a phone number. No account is needed.
3. The booking is a **request** until the venue confirms it. The page gives you a **share link**:
   open it on any phone to see the booking or cancel it.
4. Once confirmed, the booking has a pass the venue can check at the door.

The steps a booking goes through are in [Room and bookings](Room-and-Bookings.md).

## Privacy

What is kept about you, and how to have it removed, is in
[Privacy and consent](Privacy-and-Consent.md) and in the venue's privacy notice, linked from the
storefront.
