# VSA codec bench — 2026-07-05T12:35:47.791Z

Tokenizer: js-tiktoken cl100k_base. "min" = minified JSON (the honest baseline — pretty-printing is free to remove); "frame" = VSA1.

| payload | raw tok | min tok | frame tok | save vs min | lossless |
|---|---|---|---|---|---|
| cutover-flags-state.json | 379 | 370 | 213 | 42.4% | ✅ |
| location-info.json | 419 | 419 | 384 | 8.4% | ✅ |
| owner-products-list.json | 33928 | 33928 | 21068 | 37.9% | ✅ |
| public-menu-demo.json | 20063 | 20063 | 14347 | 28.5% | ✅ |
| **TOTAL** | | **54780** | **36012** | **34.3%** | |
