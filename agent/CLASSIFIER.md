# FIX vs IMPROVEMENT Classifier

Operational test for every finding. Run all 5 questions. Only "yes" to 1-4 AND confidence on 5 = FIX.

## Test

1. **Spec reference**: Is there a written spec that current behavior violates?
   - No → IMPROVEMENT (no baseline = opinion, not fix)

2. **Change scope**: Does the change ONLY bring behavior back to that spec, with no new behavior?
   - No → IMPROVEMENT

3. **File safety**: Are ALL touched files in unprotected zones (not migrations/shared-types/fly.toml/Dockerfile/.github/.claude)?
   - No → IMPROVEMENT

4. **Contract integrity**: Are contracts/schema/scope/scaffold/deps/infra ALL untouched?
   - No → IMPROVEMENT

5. **Confidence**: Any doubt on 1-4?
   - Yes → IMPROVEMENT (default: stop, don't risk)

## Output

- FIX → proceed to implement on fix branch → verify → open PR
- IMPROVEMENT → STOP → write `agent/proposals/<finding>.md` → wait for manual "yes"
