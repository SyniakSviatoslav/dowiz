# Third-party license exceptions (G5(a) — tooling-integration-eval)

> `scripts/guardrail-license.mjs` denies any third-party production dependency whose SPDX license is
> `AGPL-*` / `GPL-2.0*` / `GPL-3.0*` (LGPL is **not** denied), and flags any third-party dep with a
> **missing/unparseable** license. A missing-license dep is cleared ONLY by an explicit reviewed row
> below (`pkg@version` + reason). AGPL/GPL is **never** clearable here — it must be removed or fenced
> as an out-of-tree HTTP sidecar (e.g. Skyvern), absent from the dependency closure and never imported.
>
> First-party `private` workspace packages (no `license` field) are exempt by construction and never
> appear here.

| pkg@version | license-as-declared | reason cleared | reviewer | date |
|---|---|---|---|---|
| @mapbox/jsonlint-lines-primitives@2.0.2 | (no `license` field) | upstream is MIT (Mapbox; LICENSE file ships MIT) — transitive, no copyleft | tooling-integration-eval | 2026-06-29 |
| @tabler/icons-webfont@3.31.0 | (no `license` field) | upstream is MIT (tabler/tabler-icons) — webfont assets, no copyleft | tooling-integration-eval | 2026-06-29 |
| sylvester@0.0.21 | (no `license` field) | upstream is MIT (James Coglan) — transitive math lib, no copyleft | tooling-integration-eval | 2026-06-29 |

LAST-REVIEWED: 2026-06-29 — 0 copyleft third-party deps; 3 missing-`license`-field deps reviewed (all upstream-MIT) above.
