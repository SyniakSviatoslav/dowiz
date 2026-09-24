# FAQ

**Does a guest need an account?**
No. A guest orders and books with a name and a phone number. The order and the booking come back
with a key held on the guest's phone.

**Does dowiz take a commission on orders?**
dowiz is the venue's own ordering app on the venue's own address. Card payments go to the venue's own
Stripe account. (Pricing for venues is set by the platform, not by this repository.)

**Is dowiz a fiscal cash register?**
No. It is not a certified fiscal device and does not issue fiscal invoices. It runs beside the
venue's certified till and imports that till's sales from eBills. The code that could send invoices
is switched off.

**Which languages?**
Albanian, English and Ukrainian, in all four apps and in the menu itself.

**Which currencies?**
Each venue trades in its own currency (Dubin & Sushi in lek). At the table a payment can be taken in
a second currency at a stated rate; the bill stays in the venue's currency.

**Can two waiters edit the same table at once?**
Yes, safely. Each change names the version it was made against; a stale change is refused and the
phone reloads the order. Nothing is overwritten.

**What happens when a courier loses signal?**
Their taps are saved on the phone and sent when the signal returns. Each carries a key, so it is done
once even if it arrives twice. See [Courier](Courier.md).

**Can I undo a delivered order?**
No. `DELIVERED` and `PICKED_UP` are final. Money is returned through a refund on an order before it
is delivered; after handover, handle it outside dowiz.

**Where is my data, and is it shared with other venues?**
Each venue's records live in that venue's own store and nowhere else. Every request is tied to one
venue by its address, and a code check refuses a handler that acts on a different venue than the one
it authorised. A nightly copy goes to a bucket the venue owns.

**Does dowiz rate couriers or customers?**
No, and it is enforced in code: there is no rating, ranking, score or tier of any person, and a check
refuses code that adds one.

**Is my data protected by post-quantum encryption?**
dowiz makes no such claim. See [Privacy and consent](Privacy-and-Consent.md#encryption-claims).

**Can an AI agent run my venue?**
An agent can connect over MCP with an API key the owner creates, and can do exactly what the console
can. See [Integrations](Integrations.md#agents-over-mcp).

**Is there a kitchen display?**
Not yet. The kitchen hears about orders through the Telegram bell, the kitchen printer, or the
console on a screen. See [Kitchen](Kitchen.md).

**Is dowiz open source?**
Yes, under the GNU Affero General Public License v3.0. The name is covered by the trademark terms in
[`TRADEMARK.md`](../../TRADEMARK.md).
