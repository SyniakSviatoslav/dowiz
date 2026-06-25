import { test, expect, type APIRequestContext } from '@playwright/test';

// Testplan §4a/§4b/§4c — Modifier display_type rendering in the /s/demo product modal.
// Run: VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test modifier-display-type --project=desktop --reporter=list
//
// The storefront resolves each modifier group's control via resolveDisplayType()
// (apps/web/src/pages/client/MenuPage.tsx:41): it honours the owner-set display_type,
// else infers radio (max_select===1) / checkbox. The group container carries
// [data-testid="modifier-group"][data-display-type="<type>"] (MenuPage.tsx:1046-1051).
//
// We seed one product with four groups (radio / checkbox / select / quantity) plus one
// UN-typed single-select group (to prove the inferred-radio fallback), open the modal,
// and assert every data-display-type attribute renders. Everything is torn down in finally,
// including restoration of the product's ORIGINAL group attachments (the PUT sync is
// destructive — apps/api/src/routes/owner/products.ts:279 DELETEs before insert).

const CREDS = { email: 'test@dowiz.com', password: 'test123456' };

// Login is rate-limited (5/min per IP). Memoize one token; run a single project.
let cachedToken: string | null = null;
async function ownerToken(request: APIRequestContext): Promise<string> {
  if (cachedToken) return cachedToken;
  let res = await request.post('/api/auth/local/login', { data: CREDS });
  if (res.status() === 429) {
    // Login is capped at 5/min per IP; back off once past the window, then retry.
    await new Promise((r) => setTimeout(r, 75_000));
    res = await request.post('/api/auth/local/login', { data: CREDS });
  }
  expect(res.ok(), 'owner login should succeed').toBeTruthy();
  const body = await res.json();
  expect(body.access_token, 'login returns an access token').toBeTruthy();
  cachedToken = body.access_token as string;
  return cachedToken;
}

// The demo storefront exposes its locationId + product list on the public menu (no auth).
async function demoMenu(request: APIRequestContext) {
  const res = await request.get('/public/locations/demo/menu', { headers: { accept: 'application/json' } });
  expect(res.ok(), 'public demo menu should load').toBeTruthy();
  const menu = await res.json();
  const locationId: string = menu.locationId ?? menu.location_id;
  expect(locationId, 'demo menu carries a locationId').toBeTruthy();
  const products: any[] = (menu.categories ?? []).flatMap((c: any) => c.products ?? []);
  return { locationId, products };
}

test('storefront renders every modifier display_type on the product modal', async ({ page, request }) => {
  // Generous budget: the login backoff (up to 75s on a 429) plus storefront reloads.
  test.setTimeout(180_000);
  const token = await ownerToken(request);
  const auth = { Authorization: `Bearer ${token}` };
  const { locationId, products } = await demoMenu(request);

  // Pick an available product to host the seeded groups.
  const target = products.find((p) => p.available !== false) ?? products[0];
  expect(target, 'demo menu has a product to attach modifier groups to').toBeTruthy();

  const base = `/api/owner/locations/${locationId}`;

  // Capture the product's CURRENT modifier-group attachments so we can restore them —
  // the sync PUT below wipes them first.
  const beforeRes = await request.get(`${base}/products/${target.id}/modifier-groups`, { headers: auth });
  expect(beforeRes.ok(), 'reading current product modifier-groups should succeed').toBeTruthy();
  const beforeRows: any[] = (await beforeRes.json()).data ?? [];
  const originalAttachments = beforeRows.map((r) => ({ group_id: r.id, sort_order: r.sort_order ?? 0 }));

  const createdGroupIds: string[] = [];
  const tag = `e2e-mod-${Date.now()}`;

  async function makeGroup(name: string, displayType: string | null, maxSelect: number): Promise<string> {
    const body: any = { name: `${tag} ${name}`, min_select: 0, max_select: maxSelect, required: false };
    if (displayType) body.display_type = displayType;
    const res = await request.post(`${base}/modifier-groups`, { headers: auth, data: body });
    expect(res.ok(), `create ${name} group should succeed`).toBeTruthy();
    const gid = (await res.json()).id as string;
    createdGroupIds.push(gid);
    // A group renders only when it has at least one available modifier.
    const mod = await request.post(`${base}/modifier-groups/${gid}/modifiers`, {
      headers: auth,
      data: { name: 'Option A', price_delta: 0, available: true, sort_order: 0 },
    });
    expect(mod.ok(), `add modifier to ${name} group should succeed`).toBeTruthy();
    return gid;
  }

  try {
    const radioId = await makeGroup('Radio', 'radio', 1);
    const checkboxId = await makeGroup('Checkbox', 'checkbox', 3);
    const selectId = await makeGroup('Select', 'select', 1);
    const quantityId = await makeGroup('Quantity', 'quantity', 5);
    // Un-typed single-select → resolveDisplayType infers 'radio' (MenuPage.tsx:42-43).
    const inferredId = await makeGroup('Inferred', null, 1);

    const attach = await request.put(`${base}/products/${target.id}/modifier-groups`, {
      headers: auth,
      data: [
        ...originalAttachments,
        { group_id: radioId, sort_order: 90 },
        { group_id: checkboxId, sort_order: 91 },
        { group_id: selectId, sort_order: 92 },
        { group_id: quantityId, sort_order: 93 },
        { group_id: inferredId, sort_order: 94 },
      ],
    });
    expect(attach.ok(), 'attaching seeded groups to the product should succeed').toBeTruthy();

    // Open the storefront and click the seeded product card to open its modal.
    // The public menu fetch can transiently 500 under a concurrent burst (DB pool /
    // rate-limit) leaving an empty grid; reload until a card renders.
    const card = page.locator('[data-testid="menu-item"]', { hasText: target.name }).first();
    let cardVisible = false;
    for (let attempt = 0; attempt < 3 && !cardVisible; attempt++) {
      await page.goto('/s/demo');
      cardVisible = await card.isVisible({ timeout: 20000 }).catch(() => false);
    }
    await expect(card, 'the seeded product card should be visible on /s/demo').toBeVisible({ timeout: 25000 });
    await card.click();

    const dialog = page.locator('[role="dialog"]');
    await expect(dialog, 'product modal (role=dialog) should open').toBeVisible({ timeout: 25000 });

    // At least one modifier group renders inside the modal (§4a step 3).
    const groups = dialog.locator('[data-testid="modifier-group"]');
    await expect(groups.first()).toBeVisible({ timeout: 25000 });

    // Each assertion is scoped to THIS run's uniquely-named groups (`tag`), so a
    // leftover/concurrent run can never trip strict mode.
    const group = (name: string, type: string) =>
      dialog
        .locator(`[data-testid="modifier-group"][data-display-type="${type}"]`)
        .filter({ hasText: `${tag} ${name}` });

    // §4a — explicit radio + checkbox groups carry the correct data-display-type.
    await expect(group('Radio', 'radio'), 'radio group renders with data-display-type=radio').toBeVisible();
    await expect(group('Checkbox', 'checkbox'), 'checkbox group renders with data-display-type=checkbox').toBeVisible();

    // §4b — select group carries data-display-type=select.
    await expect(group('Select', 'select'), 'select group renders with data-display-type=select').toBeVisible();

    // §4c — quantity group carries data-display-type=quantity.
    await expect(group('Quantity', 'quantity'), 'quantity group renders with data-display-type=quantity').toBeVisible();

    // §4a step 6 — the UN-typed single-select group falls back to inferred radio.
    await expect(
      group('Inferred', 'radio'),
      'inferred single-select group also resolves to data-display-type=radio',
    ).toBeVisible();
  } finally {
    // Restore the product's original attachments (or clear if it had none), then drop
    // every seeded group (cascades its modifiers + product_modifier_groups rows).
    await request
      .put(`${base}/products/${target.id}/modifier-groups`, { headers: auth, data: originalAttachments })
      .catch(() => undefined);
    for (const gid of createdGroupIds) {
      await request.delete(`${base}/modifier-groups/${gid}`, { headers: auth }).catch(() => undefined);
    }
  }
});
