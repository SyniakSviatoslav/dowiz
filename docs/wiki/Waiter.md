# Waiter and counter manager: the room app

The room app is at `/room/` on the venue's address. It is made for a phone in one hand.

<!-- media: waiter opening table 7, adding a round, taking a split payment, 40 s (video lane fills this) -->

## Getting in

1. The owner invites you from the console (**More, Staff**) and gives you an invite code.
2. Open `/room/`, tap **I have an invite code**, enter it, and set your password (**Activate**).
3. Next time, sign in with your email and password.

What you can do depends on the role the owner gave you:

| Role | Orders | Payments | Voids after the kitchen has it | Till |
|---|---|---|---|---|
| Waiter | yes | yes | no | no |
| Counter manager | yes | yes | yes | yes |

## The floor

**Floor** shows the owner's floor plan with every table's state: **Free**, **Booked**, **Ordering**,
**Waiting for food**, **Paying**, **To clear**. The state is worked out from the orders and
bookings, never typed in. When a paid table has been cleaned, tap it and mark it cleared.

## Serving a table

- **Open a table**: name it (or pick it on the floor) and add the first round with **Add items**.
- A **round** is one trip to the kitchen. Add as many rounds as the table orders.
- **Change a round** before or after sending, in plain intentions: **Remove** a line, **Fewer** or
  **More**, or **Comp** it (on the house). Removing or comping asks for a reason: mistake, guest
  changed their mind, unavailable, dropped, or your own words.
- If someone else changed the same order on another phone while you were editing, your change is
  refused and the order reloads; nothing is overwritten. Check it and try again.
- **Move lines** from one round to another, as long as the kitchen does not have the round yet and
  one line stays behind.
- **Move the table**: every round still in the room moves to the new table.

### A guest ordered from the table's QR code

The round shows as **Guest order**. **Confirm** sends it to the kitchen; **Reject** refuses it. Until
then the kitchen does not see it.

## Taking payment

1. Open the table and tap **Take payment**.
2. Enter the amount (or tap **What is owed**), the method (cash, card, cheque, transfer, gift card,
   other) and the currency. For a second currency, type the rate from the board.
3. Add a **tip** if there is one.
4. Repeat until the bill reads **Paid in full**. A bill can be split into as many payments as it
   takes; paying more than is owed is refused, and a paid round can no longer be changed.

Cash can only be taken while a till is open; ask the counter manager to open one.

Refunds are made from the owner console, not the room app.

## The till (counter manager)

- **Open till** with the float, per currency.
- **Pay in** and **Pay out** move cash in or out with a reason.
- **Count the drawer** at any time. The count is blind: the expected figure is not shown until
  close.
- **Close till** shows expected, counted and the difference (over or short) per currency.
- **Tips by person** lists who took how much in tips over the till period. dowiz does not share
  tips out; it only reports them.

## No signal

If the network drops, actions are saved on the phone and sent by themselves when it returns (the
header shows how many are waiting). Each saved action carries a key, so if it arrives twice the
server answers the second with the first answer and nothing happens twice. If the order changed in
the meantime the saved action is refused, and the app says so.

See also: [Room and bookings](Room-and-Bookings.md), [Payments and the till](Payments-and-Till.md).
