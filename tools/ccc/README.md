# ccc — AST-semantic code search (ADR-0012 C1, dev-only)

A lightweight symbol index + search over the repo's TypeScript/JavaScript. Lets an agent answer
"where is `X`" / "what handles `Y`" from an AST symbol map instead of a grep sweep — at a fraction
of the tokens. **Not** full CocoIndex: no pgvector, no daemon, no embeddings.

> **Dev-only. Never shipped to prod.** The index lives in `.ccc/` (gitignored); writing into `dist/`
> is refused. The walker **consults ignore rules before reading a file's bytes** (B10), so a secret
> on disk (`.env*`, keys, `.gitignore`d files) is never opened — proven by `verify:ccc-secrets`.

## Use

```bash
pnpm ccc index --root . --label "$(git rev-parse --short HEAD)"   # build .ccc/index.json
pnpm ccc search sendError                                          # rank symbols by name
pnpm ccc search buildErrorEnvelope --kind function --limit 5
```

Output: `path:line  kind  signature`, ranked exact > prefix > token > substring (exported boosted).

## Security model (B10) — why it can't leak a secret

`tools/ccc/src/ignore.ts` is the gate:
1. **Hard secret deny-list** (`.env*`, `*.pem/.key/.p12`, `id_rsa*`, `.fly*token*`, `secrets.*`, …)
   — always wins, **independent of `.gitignore`**; a `.gitignore` negation can never re-include it.
   `.env.example` is explicitly allowed (documentation, not a secret).
2. Standard vendor/build skips (`node_modules`, `dist`, `build`, `.ccc`, …).
3. Best-effort `.gitignore` glob match (defence in depth, ordered negation honored).

The walker (`indexer.ts`) calls `isIgnored(relPath)` **before** any `readFileSync`, and records the
files it actually read in `readPaths`. The merge gate asserts no secret/ignored path is in
`readPaths` — a structural proof (a never-read secret cannot leak however the index is serialized),
not just output-scanning.

## Gate

`pnpm verify:ccc-secrets` is a **merge gate** (ADR-0012): over a fixture tree (on-disk `.env`, a
`.gitignore`d secret, a private key, a `node_modules` secret) it asserts none were read or indexed,
and `.env.example` / normal source still index. C1 stays disabled until this is green.
