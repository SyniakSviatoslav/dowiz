/**
 * COURIER critical-path visual snapshot spec.
 *
 * Screens × states for the courier's delivery flow:
 *   - Tasks  (/courier)               — online (no task) · task-assigned · GPS-permission-denied
 *   - Delivery (/courier/delivery/:id) — map+pickup · photo-proof · deliver · order-closed
 *                                        · BackgroundWarning · WakeLock-failure
 *
 * Breakpoints are PROJECTS (mobile-390 / tablet-768 / desktop-1280) — we do NOT loop them here;
 * we loop LOCALES inside each test. The courier UI is mobile-first but still snapshotted under
 * every project so a tablet/desktop regression can't slip through.
 *
 * Determinism: the map (CourierLiveMap → MapLibreBase) and live ETA carry `data-dynamic`, so the
 * shared MASK(page) covers tiles/markers/countdowns. Auth is the dev mock-auth courier token.
 *
 * Driving delivery states without a live backend: DeliveryPage has a DEV-only fallback — a 404 on
 * GET /api/courier/assignments/:id fabricates a full task (restaurant + cash) so the screen renders.
 * For states the fallback can't express (photo-proof, order-closed) we shape the response with
 * page.route on /api/courier/assignments/:id instead of faking the DOM.
 *
 * Selectors/routes read from source:
 *   - apps/web/src/routes/CourierRoutes.tsx        → routes /courier and /courier/delivery/:id
 *   - apps/web/src/pages/courier/TasksPage.tsx     → online dot, EmptyState, TaskCard
 *   - apps/web/src/pages/courier/DeliveryPage.tsx  → testids: courier-advance-action,
 *       entry-photo-thumb, entry-photo-modal, courier-order-closed, courier-deliver-error,
 *       task-cash-amount; "Mark as Picked Up" button → SwipeToComplete
 *   - packages/ui/src/components/molecules/MapLibreBase.tsx → data-dynamic on the map container
 *   - packages/ui/src/hooks/use-geolocation.ts     → real navigator.geolocation.watchPosition
 */
import { test, expect } from '@playwright/test';
import { loginAs, applyAuth, setLocale, MASK, settle, LOCALES } from './harness.js';
import { readFixtures } from './global-setup.js';
import { emulateGeo } from '../helpers/geo.js';

// A stable courier position in Tirana (matches the page's TIRANA_CENTER neighbourhood).
const COURIER_LAT = 41.331;
const COURIER_LNG = 19.817;

// Any id works for the delivery route: a real assignment 404s → DEV mock fabricates the task,
// or our page.route intercept shapes it. We never need a server-side assignment row.
const DELIVERY_ID = 'visual-courier-delivery';

test.describe('courier critical path — visual', () => {
  // ── Tasks: online, no task ────────────────────────────────────────────────
  test('tasks — online, no task', async ({ page, request }) => {
    const { token } = await loginAs(request, 'courier');
    await applyAuth(page, token);
    // No assignments → the empty "online / no active tasks" state.
    await page.route('**/api/courier/me/assignments', (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true, assignments: [] }) }),
    );
    for (const locale of LOCALES) {
      await setLocale(page, locale);
      await page.goto('/courier');
      await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
      await settle(page);
      await expect(page).toHaveScreenshot(`tasks-online-${locale}.png`, { mask: MASK(page) });
    }
  });

  // ── Tasks: task assigned ──────────────────────────────────────────────────
  test('tasks — task assigned', async ({ page, request }) => {
    const { token } = await loginAs(request, 'courier');
    await applyAuth(page, token);
    const fx = readFixtures();
    // One assigned task → renders a TaskCard with Accept/Reject. Shaped via route so the snapshot
    // is deterministic regardless of whether a server assignment row exists.
    await page.route('**/api/courier/me/assignments', (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          success: true,
          assignments: [
            {
              id: fx.orderId,
              status: 'assigned',
              restaurant: { name: 'Burger King', address: 'Blloku, Tirana', lat: 41.328, lng: 19.812 },
              customer: { address: 'Rruga e Elbasanit 12', lat: 41.337, lng: 19.825 },
              total: 120000,
              eta: '10 min',
            },
          ],
        }),
      }),
    );
    for (const locale of LOCALES) {
      await setLocale(page, locale);
      await page.goto('/courier');
      await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
      await settle(page);
      await expect(page).toHaveScreenshot(`tasks-assigned-${locale}.png`, { mask: MASK(page) });
    }
  });

  // ── Tasks: GPS permission denied ──────────────────────────────────────────
  // fixme: TasksPage has NO geolocation surface — it never calls useGeolocation and renders nothing
  // GPS-related, so a denied permission produces a snapshot identical to "online, no task". The
  // geo helper simulateGPSDenied() also dispatches a `dos:geo:denied` CustomEvent that no current
  // code listens to (useGeolocation uses the real navigator.geolocation.watchPosition). The only
  // real GPS-denied surface in the courier flow is the Delivery map's geoError banner, covered by
  // the "delivery — gps denied banner" test below. Re-enable here if Tasks ever shows GPS status.
  test.fixme('tasks — gps permission denied', async () => {
    // Intentionally empty: no Tasks-level GPS surface exists to snapshot (see comment above).
  });

  // ── Delivery: map + pickup state ──────────────────────────────────────────
  // The DEV-only 404 fallback fabricates a cash task → renders map + drop-off card + "Mark as
  // Picked Up". Grant geolocation so the courier marker + GPS dot render (both masked via map).
  test('delivery — map + pickup state', async ({ page, context, request }) => {
    await emulateGeo(context, COURIER_LAT, COURIER_LNG);
    const { token } = await loginAs(request, 'courier');
    await applyAuth(page, token);
    for (const locale of LOCALES) {
      await setLocale(page, locale);
      await page.goto(`/courier/delivery/${DELIVERY_ID}`);
      // Pickup state: the "Mark as Picked Up" primary button is present (pickedUp === false).
      await expect(page.getByText(/Mark as Picked Up|Shëno si i marrë/i)).toBeVisible();
      await settle(page);
      await expect(page).toHaveScreenshot(`delivery-pickup-${locale}.png`, { mask: MASK(page) });
    }
  });

  // ── Delivery: photo-proof (entry-photo) state ─────────────────────────────
  // entryPhotoUrl is only rendered when present on the task; the DEV mock omits it, so we shape
  // the assignment response to include it, then open the fullscreen proof modal (entry-photo-modal).
  test('delivery — photo proof modal', async ({ page, context, request }) => {
    await emulateGeo(context, COURIER_LAT, COURIER_LNG);
    const { token } = await loginAs(request, 'courier');
    await applyAuth(page, token);
    await page.route(`**/api/courier/assignments/${DELIVERY_ID}`, (route) =>
      route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          id: DELIVERY_ID,
          status: 'IN_DELIVERY',
          restaurant: { name: 'Burger King', address: 'Blloku, Tirana', lat: 41.328, lng: 19.812 },
          customer: {
            address: 'Rruga e Elbasanit 12',
            phone: '+355 69 123 4567',
            lat: 41.337,
            lng: 19.825,
            // 1x1 transparent PNG — deterministic, no network image dependency.
            entryPhotoUrl:
              'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+M8AAAMBAQDJ/1eRAAAAAElFTkSuQmCC',
          },
          total: 120000,
          eta: '10 min',
        }),
      }),
    );
    for (const locale of LOCALES) {
      await setLocale(page, locale);
      await page.goto(`/courier/delivery/${DELIVERY_ID}`);
      await page.getByTestId('entry-photo-thumb').click();
      await expect(page.getByTestId('entry-photo-modal')).toBeVisible();
      await settle(page);
      await expect(page).toHaveScreenshot(`delivery-photo-proof-${locale}.png`, { mask: MASK(page) });
    }
  });

  // ── Delivery: deliver (post-pickup, swipe-to-deliver) state ───────────────
  // After "Mark as Picked Up" the screen swaps the primary button for the cash-collected input +
  // SwipeToComplete (data-testid courier-advance-action). The DEV mock task is cash, so the cash
  // input renders too.
  test('delivery — deliver (swipe) state', async ({ page, context, request }) => {
    await emulateGeo(context, COURIER_LAT, COURIER_LNG);
    const { token } = await loginAs(request, 'courier');
    await applyAuth(page, token);
    // Swallow the picked-up POST so the optimistic state flips without a backend assignment.
    await page.route(`**/api/courier/assignments/${DELIVERY_ID}/picked-up`, (route) =>
      route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify({ success: true }) }),
    );
    for (const locale of LOCALES) {
      await setLocale(page, locale);
      await page.goto(`/courier/delivery/${DELIVERY_ID}`);
      await page.getByText(/Mark as Picked Up|Shëno si i marrë/i).click();
      await expect(page.getByTestId('courier-advance-action')).toBeVisible();
      await settle(page);
      await expect(page).toHaveScreenshot(`delivery-deliver-${locale}.png`, { mask: MASK(page) });
    }
  });

  // ── Delivery: order-closed soft banner ────────────────────────────────────
  // fixme: courier-order-closed is driven SOLELY by a WS `order.status` (CANCELLED/REJECTED) frame
  // on room order:<id> — there is no REST response, query param, or test hook that renders it, and
  // the page exposes no documented way to inject a WS frame from the test. Mocking the WS transport
  // deterministically needs harness support (a server-side seed that pushes the frame, or a page
  // WS-inject hook) that does not exist yet. Author once that hook lands — until then snapshotting
  // it would mean faking the DOM, which the rules forbid.
  test.fixme('delivery — order closed banner', async () => {
    // Intentionally empty: no deterministic way to deliver the WS order.status frame (see above).
  });

  // ── Delivery: GPS-denied banner ───────────────────────────────────────────
  // The real courier-facing GPS-denied surface: useGeolocation reports PERMISSION_DENIED and the
  // page renders the geoError banner over the map. We achieve this by NOT granting geolocation
  // (Playwright default → permission denied → watchPosition error).
  test('delivery — gps denied banner', async ({ page, request }) => {
    const { token } = await loginAs(request, 'courier');
    await applyAuth(page, token);
    for (const locale of LOCALES) {
      await setLocale(page, locale);
      await page.goto(`/courier/delivery/${DELIVERY_ID}`);
      await expect(page.getByText(/Mark as Picked Up|Shëno si i marrë/i)).toBeVisible();
      await settle(page);
      await expect(page).toHaveScreenshot(`delivery-gps-denied-${locale}.png`, { mask: MASK(page) });
    }
  });

  // ── Delivery: BackgroundWarning ───────────────────────────────────────────
  // fixme: there is NO BackgroundWarning surface in the courier code (no component, no testid, no
  // copy) in DeliveryPage.tsx or packages/ui. The page tracks GPS via a time-based heartbeat but
  // never warns about backgrounding/tab-visibility. Author when the warning UI is built.
  test.fixme('delivery — background warning', async () => {
    // Intentionally empty: no BackgroundWarning UI exists to snapshot.
  });

  // ── Delivery: WakeLock failure ────────────────────────────────────────────
  // fixme: there is NO WakeLock acquisition or WakeLock-failure surface in the courier code
  // (grep for WakeLock/wake-lock returns nothing in apps/web/src/pages/courier or packages/ui).
  // Author when a wake-lock indicator/failure banner is built.
  test.fixme('delivery — wake lock failure', async () => {
    // Intentionally empty: no WakeLock UI exists to snapshot.
  });
});
