export const meta = {
  name: 'test-hardening-attack',
  description: 'Sweep ALL test files with a combined critique+security+QA lens → ranked blind-spot ledger',
  whenToUse: 'Find false-greens / weak assertions / coverage gaps across the whole test surface before hardening',
  phases: [
    { title: 'Scan', detail: 'one combined-lens agent per test file, in parallel (capped)' },
    { title: 'Consolidate', detail: 'in-script counts + an agent synthesis over the HIGH/CRITICAL findings' },
  ],
};

// args.specs = the full list of test files to scan (passed by the lead; do not hardcode).
const ALL_SPECS = ["apps/api/e2e/api-integrity.spec.ts","apps/api/e2e/phase2.spec.ts","apps/api/tests/access-gate-copy.test.ts","apps/api/tests/access-requests.test.ts","apps/api/tests/access-request-workers.test.ts","apps/api/tests/ai-ocr-parser.test.ts","apps/api/tests/auth-refresh-race.test.ts","apps/api/tests/auth-refresh-role.test.ts","apps/api/tests/boot-guard-prod.test.ts","apps/api/tests/brand-extractor-ssrf.test.ts","apps/api/tests/client-cart.test.ts","apps/api/tests/courier-assignment-idor.test.ts","apps/api/tests/courier-history-pii.test.ts","apps/api/tests/courier-multi-delivery.test.ts","apps/api/tests/courier-session-binding.test.ts","apps/api/tests/csv-parser.test.ts","apps/api/tests/dev-guard.test.ts","apps/api/tests/eta-service.test.ts","apps/api/tests/eta-synthesis.test.ts","apps/api/tests/fee-parity.test.ts","apps/api/tests/geo-anim-g2.test.ts","apps/api/tests/health-truthfulness.test.ts","apps/api/tests/image-key.test.ts","apps/api/tests/image-url.test.ts","apps/api/tests/keyset-pagination.test.ts","apps/api/tests/menu-grounding.test.ts","apps/api/tests/menu-parse/eval.test.ts","apps/api/tests/money-tax.test.ts","apps/api/tests/notifications-bootstrap.test.ts","apps/api/tests/notifications/category-gating.test.ts","apps/api/tests/notifications/prefs-service.test.ts","apps/api/tests/notifications/quiet-hours.test.ts","apps/api/tests/notifications/storefront-action.test.ts","apps/api/tests/notifications/telegram.test.ts","apps/api/tests/notifications/telegram-webhook-storefront.test.ts","apps/api/tests/ocr-redaction.test.ts","apps/api/tests/order-canonical.test.ts","apps/api/tests/order-persistence.test.ts","apps/api/tests/order-pricing.test.ts","apps/api/tests/orders-guards.test.ts","apps/api/tests/p0-bus-claimcheck.test.ts","apps/api/tests/p0-privacy.test.ts","apps/api/tests/p0-telegram-detail.test.ts","apps/api/tests/paddle-ocr-seam.test.ts","apps/api/tests/phase5/integrity.test.ts","apps/api/tests/phase5/jwt-rotation.test.ts","apps/api/tests/phase5/rls-adversarial.test.ts","apps/api/tests/pii-cipher.test.ts","apps/api/tests/pii-leak-detector.test.ts","apps/api/tests/pii-redactor.test.ts","apps/api/tests/preflight.test.ts","apps/api/tests/product-media-validation.test.ts","apps/api/tests/r2-storage.test.ts","apps/api/tests/rate-limit-envelope.test.ts","apps/api/tests/route-store-g1.test.ts","apps/api/tests/routing-g1.test.ts","apps/api/tests/send-error.test.ts","apps/api/tests/spa-proxy.test.ts","apps/api/tests/spa-shell.test.ts","apps/api/tests/ssr-client-shell.test.ts","apps/api/tests/ssr-escaping.test.ts","apps/api/tests/subdomain-rewrite.test.ts","apps/api/tests/theme-renderer.test.ts","apps/api/tests/websocket-churn.test.ts","apps/web/src/components/accessRequestOutcome.test.ts","apps/web/src/lib/__tests__/analytics.test.ts","apps/web/src/pages/admin/dashboard-utils.test.ts","apps/web/src/pages/admin/__tests__/dashboard-utils.test.ts","apps/worker/tests/timeout-handler.test.ts","e2e/lifecycle-e2e/critical-lifecycle.spec.ts","e2e/rites/song-of-singularity.test.ts","e2e/tests/a11y/sense-redproof.spec.ts","e2e/tests/a11y/storefront.a11y.spec.ts","e2e/tests/admin/dashboard.spec.ts","e2e/tests/admin/full-coverage.spec.ts","e2e/tests/admin/menu-manager.spec.ts","e2e/tests/admin/orders.spec.ts","e2e/tests/admin/promotions.spec.ts","e2e/tests/admin/supplies.spec.ts","e2e/tests/api-real.spec.ts","e2e/tests/audit-fixes.spec.ts","e2e/tests/behavioral-proof.spec.ts","e2e/tests/behavioural-invariants.spec.ts","e2e/tests/capture-delivery.spec.ts","e2e/tests/capture-screens.spec.ts","e2e/tests/capture-states.spec.ts","e2e/tests/client/cart.spec.ts","e2e/tests/client/checkout.spec.ts","e2e/tests/client/client-checkout-happy-path.spec.ts","e2e/tests/client/menu-interaction.spec.ts","e2e/tests/client/menu.spec.ts","e2e/tests/client/modifier-display-type.spec.ts","e2e/tests/client/order-stepper.spec.ts","e2e/tests/client/status-live.spec.ts","e2e/tests/client/status.spec.ts","e2e/tests/client/venue-state.spec.ts","e2e/tests/courier/full-coverage.spec.ts","e2e/tests/courier/offer-timer.spec.ts","e2e/tests/courier/tasks.spec.ts","e2e/tests/cr8-order-messages.spec.ts","e2e/tests/cross-cutting.spec.ts","e2e/tests/cross-tenant-realtime-qa.spec.ts","e2e/tests/dashboard-courier-pins.spec.ts","e2e/tests/debug-order.spec.ts","e2e/tests/deploy-validation.spec.ts","e2e/tests/driver-smoke.spec.ts","e2e/tests/embed-mode.spec.ts","e2e/tests/error-contract.spec.ts","e2e/tests/error-handling.spec.ts","e2e/tests/fe-polish-batch.spec.ts","e2e/tests/fe-radar.spec.ts","e2e/tests/fe-radar-v2.spec.ts","e2e/tests/fix-validation.spec.ts","e2e/tests/flow-admin-deep.spec.ts","e2e/tests/flow-client-product-images.spec.ts","e2e/tests/flow-core-lifecycles.spec.ts","e2e/tests/flow-courier-deep.spec.ts","e2e/tests/flow-customer-checkout-render.spec.ts","e2e/tests/flow-customer-track-link.spec.ts","e2e/tests/flow-geo-tracking.spec.ts","e2e/tests/flow-ingredients.spec.ts","e2e/tests/flow-modifiers-promotions.spec.ts","e2e/tests/flow-offline-phone-fallback.spec.ts","e2e/tests/flow-onboarding-auth.spec.ts","e2e/tests/flow-onboarding-parsing.spec.ts","e2e/tests/flow-order-creation.spec.ts","e2e/tests/flow-order-lifecycle-trace.spec.ts","e2e/tests/flow-orders-checkout.spec.ts","e2e/tests/flow-productcard-declutter.spec.ts","e2e/tests/flow-proofs.spec.ts","e2e/tests/flow-regulatory-settlements.spec.ts","e2e/tests/flow-security-contracts.spec.ts","e2e/tests/flow-security-regression-2026-06.spec.ts","e2e/tests/flow-sensor-delivery-baseline.spec.ts","e2e/tests/flow-sensor-eta-window.spec.ts","e2e/tests/flow-sensor-funnel.spec.ts","e2e/tests/flow-sensor-geofence.spec.ts","e2e/tests/flow-start-hero.spec.ts","e2e/tests/flow-ui-admin-branding.spec.ts","e2e/tests/flow-ui-admin-dashboard.spec.ts","e2e/tests/flow-ui-admin-menumanager.spec.ts","e2e/tests/flow-ui-admin-product-bom.spec.ts","e2e/tests/flow-ui-admin-settings.spec.ts","e2e/tests/flow-ui-admin-supply-library.spec.ts","e2e/tests/flow-ui-analytics-supplies.spec.ts","e2e/tests/flow-ui-client-checkout.spec.ts","e2e/tests/flow-ui-client-order-full.spec.ts","e2e/tests/flow-ui-courier-actions.spec.ts","e2e/tests/flow-ui-courier-core.spec.ts","e2e/tests/flow-ui-courier-full.spec.ts","e2e/tests/flow-ui-courier-invite.spec.ts","e2e/tests/flow-ui-empty-states.spec.ts","e2e/tests/flow-ui-images.spec.ts","e2e/tests/flow-ui-invite-onboarding.spec.ts","e2e/tests/flow-ui-menu-interactions.spec.ts","e2e/tests/flow-ui-order-lifecycle.spec.ts","e2e/tests/flow-ui-order-status.spec.ts","e2e/tests/flow-ui-owner-core.spec.ts","e2e/tests/flow-ui-owner-crud.spec.ts","e2e/tests/flow-ui-proof-comprehensive.spec.ts","e2e/tests/flow-ui-settings-promotions.spec.ts","e2e/tests/flow-ui-validation.spec.ts","e2e/tests/golive-remediation.spec.ts","e2e/tests/groq-import-proof.spec.ts","e2e/tests/live-smoke.spec.ts","e2e/tests/maps.spec.ts","e2e/tests/media-render.spec.ts","e2e/tests/menu-first-onboarding.spec.ts","e2e/tests/menu-load.spec.ts","e2e/tests/mobile-polish.spec.ts","e2e/tests/nomadic-skin.spec.ts","e2e/tests/non-pixel-sweep.spec.ts","e2e/tests/notif-categories-local.spec.ts","e2e/tests/notif-categories.spec.ts","e2e/tests/notification-events.spec.ts","e2e/tests/notification-flow.spec.ts","e2e/tests/notification-helper-test.spec.ts","e2e/tests/onboarding-copy-qa.spec.ts","e2e/tests/onboarding-e2e.spec.ts","e2e/tests/onboarding-wizard-retired.spec.ts","e2e/tests/order-created-notification.spec.ts","e2e/tests/owner-fixes-batch.spec.ts","e2e/tests/owner-revocation.spec.ts","e2e/tests/paper-skin-tokens.spec.ts","e2e/tests/polish-debt-logic.spec.ts","e2e/tests/polish-debt.spec.ts","e2e/tests/polish-qa-loop.spec.ts","e2e/tests/prod-adr0004-smoke.spec.ts","e2e/tests/prod-smoke.spec.ts","e2e/tests/quick-order-test.spec.ts","e2e/tests/real-notifications.spec.ts","e2e/tests/real-session-menu.spec.ts","e2e/tests/recent-changes-validation.spec.ts","e2e/tests/rsi-round.spec.ts","e2e/tests/seam-polish.spec.ts","e2e/tests/seed-public.spec.ts","e2e/tests/seed.spec.ts","e2e/tests/simple-auth.spec.ts","e2e/tests/smoke.spec.ts","e2e/tests/soft-access-gate.spec.ts","e2e/tests/storefront-smoke.spec.ts","e2e/tests/storefront.smoke.spec.ts","e2e/tests/sunlight-mode.spec.ts","e2e/tests/telegram-full-flow.spec.ts","e2e/tests/telegram-test.spec.ts","e2e/tests/telegram-webhook.spec.ts","e2e/tests/telegram-webhook-test.spec.ts","e2e/tests/ui-improvements.spec.ts","e2e/tests/ui-loop-fixes.spec.ts","e2e/tests/ui-polish.spec.ts","e2e/tests/ux1-storefront-links.spec.ts","e2e/tests/ux2-messenger-deeplink.spec.ts","e2e/tests/ux3-entry-photo.spec.ts","e2e/tests/ux4-tips.spec.ts","e2e/tests/ws-courier-assignment.spec.ts","e2e/visual/client-path.visual.spec.ts","e2e/visual/courier-path.visual.spec.ts","e2e/visual/owner-path.visual.spec.ts","packages/core/preflight/evaluatePreflight.test.ts","packages/platform/tests/message-bus-dispatch.test.ts","packages/platform/tests/message-bus-notify.test.ts","packages/ui/src/lib/__tests__/i18n.test.ts","packages/ui/src/theme/css-comment-integrity.test.ts","packages/ui/src/utils/__tests__/index.test.ts","tools/ccc/tests/secret-scan.test.ts","tools/eslint-plugin-local/__fixtures__/permissive-status.spec.ts","tools/loop-harness/tests/autoupgrade.test.ts","tools/loop-harness/tests/breaker.test.ts","tools/loop-harness/tests/collect.test.ts","tools/loop-harness/tests/containment.test.ts","tools/loop-harness/tests/detectors.test.ts","tools/loop-harness/tests/eco.test.ts","tools/loop-harness/tests/governor.test.ts","tools/loop-harness/tests/harness.test.ts","tools/loop-harness/tests/loop-builder.test.ts","tools/loop-harness/tests/oracle-integrity.test.ts","tools/loop-harness/tests/oracle.test.ts","tools/loop-harness/tests/proposals.test.ts","tools/loop-harness/tests/registry.test.ts","tools/loop-harness/tests/repo-apply.test.ts","tools/loop-harness/tests/report.test.ts","tools/loop-harness/tests/review-queue.test.ts","tools/loop-harness/tests/router.test.ts","tools/loop-harness/tests/smoke.test.ts","tools/loop-harness/tests/storage.test.ts"];
const specs = (args && Array.isArray(args.specs) && args.specs.length) ? args.specs : ALL_SPECS;
if (!specs.length) { log('no specs passed in args.specs — nothing to scan'); return { scanned: 0, files: [] }; }
log(`scanning ${specs.length} test files for blind-spots (combined critique+security+QA lens)`);

const FINDINGS = {
  type: 'object', additionalProperties: false, required: ['findings'],
  properties: {
    findings: {
      type: 'array',
      items: {
        type: 'object', additionalProperties: false,
        required: ['severity', 'lens', 'blindspot', 'location', 'slips_through', 'fix'],
        properties: {
          severity: { type: 'string', enum: ['CRITICAL', 'HIGH', 'MED', 'LOW'] },
          lens: { type: 'string', enum: ['critique', 'security', 'qa'] },
          blindspot: { type: 'string', description: 'one-line: what the test misses / asserts weakly' },
          location: { type: 'string', description: 'file:line' },
          slips_through: { type: 'string', description: 'the real bug that would stay green' },
          fix: { type: 'string', description: 'the concrete stronger assertion / scenario to add' },
        },
      },
    },
  },
};

const scanPrompt = (spec) => `Read the test file at \`/root/dowiz/${spec}\` (read it fully). Find its BLIND SPOTS through three lenses:
- critique (false-greens): assertions that PASS even when the feature is broken — too-loose matchers (toContain/truthy/length>0), measuring a snapshot not a delta, swallowed errors, happy-path-only, serial/timing flakiness, fixed sleeps.
- security: weak authz/isolation assertions that pass while the boundary is broken (empty fixtures, permissive status arrays masking leaks, missing positive controls), untested IDOR / cross-tenant / privilege-escalation, secrets/PII hardcoded in the test, risky real-environment side effects.
- qa: error-matrix gaps (401/403/404/409/422/429/5xx/network), unexercised loading/empty/error/terminal states, real-time rigor (asserts the user-facing live update, not just a buffer?), assertion strength.

Return ONLY real, specific findings with a file:line. If the test is already tight (exact assertions, good coverage, no false-green), return an EMPTY findings array — do NOT invent findings to fill space. Severity by how badly a real bug would slip through.`;

const perFile = await parallel(specs.map((spec) => () =>
  agent(scanPrompt(spec), { agentType: 'test-scout', label: `scan:${spec}`, phase: 'Scan', schema: FINDINGS, effort: 'low' })
    .then((r) => ({ spec, findings: (r && r.findings) ? r.findings : [] }))
    .catch(() => ({ spec, findings: [] })),
));

// In-script consolidation (deterministic) — counts, severity histogram, files ranked by weight.
const ok = perFile.filter(Boolean);
const flat = ok.flatMap((f) => f.findings.map((x) => ({ ...x, spec: f.spec })));
const WEIGHT = { CRITICAL: 8, HIGH: 4, MED: 2, LOW: 1 };
const bySeverity = flat.reduce((a, f) => ((a[f.severity] = (a[f.severity] || 0) + 1), a), {});
const byLens = flat.reduce((a, f) => ((a[f.lens] = (a[f.lens] || 0) + 1), a), {});
const fileWeight = ok.map((f) => ({ spec: f.spec, n: f.findings.length, weight: f.findings.reduce((s, x) => s + (WEIGHT[x.severity] || 1), 0) }))
  .filter((f) => f.n > 0).sort((a, b) => b.weight - a.weight);
const critHigh = flat.filter((f) => f.severity === 'CRITICAL' || f.severity === 'HIGH');
log(`scanned ${ok.length} files · ${flat.length} findings (CRIT ${bySeverity.CRITICAL || 0} · HIGH ${bySeverity.HIGH || 0} · MED ${bySeverity.MED || 0} · LOW ${bySeverity.LOW || 0})`);

// One synthesis agent over the CRITICAL+HIGH findings → systemic patterns + top fixes (bounded input).
let systemic = null;
if (critHigh.length) {
  systemic = await agent(
    `These are the CRITICAL/HIGH test blind-spots found across the dowiz test surface (${ok.length} files scanned). Identify the RECURRING SYSTEMIC patterns (the same weakness appearing across many files — e.g. "real-time asserted via buffer not user-facing update", "permissive [401,403,404] status arrays", "empty-fixture isolation", "no positive control"), and the TOP 10 highest-ROI hardenings to implement first (each: pattern · which files · the fix). Findings:\n\n${JSON.stringify(critHigh, null, 2)}`,
    { label: 'synthesize', phase: 'Consolidate', schema: {
      type: 'object', additionalProperties: false, required: ['systemic_patterns', 'top_fixes'],
      properties: {
        systemic_patterns: { type: 'array', items: { type: 'string' } },
        top_fixes: { type: 'array', items: { type: 'string' } },
      },
    } },
  );
}

return {
  scanned: ok.length,
  total_findings: flat.length,
  by_severity: bySeverity,
  by_lens: byLens,
  ranked_files: fileWeight.slice(0, 40),
  crit_high: critHigh,
  systemic,
};
