# APPLY — i18n script aliases into `package.json` (operator action)

`package.json` is a protected zone (`protect-paths.sh`). Add these two scripts to the root
`package.json` `"scripts"` block so the i18n tooling has ergonomic entry points:

```json
"i18n:add": "tsx scripts/i18n-add.ts",
"i18n:parity": "tsx scripts/i18n-parity.ts"
```

Then:

```bash
pnpm i18n:add order.refunded "Order refunded" "Porosia u rimbursua" "Кошти повернено"
pnpm i18n:parity   # CI/manual gate; also runs in .husky/pre-commit when i18n files are staged
```

Both work today without the aliases via `pnpm exec tsx scripts/i18n-{add,parity}.ts`. The pre-commit
hook already calls the script by path, so the gate is active regardless of these aliases.

**Optional CI step** (`.github/workflows/*` — also protected): run `pnpm exec tsx scripts/i18n-parity.ts`
so parity is enforced on PRs even if a local hook is bypassed.
