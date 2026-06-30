# Crypto-first payments — non-custodial provider finalists (operator decision input)

Operator pivot (2026-06-30): **crypto-first, non-custodial** (sidesteps the Albania card-acquiring gap — no
bank contract, no MSB/KYC on us), **stablecoin-only**, **accept both USDT + USDC** (no volatile coins).
This narrows the field; "research 2-3 finalists deeper" → below. Provider choice gates the resolve round.

## Why non-custodial narrows it hard
Only a gateway that routes funds **directly to our wallet** keeps us out of MSB/KYC scope. Most "no-KYC"
gateways are still *custodial* (funds pass through their wallet) — that's a different thing. Genuinely
non-custodial: **BTCPay, Plisio**. NOWPayments is custodial-by-default (non-custodial only if explicitly
flipped); 0xProcessing is a licensed VASP with full AML/KYT. OxaPay is no-KYC but its custody model needs a
direct check before we trust the "non-custodial" label.

## Finalists
| | **BTCPay Server** | **Plisio** | **OxaPay** |
|---|---|---|---|
| Custody | **Truly non-custodial** (you run it, you hold keys) | **Non-custodial** (direct-to-wallet) | No-KYC, **custody model unclear — VERIFY** |
| KYC | **None, ever** | Light-KYC past a volume threshold | **None** (email-only signup) |
| Fee | **0%** (you pay hosting + network) | 0.5% mono / 1% multi | **0.4%** (lowest hosted) |
| USDT/USDC | USDT-TRON + USDC via plugins (BTC/LN native) | USDT (Tron/ETH/BSC/TON/Sol) + USDC (ETH/Base/Sol) | USDT/USDC/BTC/… |
| API/webhook | Greenfield (Greenfield API + webhooks), self-run | Mature REST + webhooks, many CMS plugins | Simple REST + callbacks (less comprehensive) |
| Ops burden | **High** — VPS, node sync, key mgmt, updates, own off-ramp | **Low** — hosted | **Low** — hosted |
| Confirmation | 1–2 confs (~2–5 min) | 1–2 confs (~2–5 min) | 1–2 confs (~2–5 min) |
| Best for | Max sovereignty / zero-fee / no-KYC, ops-tolerant | **Balanced**: non-custodial + low ops + clean webhooks | Cheapest + easiest signup (pending custody check) |

Sources: [Plisio field guide](https://plisio.net/crypto/crypto-payment-gateway),
[BTCPay USDt plugin](https://github.com/btcpayserver-tether/BTCPayServer.Plugins.USDt/),
[OxaPay review](https://payyd.co/blog/oxapay-review),
[gateway comparison](https://eco.com/support/en/articles/15083177-best-crypto-payment-gateways-2026).

## Recommendation
**Plisio** for v1: genuinely non-custodial (funds → our wallet), hosted (no node/key ops), both USDT + USDC,
0.5%, mature signature-verified **webhook** that fits the council's webhook-as-source-of-truth design with the
least build risk. **BTCPay** if full sovereignty / zero-fee / no-KYC is worth the ops + key-custody
responsibility. **OxaPay** only if its non-custodial claim checks out (cheapest + frictionless signup).

## The off-ramp is a separate operator reality (NEEDS-HUMAN, gateway-independent)
How the merchant turns received stablecoin into spendable ALL/EUR:
- **USDT-TRC20** → best for **Albania/Balkans via Binance P2P** (~0.5–1.5% spread); widely held; low network fee.
- **USDC** → cleaner for EU-regulated bank settlement, BUT **Albania isn't EU/SEPA**, so less directly useful;
  and under **MiCA, USDT is delisted on EU-regulated exchanges** (matters only if cashing out via the EU).
- Practical Albania path: **Binance P2P (USDT)** or a regional exchange. Accepting **both** (operator's choice)
  hedges: USDT for P2P liquidity, USDC for any regulated rail. This is the merchant's treasury op, outside our
  code.

## What still gates the build (after provider pick)
- The **resolve round** (operator chose to wait for the provider) then bakes in: C1 prepaid completion
  (`delivered_prepaid` outcome, no cash-proof/`hold`), C2 refund trigger (crypto = manual/owner-review per the
  irreversibility), C3 webhook RLS (`WITH CHECK` on `app.current_tenant` from the order's location), + the
  crypto specifics (await-confirmation state, under/over/late payment, reorg M5, depeg L2, **wallet-key custody
  L3** — the one real risk of self-managed non-custodial).
- Counsel STOP (still live): **no crypto launch without honest irreversibility disclosure + a written refund
  SLA** at checkout.
