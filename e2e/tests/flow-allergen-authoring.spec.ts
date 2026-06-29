/**
 * Allergen authoring (data prerequisite for menu-characteristics-model) — owner declares allergens, they persist.
 *
 * The council-approved coverage lever: the AllergenEditor was built but orphaned. This proves it is now wired
 * into the product form and that an owner's allergen DECLARATION (L3 — owner is the authority, never derived)
 * round-trips: declare "Has allergens" + nuts → save → reopen → the selection is still there. Persisted in
 * attributes.allergen_status + attributes.declared_allergens (no schema/migration change; API merges attributes).
 *
 * Mandatory-Proof: DOM-level toBeVisible/aria-pressed on the real owner form, against the deployed app.
 */
import { test, expect, type Page, type Locator } from '@playwright/test';
import { expectJwt } from '../helpers/assert-shape';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';
const ALLERGEN = 'nuts'; // a declared-presence chip; allergen-chip-nuts

let ownerToken = '';
let product: { id: string; name: string; categoryName: string };

async function injectAuth(page: Page) {
  await page.goto(`${BASE}/admin`, { waitUntil: 'load', timeout: 30000 });
  await page.evaluate((tok) => localStorage.setItem('dos_access_token', tok), ownerToken);
}

// Open the edit modal for a product (category chip sets React's expandedCat, which handleSaveProduct guards on).
async function openEditForm(page: Page): Promise<Locator> {
  await injectAuth(page); // sets the owner token in localStorage (else /admin/menu redirects to login)
  await page.goto(`${BASE}/admin/menu`, { waitUntil: 'networkidle', timeout: 30000 });
  await page.waitForTimeout(2000);
  const card = page.locator('div.rounded-xl.border').filter({ hasText: product.name }).first();
  // Must click the product's CATEGORY tab (index 0 is "All", which does NOT set expandedCat — and
  // handleSaveProduct returns early when expandedCat is null, silently no-op'ing the save). There are many
  // categories, so click the named tab directly (fast); fall back to a bounded iteration.
  const tabs = page.locator('div.snap-x button');
  if (product.categoryName) {
    const named = tabs.filter({ hasText: product.categoryName }).first();
    if (await named.isVisible({ timeout: 3000 }).catch(() => false)) {
      await named.scrollIntoViewIfNeeded().catch(() => {});
      await named.click().catch(() => {});
      await page.waitForTimeout(1500);
    }
  }
  if (!(await card.isVisible({ timeout: 2000 }).catch(() => false))) {
    const n = Math.min(await tabs.count(), 25);
    for (let i = 1; i < n; i++) {
      await tabs.nth(i).click().catch(() => {});
      await page.waitForTimeout(900);
      if (await card.isVisible({ timeout: 1000 }).catch(() => false)) break;
    }
  }
  await card.waitFor({ state: 'visible', timeout: 10000 });
  const editBtn = card.locator('button').filter({ has: page.locator('.ti-edit') }).first();
  if (await editBtn.isVisible({ timeout: 3000 }).catch(() => false)) {
    await editBtn.click();
  } else {
    await card.click();
    await page.waitForTimeout(500);
    await page.getByRole('button', { name: /edit|ndrysho/i }).first().click();
  }
  const modal = page.locator('div.fixed.inset-0.z-50').last();
  await modal.waitFor({ state: 'visible', timeout: 8000 });
  // scroll the attestation block into view (it sits below the recipe editor)
  await modal.getByTestId('allergen-attestation').scrollIntoViewIfNeeded().catch(() => {});
  return modal;
}

test.describe.configure({ mode: 'serial' });

test.describe('Allergen authoring — owner declaration persists (menu-characteristics data prerequisite)', () => {
  test.beforeAll(async ({ request }) => {
    const r = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(r.status()).toBe(200);
    ownerToken = (await r.json()).access_token;
    expectJwt(ownerToken, 'mock-auth access_token');

    const pr = await request.get(`${BASE}/api/owner/menu/products`, { headers: { Authorization: `Bearer ${ownerToken}` } });
    expect(pr.status()).toBe(200);
    const body = await pr.json();
    const raw: any[] = body.products || body.data || body;
    expect(raw.length, 'demo must have ≥1 product').toBeGreaterThan(0);
    const p = raw[0];
    let categoryName = '';
    const cr = await request.get(`${BASE}/api/owner/menu/categories`, { headers: { Authorization: `Bearer ${ownerToken}` } });
    if (cr.ok()) {
      const cb = await cr.json();
      const cats: any[] = cb.categories || cb.data || cb;
      categoryName = cats.find((c: any) => c.id === (p.categoryId || p.category_id))?.name || '';
    }
    product = { id: p.id, name: p.name, categoryName };
    // idempotency: clear any prior attestation so the "reopen shows unset" precondition holds
    await request.patch(`${BASE}/api/owner/menu/products/${product.id}`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
      data: { attributes: { allergen_status: 'unset', declared_allergens: [] } },
    });
  });

  test('the AllergenEditor is wired into the product form (was orphaned)', async ({ page }) => {
    const modal = await openEditForm(page);
    await expect(modal.getByTestId('allergen-attestation')).toBeVisible();
    await expect(modal.getByTestId('allergen-status-listed')).toBeVisible();
    // precondition (post-reset): unset is active, chips hidden
    await expect(modal.getByTestId('allergen-status-unset')).toHaveAttribute('aria-pressed', 'true');
    await expect(modal.getByTestId(`allergen-chip-${ALLERGEN}`)).toHaveCount(0);
  });

  test('owner declares "Has allergens" + nuts, saves → it persists on reopen', async ({ page }) => {
    let modal = await openEditForm(page);
    await modal.getByTestId('allergen-status-listed').click();
    const chip = modal.getByTestId(`allergen-chip-${ALLERGEN}`);
    await expect(chip).toBeVisible(); // chips appear only when 'listed'
    await chip.click();
    await expect(chip).toHaveAttribute('aria-pressed', 'true');

    await modal.getByRole('button', { name: /save|ruaj/i }).last().click();
    await page.waitForTimeout(3000); // save + modal close

    // Reopen the SAME product — the declaration must have round-tripped.
    modal = await openEditForm(page);
    await expect(modal.getByTestId('allergen-status-listed')).toHaveAttribute('aria-pressed', 'true');
    await expect(modal.getByTestId(`allergen-chip-${ALLERGEN}`)).toHaveAttribute('aria-pressed', 'true');
  });

  test('API cross-check: the declaration is stored in attributes (owner-authored, not derived)', async ({ request }) => {
    const r = await request.get(`${BASE}/api/owner/menu/products`, { headers: { Authorization: `Bearer ${ownerToken}` } });
    expect(r.status()).toBe(200);
    const body = await r.json();
    const products: any[] = body.products || body.data || body;
    const saved = products.find(p => p.id === product.id);
    expect(saved?.attributes?.allergen_status, 'allergen_status persisted').toBe('listed');
    expect(saved?.attributes?.declared_allergens, 'declared allergen persisted').toContain(ALLERGEN);
  });
});
