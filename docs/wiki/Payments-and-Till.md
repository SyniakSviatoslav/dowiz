# Payments and the till

## Money is whole numbers

Every amount in dowiz is an integer in the currency's smallest unit, and VAT rates are parts per
million. No amount ever passes through a floating-point number, so a bill adds up the same way on
every phone and on the server. A code check (`tools/gates/float-money.sh`) refuses a change that
would put a float near money.

## Online payments (storefront)

What a guest can pay with is what the venue has set up (console, **More, Payments**):

| Method | When it is offered | How it works |
|---|---|---|
| Cash | always | paid to the courier or at pickup |
| Card, Apple Pay, Google Pay | when the venue has connected its Stripe account | the card goes from the browser to Stripe; the venue's server never sees a card number |
| Crypto | when the venue has published a wallet address | the guest pays after ordering, to the address shown |

## Paying at the table (room app)

- A table's bill can be split into **as many payments as it takes**. Each payment has an amount, a
  method (cash, card, cheque, transfer, gift card, other) and a currency.
- **Two currencies.** The bill stays in the order's currency. A payment in another currency states
  the rate from the board by the till ("1 EUR = 97.50 ALL"), and the drawer counts the note in the
  currency it was handed over in.
- **Paying more than is owed is refused.** When payments cover the bill, it is settled and can no
  longer be changed; taking money off a settled bill is a refund.
- **Cash needs an open till.** A cash payment with no till open is refused, and the payment is
  stamped with the open till; the phone does not choose the drawer.
- A **tip** is recorded with the payment and belongs to the person who took it.

## The till

A counter manager (or the owner) runs the cash drawer from the room app's **Till**:

1. **Open** with a float, per currency.
2. **Pay in** and **pay out** during the day, each with a reason.
3. **Count** the drawer blind: the expected figure is hidden until close.
4. **Close**: expected, counted and over or short, per currency.

**Tips by person** reports the tips each person took over the till period. dowiz reports tips; it
does not share them out.

## Refunds

An order past `PENDING` is refunded from the console: it moves to `REFUNDING`, and to
`COMPENSATED_REFUND` when the money back is recorded. A courier's "refused at the door" starts the
same path. A refund that would not exactly reverse what it names is refused.

## Exceptions report

Console, **More, Exceptions** lists every void, comp, refund and till gap, each with the person who
signed it. It exists so an owner can see where money left, not to rank anyone.

## Checking the register

`e2e/gates/conservation.mjs` reads a venue's records through the API (read-only, with the owner's
credentials) and checks a set of conservation laws: payments against bills, refunds against what
they reverse, cash against the till, and so on. See [testing](../testing.md).

## Fiscal receipts

dowiz is not a certified fiscal device and does not fiscalise sales. It runs beside the venue's
certified till and imports that till's sales from ebills.al (see [Integrations](Integrations.md)).
The code that could issue invoices exists but is switched off (`SEND_ENABLED = false`).
