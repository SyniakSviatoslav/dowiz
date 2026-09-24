# dowiz documentation

Start with the [project README](../README.md). This index lists the documents that describe the
tree as it is; `docs/` also holds a large design archive, and older files in it can be stale
(see the note at the end).

## Using dowiz

- [Wiki home](wiki/Home.md): a guide per role ([Guest](wiki/Guest.md), [Owner](wiki/Owner.md),
  [Waiter](wiki/Waiter.md), [Kitchen](wiki/Kitchen.md), [Courier](wiki/Courier.md)), per area,
  plus the [Glossary](wiki/Glossary.md), [FAQ](wiki/FAQ.md) and [Troubleshooting](wiki/Troubleshooting.md).

## Building and running dowiz

| Document | Read it for |
|---|---|
| [architecture.md](architecture.md) | the layers, one request end to end, the venue's images, the bebop format, state machines, outbox and rails, eBills, crons |
| [testing.md](testing.md) | every test layer, how to run it, what CI runs |
| [code-quality.md](code-quality.md) | each house rule and the gate that enforces it |
| [operations.md](operations.md) | deploy, production health, crons, runbooks, the development box |
| [releases/](releases/) | release notes per version |
| [github-topics.txt](github-topics.txt) | the repository topics to set on GitHub |

## Decisions and plans

| Document | Read it for |
|---|---|
| [design/ROADMAP-2026-09-22.md](design/ROADMAP-2026-09-22.md) | the current roadmap: what is true, what is next, what was decided against |
| [design/AUDIT-BUGS-BLINDSPOTS-TESTS-2026-09-24.md](design/AUDIT-BUGS-BLINDSPOTS-TESTS-2026-09-24.md) | the latest defect audit and the Wave G fix list |
| [design/BLUEPRINT-LAUNCH-GAPS-2026-09-24.md](design/BLUEPRINT-LAUNCH-GAPS-2026-09-24.md) | Wave F, what stands between today and more venues |
| [design/BLUEPRINT-GDPR-AND-MCP-2026-09-24.md](design/BLUEPRINT-GDPR-AND-MCP-2026-09-24.md) | Wave P, privacy duties and agent access |
| [design/BLUEPRINT-POS-THE-ROOM-2026-09-22.md](design/BLUEPRINT-POS-THE-ROOM-2026-09-22.md) | the room: tables, rounds, amend, pay, till |
| [design/BLUEPRINT-TAX-PRICE-CHANNEL-2026-09-22.md](design/BLUEPRINT-TAX-PRICE-CHANNEL-2026-09-22.md) | tax rates, inclusive and exclusive prices, order channel |
| [design/BLUEPRINT-CRM-CONSENT-LOYALTY-2026-09-22.md](design/BLUEPRINT-CRM-CONSENT-LOYALTY-2026-09-22.md) | the customer record, consent, erasure, the stamp card |
| [design/BLUEPRINT-EBILLS-INTEGRATION-2026-09-22.md](design/BLUEPRINT-EBILLS-INTEGRATION-2026-09-22.md) | the fiscal platform, measured |
| [design/BLUEPRINT-BEBOP-IN-WASM-2026-09-23.md](design/BLUEPRINT-BEBOP-IN-WASM-2026-09-23.md) | the image format in wasm32 and the four-reader gate |
| [../DECISIONS.md](../DECISIONS.md) | red-line rulings |
| [adr/](adr/) | architecture decision records |

## A note on older documents

`docs/` has several hundred files from earlier phases of the project, including plans for a
TypeScript and Postgres stack that no longer exists. When a document and the tree disagree, the tree
wins; `docs/design/ROADMAP.md` is kept as history, and the roadmap above says which older documents
are superseded.
