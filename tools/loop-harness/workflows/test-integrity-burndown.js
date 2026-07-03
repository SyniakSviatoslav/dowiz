export const meta = {
  name: 'test-integrity-burndown',
  description: 'Fix the 4 test-integrity lint violations across all flagged files (one agent per file)',
  phases: [{ title: 'Harden', detail: 'one agent per file fixes its truthy-id/swallowed-catch/permissive-status/tautology violations' }],
};

const FILES = ["e2e/tests/flow-ui-proof-comprehensive.spec.ts","e2e/tests/flow-core-lifecycles.spec.ts","e2e/tests/flow-regulatory-settlements.spec.ts","e2e/tests/flow-ui-order-lifecycle.spec.ts","e2e/tests/flow-order-creation.spec.ts","e2e/tests/capture-states.spec.ts","e2e/tests/flow-ui-validation.spec.ts","e2e/tests/admin/promotions.spec.ts","e2e/tests/telegram-full-flow.spec.ts","e2e/lifecycle-e2e/critical-lifecycle.spec.ts","e2e/tests/api-real.spec.ts","e2e/tests/cr8-order-messages.spec.ts","e2e/tests/deploy-validation.spec.ts","e2e/tests/flow-ingredients.spec.ts","e2e/tests/flow-orders-checkout.spec.ts","e2e/tests/flow-proofs.spec.ts","e2e/tests/flow-ui-admin-dashboard.spec.ts","e2e/tests/flow-ui-courier-core.spec.ts","apps/api/tests/p0-privacy.test.ts","e2e/tests/capture-delivery.spec.ts","e2e/tests/flow-modifiers-promotions.spec.ts","e2e/tests/flow-ui-admin-branding.spec.ts","e2e/tests/flow-ui-courier-full.spec.ts","e2e/tests/flow-ui-owner-crud.spec.ts","e2e/tests/owner-fixes-batch.spec.ts","e2e/tests/seed.spec.ts","apps/api/tests/phase5/integrity.test.ts","e2e/driver/agent-driver.ts","e2e/tests/client/modifier-display-type.spec.ts","e2e/tests/flow-admin-deep.spec.ts","e2e/tests/flow-security-regression-2026-06.spec.ts","e2e/tests/flow-ui-admin-product-bom.spec.ts","e2e/tests/flow-ui-courier-actions.spec.ts","e2e/tests/flow-ui-images.spec.ts","e2e/tests/flow-ui-invite-onboarding.spec.ts","e2e/tests/flow-ui-menu-interactions.spec.ts","e2e/tests/flow-ui-owner-core.spec.ts","e2e/tests/groq-import-proof.spec.ts","e2e/tests/non-pixel-sweep.spec.ts","e2e/tests/prod-smoke.spec.ts","e2e/tests/real-notifications.spec.ts","e2e/tests/ui-improvements.spec.ts","e2e/tests/ws-courier-assignment.spec.ts","e2e/visual/harness.ts","e2e/visual/owner-path.visual.spec.ts","e2e/tests/a11y/storefront.a11y.spec.ts","e2e/tests/behavioral-proof.spec.ts","e2e/tests/capture-screens.spec.ts","e2e/tests/client/order-stepper.spec.ts","e2e/tests/client/venue-state.spec.ts","e2e/tests/courier/offer-timer.spec.ts","e2e/tests/debug-order.spec.ts","e2e/tests/fix-validation.spec.ts","e2e/tests/flow-client-product-images.spec.ts","e2e/tests/flow-courier-deep.spec.ts","e2e/tests/flow-customer-track-link.spec.ts","e2e/tests/flow-offline-phone-fallback.spec.ts","e2e/tests/flow-sensor-delivery-baseline.spec.ts","e2e/tests/flow-sensor-funnel.spec.ts","e2e/tests/flow-sensor-geofence.spec.ts","e2e/tests/flow-ui-admin-menumanager.spec.ts","e2e/tests/flow-ui-admin-settings.spec.ts","e2e/tests/flow-ui-admin-supply-library.spec.ts","e2e/tests/flow-ui-analytics-supplies.spec.ts","e2e/tests/flow-ui-client-checkout.spec.ts","e2e/tests/flow-ui-courier-invite.spec.ts","e2e/tests/flow-ui-order-status.spec.ts","e2e/tests/flow-ui-settings-promotions.spec.ts","e2e/tests/notif-categories.spec.ts","e2e/tests/notification-events.spec.ts","e2e/tests/notification-helper-test.spec.ts","e2e/tests/onboarding-e2e.spec.ts","e2e/tests/onboarding-wizard-retired.spec.ts","e2e/tests/order-created-notification.spec.ts","e2e/tests/quick-order-test.spec.ts","e2e/tests/recent-changes-validation.spec.ts","e2e/tests/rsi-round.spec.ts","e2e/tests/simple-auth.spec.ts","e2e/tests/telegram-webhook.spec.ts"];
log(`burning down test-integrity debt across ${FILES.length} files`);

const SCHEMA = {
  type: 'object', additionalProperties: false, required: ['file', 'remaining', 'summary'],
  properties: {
    file: { type: 'string' },
    before: { type: 'number' },
    remaining: { type: 'number', description: 'test-integrity violations left after the fix (target 0)' },
    summary: { type: 'string', description: 'one line: what was changed' },
  },
};

const prompt = (f) => `Harden the test file \`/root/dowiz/${f}\` by fixing ONLY its test-integrity lint violations — minimal edits, NEVER weaken.

1. Run: npx eslint ${f}  — note the violations from these rules:
   no-truthy-on-identifier · no-swallowed-catch · no-permissive-status-assertion · no-tautological-assertion
2. Fix each:
   • no-truthy-on-identifier — \`expect(<token/id/url>).toBeTruthy()/.toBeDefined()\` → a SHAPE assertion.
     If the file is under e2e/, import { expectJwt, expectUuid } from the correct relative path to
     e2e/helpers/assert-shape (e2e/tests/X.spec.ts → '../helpers/assert-shape'; e2e/tests/sub/X → '../../helpers/assert-shape';
     e2e/lifecycle-e2e/X → '../helpers/assert-shape'). expectJwt for *token/jwt, expectUuid for *id.
     For a URL or other value, assert an exact value or \`expect(String(x)).toMatch(/.../)\`. If the file is
     NOT under e2e/ (e.g. apps/api/tests), do NOT import the helper — use \`expect(String(x)).toMatch(/regex/)\` inline.
   • no-swallowed-catch — a \`.catch(() => {})\` empty handler. PREFER removing the .catch so a real failure
     surfaces; if the tolerance is intentional, add a real statement + reason: \`.catch((e) => { void e; /* tolerated: <why> */ })\`.
   • no-permissive-status-assertion — \`expect([...incl 4xx/5xx...]).toContain(x)\` accepts an error as pass.
     Grep the route the test calls under apps/api/src/routes to find the EXACT status, then \`expect(x).toBe(<exact>)\`.
   • no-tautological-assertion — \`expect(true)\`/\`assert.ok(true)\` → assert the real thing the test claims to check.
3. HARD RULES: never add .skip/.only/.fixme/expect(true); never remove a real assertion; keep behaviour; minimal diff.
4. Verify: run \`cd /root/dowiz && npx eslint ${f} 2>/dev/null | grep -cE "no-truthy-on-id|no-swallowed-catch|no-permissive|no-tautolog"\` — the count MUST be 0.
Return { file: "${f}", before: <initial count>, remaining: <final count, must be 0>, summary: "<what you changed>" }.`;

const results = await parallel(FILES.map((f) => () =>
  agent(prompt(f), { label: `fix:${f}`, phase: 'Harden', schema: SCHEMA, effort: 'medium' })
    .then((r) => r || { file: f, remaining: -1, summary: 'agent returned null' })
    .catch(() => ({ file: f, remaining: -1, summary: 'agent errored' }))));

const ok = results.filter((r) => r && r.remaining === 0).length;
const left = results.filter((r) => r && r.remaining > 0);
const failed = results.filter((r) => r && r.remaining < 0);
log(`burndown: ${ok}/${FILES.length} files clean · ${left.length} partial · ${failed.length} errored`);
return { total: FILES.length, clean: ok, partial: left, errored: failed.map((f) => f.file), results };
