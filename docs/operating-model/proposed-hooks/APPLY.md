# Applying the §1 hook changes (needs your approval — `.claude/` is protect-paths-blocked)

These two files extend the live hooks to the 4-label classification + governance routing. They live
here as artifacts because `.claude/hooks/protect-paths.sh` hard-blocks edits to `.claude/` (exit 2) —
by design, so an agent can't silently rewrite governance. Apply them yourself (or relax protect-paths
for one approved commit):

```bash
cp docs/operating-model/proposed-hooks/require-classification.sh .claude/hooks/require-classification.sh
cp docs/operating-model/proposed-hooks/post-edit-gates.sh        .claude/hooks/post-edit-gates.sh
chmod +x .claude/hooks/*.sh
```

## What changes vs the current hooks
- **require-classification.sh** (Stop hook): the CHANGE-MANIFEST must now carry a
  `CLASSIFICATION: spike | build | audit | challenge` line (was: manifest presence only). Also
  watches `spikes/` so recon work is labeled too.
- **post-edit-gates.sh** (PostToolUse on edit): reads the classification and ROUTES —
  - `spike`/`challenge` → red-line grep + boundary check only (relaxed; throwaway ok).
  - `build`/`audit` → red lines + boundary + full `lint:gates`.
  - Default = `build` (full discipline) when no manifest, so the safe path is never the relaxed one.
  - Boundary: a `spike` edit outside `spikes/` fails; any apps/packages import from `spikes/` fails
    (delegates to `scripts/guardrail-spike-boundary.mjs`, already in `verify:all`).

## settings.json
`post-edit-gates.sh` must receive the tool input on stdin (it parses `file_path`). The existing
PostToolUse wiring already pipes the hook payload; no settings change needed unless the matcher is
narrowed. Verify with a throwaway `spike`-labeled edit outside `spikes/` → expect the BOUNDARY block.

## Manifest
Use `agent/CHANGE-MANIFEST.template.md` as the shape. One `CLASSIFICATION:` line is the load-bearing
addition.
