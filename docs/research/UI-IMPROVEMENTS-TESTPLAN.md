# UI Improvements Test Plan — feat/golive-remediation

## Application Overview

Comprehensive Playwright test plan for the six UI improvements shipped to staging (https://dowiz-staging.fly.dev) on feat/golive-remediation. Every test targets real DOM elements via data-testid attributes confirmed in source code and/or live page inspection. Tests are structured as independent, stateless scenarios using expect(...).toBeVisible() / toContainText() / not.toBeVisible() on real elements. Where a scenario requires seeded state not available on staging today it is marked BLOCKED with the exact seed operation needed. The test owner for all specs is test@dowiz.com / test123456. The storefront demo slug is "demo". The admin routes are /admin/* and the courier routes are /courier/*. Login for owners goes via /login (NOT /admin/login). Live exploration confirmed: venue-state-chip (data-state=open) renders on /s/demo; owner-alert-enable (data-state=blocked) renders on /admin dashboard; schedule-editor renders on /admin/menu; modifier-group testid exists in code but the demo products have no modifier groups seeded; courier-offer-timer and courier-advance-action require an active courier session with a live dispatched task.

## Test Scenarios

### 1. 1 — Storefront Venue State (/s/demo)

**Seed:** `e2e/tests/smoke.spec.ts`

#### 1.1. 1a — venue-state-chip renders with data-state=open on the demo storefront [GO]

**File:** `e2e/tests/client/venue-state.spec.ts`

**Steps:**
  1. Navigate to https://dowiz-staging.fly.dev/s/demo and wait for the page title 'Dubin & Sushi' to appear.
    - expect: Page title contains 'Dubin & Sushi'
  2. Assert that [data-testid='venue-state-chip'] is visible in the header section.
    - expect: expect(page.locator('[data-testid="venue-state-chip"]')).toBeVisible()
    - expect: chip has data-state='open' when the demo location is within open hours
    - expect: chip text contains 'Open' (or 'I hapur' in Albanian locale)
  3. Assert that no venue-busy-banner and no venue-closed-banner are present when state is open.
    - expect: expect(page.locator('[data-testid="venue-busy-banner"]')).not.toBeVisible()
    - expect: expect(page.locator('[data-testid="venue-closed-banner"]')).not.toBeVisible()

#### 1.2. 1b — venue-closed-banner renders when demo location is closed [BLOCKED: seed required]

**File:** `e2e/tests/client/venue-state.spec.ts`

**Steps:**
  1. SEED OPERATION: Using an authenticated owner JWT (POST /api/auth/local/login with test@dowiz.com/test123456), PATCH /api/owner/locations/:locationId/hours to set hours_json that marks the current time as closed (e.g. empty hours_json or hours that exclude the current UTC time). Alternatively, set delivery_paused=true.
    - expect: API returns 200 with updated location record
  2. Navigate to https://dowiz-staging.fly.dev/s/demo.
    - expect: Page loads without error
  3. Assert that [data-testid='venue-closed-banner'] is visible.
    - expect: expect(page.locator('[data-testid="venue-closed-banner"]')).toBeVisible()
    - expect: Banner text contains a closed message (e.g. 'closed' / 'mbyllur')
  4. Assert that [data-testid='venue-state-chip'] has data-state='closed'.
    - expect: expect(page.locator('[data-testid="venue-state-chip"]')).toHaveAttribute('data-state', 'closed')
  5. CLEANUP: Restore the location to open hours.
    - expect: Location returns to open state

#### 1.3. 1c — venue-busy-banner renders and ordering stays open when kitchen_busy_until is set [BLOCKED: seed required]

**File:** `e2e/tests/client/venue-state.spec.ts`

**Steps:**
  1. SEED OPERATION: Authenticate as owner (POST /api/auth/local/login). PATCH /api/owner/locations/:locationId/kitchen-busy with body { busy_until: '<ISO timestamp 2 hours from now>' }. Confirm response includes kitchenBusyUntil.
    - expect: API returns 200 with kitchenBusyUntil set to future timestamp
  2. Navigate to https://dowiz-staging.fly.dev/s/demo.
    - expect: Page loads without error
  3. Assert [data-testid='venue-busy-banner'] is visible and shows a 'kitchen busy' message.
    - expect: expect(page.locator('[data-testid="venue-busy-banner"]')).toBeVisible()
    - expect: Banner text contains 'kitchen is busy' or 'The kitchen is busy right now'
  4. Assert [data-testid='venue-state-chip'] has data-state='busy'.
    - expect: expect(page.locator('[data-testid="venue-state-chip"]')).toHaveAttribute('data-state', 'busy')
  5. Assert that the 'Add to cart' buttons (button with aria-label matching 'Shto') are still visible and enabled — busy does NOT close ordering.
    - expect: At least one add-to-cart button remains enabled
    - expect: expect(page.locator('[data-testid="venue-closed-banner"]')).not.toBeVisible()
  6. CLEANUP: PATCH /api/owner/locations/:locationId/kitchen-busy with { busy_until: null } to clear the busy state.
    - expect: Location returns to open state

#### 1.4. 1d — item-state-chip (sold-out) renders on unavailable products [BLOCKED: no sold-out item in demo]

**File:** `e2e/tests/client/venue-state.spec.ts`

**Steps:**
  1. SEED OPERATION: Authenticate as owner. PATCH /api/owner/locations/:locationId/products/:productId with { available: false } for one product visible on /s/demo.
    - expect: API returns 200; product.available=false
  2. Navigate to https://dowiz-staging.fly.dev/s/demo and wait for product cards to render.
    - expect: Menu grid is visible
  3. Assert that [data-testid='item-state-chip'] is visible on at least one product card.
    - expect: expect(page.locator('[data-testid="item-state-chip"]').first()).toBeVisible()
    - expect: chip has data-state='sold_out'
    - expect: chip text contains 'Sold out' or 'I shitur'
  4. CLEANUP: PATCH product back to { available: true }.
    - expect: Product returns to available state

### 2. 2 — Order-Status Stepper (/s/:slug/orders/:id)

**Seed:** `e2e/tests/smoke.spec.ts`

#### 2.1. 2a — order-progress container renders with correct data-order-type for a CONFIRMED delivery order [BLOCKED: needs customer order session + order ID]

**File:** `e2e/tests/client/order-stepper.spec.ts`

**Steps:**
  1. SEED OPERATION: Place a delivery order via the /s/demo storefront checkout (or via POST /api/customer/orders with a valid customer JWT), note the returned order ID and customer access token.
    - expect: Order created with status PENDING; order ID obtained
  2. Navigate to /s/demo/orders/:orderId?t=<trackingToken> (or inject customer JWT via localStorage dos_access_token and navigate to /s/demo/orders/:orderId).
    - expect: OrderStatusPage renders without redirect or error
  3. Assert [data-testid='order-progress'] is visible.
    - expect: expect(page.locator('[data-testid="order-progress"]')).toBeVisible()
  4. Assert data-order-type='delivery' on the order-progress container.
    - expect: expect(page.locator('[data-testid="order-progress"]')).toHaveAttribute('data-order-type', 'delivery')
  5. Assert the PENDING step is active (data-active='true') and the CONFIRMED step is not yet active (data-active='false').
    - expect: expect(page.locator('[data-testid="order-step-pending"]')).toHaveAttribute('data-active', 'true')
    - expect: expect(page.locator('[data-testid="order-step-confirmed"]')).toHaveAttribute('data-active', 'false')
  6. SEED: Confirm the order as owner (PATCH /api/owner/orders/:id with { status: 'CONFIRMED' }). Wait for the WS push or reload the page.
    - expect: Order status changes to CONFIRMED
  7. Assert order-step-confirmed has data-active='true' and order-step-confirmed-time is visible with a time string.
    - expect: expect(page.locator('[data-testid="order-step-confirmed"]')).toHaveAttribute('data-active', 'true')
    - expect: expect(page.locator('[data-testid="order-step-confirmed-time"]')).toBeVisible()
  8. Assert the delivery-branch steps are present: order-step-in_delivery and order-step-delivered are in the DOM; order-step-picked_up is absent.
    - expect: expect(page.locator('[data-testid="order-step-in_delivery"]')).toBeVisible()
    - expect: expect(page.locator('[data-testid="order-step-delivered"]')).toBeVisible()
    - expect: expect(page.locator('[data-testid="order-step-picked_up"]')).not.toBeVisible()

#### 2.2. 2b — order-progress shows pickup branch (READY→PICKED_UP) for a pickup order [BLOCKED: needs pickup order]

**File:** `e2e/tests/client/order-stepper.spec.ts`

**Steps:**
  1. SEED OPERATION: Place or create a pickup order (type='pickup') and obtain its ID and customer token.
    - expect: Pickup order created; data-order-type will be 'pickup'
  2. Navigate to the order status page for the pickup order.
    - expect: OrderStatusPage renders
  3. Assert data-order-type='pickup' on [data-testid='order-progress'].
    - expect: expect(page.locator('[data-testid="order-progress"]')).toHaveAttribute('data-order-type', 'pickup')
  4. Assert order-step-picked_up is visible; order-step-in_delivery and order-step-delivered are absent.
    - expect: expect(page.locator('[data-testid="order-step-picked_up"]')).toBeVisible()
    - expect: expect(page.locator('[data-testid="order-step-in_delivery"]')).not.toBeVisible()
    - expect: expect(page.locator('[data-testid="order-step-delivered"]')).not.toBeVisible()

#### 2.3. 2c — order-progress shows terminal styling for REJECTED order [BLOCKED: needs rejected order]

**File:** `e2e/tests/client/order-stepper.spec.ts`

**Steps:**
  1. SEED OPERATION: Create an order and reject it as owner (PATCH /api/owner/orders/:id with { status: 'REJECTED' }).
    - expect: Order transitions to REJECTED
  2. Navigate to the order status page.
    - expect: OrderStatusPage renders
  3. Assert order-step-rejected has data-active='true'.
    - expect: expect(page.locator('[data-testid="order-step-rejected"]')).toHaveAttribute('data-active', 'true')
  4. Assert the progress bar has a red/danger colour: inspect the inline style of the progress fill element and confirm background uses --color-danger.
    - expect: The progress fill element has background containing 'danger' or a danger-red hex value
    - expect: The order-step-rejected dot colour reflects the terminal/danger state

#### 2.4. 2d — order-progress shows terminal styling for CANCELLED order [BLOCKED: needs cancelled order]

**File:** `e2e/tests/client/order-stepper.spec.ts`

**Steps:**
  1. SEED OPERATION: Create an order and cancel it (PATCH with { status: 'CANCELLED' } or customer cancellation if available).
    - expect: Order transitions to CANCELLED
  2. Navigate to the order status page.
    - expect: OrderStatusPage renders
  3. Assert [data-testid='order-step-cancelled'] has data-active='true'.
    - expect: expect(page.locator('[data-testid="order-step-cancelled"]')).toHaveAttribute('data-active', 'true')

### 3. 3 — Owner New-Order Alert (/admin dashboard)

**Seed:** `e2e/tests/smoke.spec.ts`

#### 3.1. 3a — owner-alert-enable renders with data-state='blocked' on a fresh browser session [GO]

**File:** `e2e/tests/admin/owner-alert.spec.ts`

**Steps:**
  1. Navigate to https://dowiz-staging.fly.dev/login. Fill email=test@dowiz.com, password=test123456, click 'Hyr'. Wait for redirect to /admin.
    - expect: URL becomes /admin
    - expect: Dashboard content is visible
  2. In a fresh browser context (no AudioContext gesture yet), assert that [data-testid='owner-alert-enable'] is visible.
    - expect: expect(page.locator('[data-testid="owner-alert-enable"]')).toBeVisible()
    - expect: Element has data-state that is either 'blocked' or 'muted' (never 'armed')
    - expect: Button text contains 'Enable sound'
  3. Assert that [data-testid='owner-alert-status'] is NOT visible (no armed chip before user gesture).
    - expect: expect(page.locator('[data-testid="owner-alert-status"]')).not.toBeVisible()

#### 3.2. 3b — owner-alert-status renders with data-state='armed' after clicking Enable Sound [GO - with interaction]

**File:** `e2e/tests/admin/owner-alert.spec.ts`

**Steps:**
  1. Log in as test@dowiz.com and navigate to /admin.
    - expect: Dashboard loads; owner-alert-enable is visible
  2. Click [data-testid='owner-alert-enable'] to perform the browser AudioContext unlock gesture.
    - expect: Click is registered
  3. Wait up to 2 seconds and assert [data-testid='owner-alert-status'] is now visible with data-state='armed'.
    - expect: expect(page.locator('[data-testid="owner-alert-status"]')).toBeVisible({ timeout: 2000 })
    - expect: expect(page.locator('[data-testid="owner-alert-status"]')).toHaveAttribute('data-state', 'armed')
    - expect: Element text contains 'Alerts on'
  4. Assert that [data-testid='owner-alert-enable'] is no longer visible.
    - expect: expect(page.locator('[data-testid="owner-alert-enable"]')).not.toBeVisible()

#### 3.3. 3c — owner-new-order-banner appears when there is an unacknowledged order AND alert is not armed [GO - with existing unack orders on staging]

**File:** `e2e/tests/admin/owner-alert.spec.ts`

**Steps:**
  1. Log in as test@dowiz.com and navigate to /admin. Do NOT click Enable Sound (so alertState stays 'blocked' or 'muted').
    - expect: owner-alert-enable is visible; alert is not armed
  2. SEED OPERATION (if no unacknowledged orders exist): Place a new order on /s/demo to create an unacknowledged PENDING order, or via POST /api/customer/orders. If staging already has unack orders (observed: CONFIRMED order #4578 exists), this step may be skippable.
    - expect: At least one unacknowledged order exists in PENDING state
  3. Reload /admin and assert [data-testid='owner-new-order-banner'] is visible with pulsing animation.
    - expect: expect(page.locator('[data-testid="owner-new-order-banner"]')).toBeVisible()
    - expect: Banner text contains 'New order' or a count of unacknowledged orders
    - expect: Banner has animate-pulse CSS class
  4. Click [data-testid='owner-new-order-banner'] to acknowledge orders.
    - expect: Banner disappears after click
    - expect: expect(page.locator('[data-testid="owner-new-order-banner"]')).not.toBeVisible({ timeout: 2000 })

#### 3.4. 3d — owner-new-order-banner is absent when alert IS armed [GO - after arming alerts]

**File:** `e2e/tests/admin/owner-alert.spec.ts`

**Steps:**
  1. Log in as test@dowiz.com and navigate to /admin. Click [data-testid='owner-alert-enable'] to arm alerts.
    - expect: owner-alert-status renders with data-state='armed'
  2. Regardless of unacknowledged order count, assert [data-testid='owner-new-order-banner'] is NOT visible (the armed state suppresses the fallback banner).
    - expect: expect(page.locator('[data-testid="owner-new-order-banner"]')).not.toBeVisible()

### 4. 4 — Modifier display_type (/s/demo product modal)

**Seed:** `e2e/tests/smoke.spec.ts`

#### 4.1. 4a — modifier-group with data-display-type renders in product modal for a product with modifier groups [BLOCKED: no product on demo has modifier groups seeded]

**File:** `e2e/tests/client/modifier-display-type.spec.ts`

**Steps:**
  1. SEED OPERATION: Authenticate as owner. Create a modifier group via POST /api/owner/locations/:locationId/modifier-groups with { name: 'Size', min_select: 1, max_select: 1, required: true, display_type: 'radio' }. Add modifiers (e.g. Small, Large) to the group. Attach the group to a product via PUT /api/owner/locations/:locationId/products/:productId/modifier-groups. Repeat for a second group with display_type='checkbox' (max_select>1).
    - expect: Modifier groups created and attached to product; product appears on /s/demo
  2. Navigate to https://dowiz-staging.fly.dev/s/demo. Click the product card that now has modifier groups. Wait for the modal dialog to open.
    - expect: Modal (role=dialog) is visible
    - expect: Modal title matches the product name
  3. Assert at least one [data-testid='modifier-group'] element is visible inside the modal.
    - expect: expect(page.locator('[data-testid="modifier-group"]').first()).toBeVisible()
  4. Assert the radio group has data-display-type='radio'.
    - expect: expect(page.locator('[data-testid="modifier-group"][data-display-type="radio"]')).toBeVisible()
  5. Assert the checkbox group has data-display-type='checkbox'.
    - expect: expect(page.locator('[data-testid="modifier-group"][data-display-type="checkbox"]')).toBeVisible()
  6. Verify that a group without an explicit display_type and max_select=1 defaults to data-display-type='radio' (inferred fallback).
    - expect: Inferred group renders with data-display-type='radio' matching the resolveDisplayType fallback logic

#### 4.2. 4b — modifier-group with display_type='select' renders a <select> control [BLOCKED: needs seeded data]

**File:** `e2e/tests/client/modifier-display-type.spec.ts`

**Steps:**
  1. SEED OPERATION: Create a modifier group with display_type='select' attached to a product on the demo location.
    - expect: Group with display_type='select' created
  2. Open the product modal on /s/demo. Locate [data-testid='modifier-group'][data-display-type='select'].
    - expect: expect(page.locator('[data-testid="modifier-group"][data-display-type="select"]')).toBeVisible()
  3. Assert a native <select> element is rendered inside that modifier-group.
    - expect: expect(page.locator('[data-testid="modifier-group"][data-display-type="select"] select')).toBeVisible()

#### 4.3. 4c — modifier-group with display_type='quantity' renders quantity controls [BLOCKED: needs seeded data]

**File:** `e2e/tests/client/modifier-display-type.spec.ts`

**Steps:**
  1. SEED OPERATION: Create a modifier group with display_type='quantity' and attach to a product.
    - expect: Group with display_type='quantity' created
  2. Open the product modal on /s/demo. Locate [data-testid='modifier-group'][data-display-type='quantity'].
    - expect: expect(page.locator('[data-testid="modifier-group"][data-display-type="quantity"]')).toBeVisible()
  3. Assert quantity stepper controls (increment / decrement buttons) are present inside the modifier group.
    - expect: Increment and decrement buttons visible within the quantity modifier group

### 5. 5 — Owner Menu Schedule + Kitchen-Busy (/admin/menu)

**Seed:** `e2e/tests/smoke.spec.ts`

#### 5.1. 5a — schedule-editor renders collapsed on /admin/menu [GO]

**File:** `e2e/tests/admin/menu-schedule.spec.ts`

**Steps:**
  1. Log in as test@dowiz.com / test123456 and navigate to https://dowiz-staging.fly.dev/admin/menu.
    - expect: Menu manager page loads; category list or product grid is visible
  2. Assert [data-testid='schedule-editor'] is visible.
    - expect: expect(page.locator('[data-testid="schedule-editor"]')).toBeVisible()
  3. Assert the schedule editor is in its collapsed state: the chevron-down icon is visible and no time-input fields are shown.
    - expect: Schedule editor shows 'Availability schedules (mealtimes)' label
    - expect: No time inputs visible (panel is collapsed by default)

#### 5.2. 5b — schedule-editor expands and shows form controls [GO]

**File:** `e2e/tests/admin/menu-schedule.spec.ts`

**Steps:**
  1. Log in as test@dowiz.com and navigate to /admin/menu. Click the [data-testid='schedule-editor'] toggle button.
    - expect: Clicking the button reveals the schedule form
  2. Assert that a <select> for category, a time input for 'From', and a time input for 'To' are all visible inside the schedule editor.
    - expect: expect(page.locator('[data-testid="schedule-editor"] select')).toBeVisible()
    - expect: expect(page.locator('[data-testid="schedule-editor"] input[type="time"]').first()).toBeVisible()

#### 5.3. 5c — Add a schedule window and confirm it persists [GO]

**File:** `e2e/tests/admin/menu-schedule.spec.ts`

**Steps:**
  1. Log in as test@dowiz.com, navigate to /admin/menu, expand the schedule-editor. Select the first available category in the category dropdown.
    - expect: A category is selected
  2. Set From time to '07:00' and To time to '11:00'. Click the 'Add window' button.
    - expect: A success toast appears containing 'Availability window saved' or 'schedule saved'
    - expect: The schedule entry appears in the list below the form
  3. Assert the new schedule row is visible with the category name and '07:00–11:00' time range.
    - expect: Schedule list contains an item with '07:00' and '11:00'
  4. CLEANUP: Click the delete (trash) button on the new schedule row to remove it.
    - expect: Schedule row is removed from the list

#### 5.4. 5d — Kitchen-busy toggle (API only — no UI testid present on staging) [BLOCKED: no data-testid='kitchen-busy-toggle' in admin UI]

**File:** `e2e/tests/admin/menu-schedule.spec.ts`

**Steps:**
  1. NOTE: The kitchen-busy feature is backend-only in the current build. The API endpoint PATCH /api/owner/locations/:locationId/kitchen-busy exists and is confirmed in source, but there is no data-testid='kitchen-busy-toggle' in the admin UI (confirmed by live DOM inspection of /admin/menu). This test is BLOCKED until a UI control with data-testid='kitchen-busy-toggle' is added to the MenuManagerPage.
    - expect: BLOCKED: Add a kitchen-busy toggle to MenuManagerPage with data-testid='kitchen-busy-toggle' before this test can run
    - expect: WORKAROUND: Test the busy state end-to-end via API: POST kitchen-busy, then assert venue-busy-banner on /s/demo (see test 1c)

### 6. 6 — Courier Offer Timer (/courier)

**Seed:** `e2e/tests/smoke.spec.ts`

#### 6.1. 6a — courier-offer-timer renders with data-remaining when a task is offered [BLOCKED: needs courier session + dispatched offer]

**File:** `e2e/tests/courier/offer-timer.spec.ts`

**Steps:**
  1. SEED OPERATION: Obtain a courier JWT. Options: (A) Use POST /api/auth/local/login with courier credentials (need a courier account on staging). (B) Use the dev backdoor endpoint /api/dev/repair-test-owner to establish a courier identity. (C) Create a courier invite via the owner dashboard at /admin/couriers and complete onboarding. Once authenticated, store the JWT in localStorage dos_access_token.
    - expect: Courier JWT obtained and stored
  2. SEED OPERATION: Dispatch a delivery order to the courier. Place a new order on /s/demo as a customer. As the owner, confirm it (CONFIRMED), set it to PREPARING, then READY. The dispatch system should offer it to the courier. Alternatively, directly PATCH the assignment to 'offered' status for the courier ID.
    - expect: Assignment in 'offered' state exists for the courier
  3. Navigate to https://dowiz-staging.fly.dev/courier (the TasksPage). Wait for the task card to appear.
    - expect: At least one task card is visible on the courier tasks page
  4. Assert [data-testid='courier-offer-timer'] is visible with a data-remaining attribute containing a positive integer.
    - expect: expect(page.locator('[data-testid="courier-offer-timer"]')).toBeVisible()
    - expect: page.locator('[data-testid="courier-offer-timer"]').getAttribute('data-remaining') resolves to a string parseable as a positive number
  5. Wait 3 seconds. Assert data-remaining has decreased (the countdown is ticking).
    - expect: data-remaining value after 3s is less than the initial value (countdown is active)

#### 6.2. 6b — task-accept button is visible and accepting navigates to delivery page [BLOCKED: needs courier session + offered task]

**File:** `e2e/tests/courier/offer-timer.spec.ts`

**Steps:**
  1. With a courier session and an offered task on the tasks page (per seed in 6a), assert [data-testid='task-accept'] is visible.
    - expect: expect(page.locator('[data-testid="task-accept"]')).toBeVisible()
  2. Click [data-testid='task-accept'].
    - expect: Navigation occurs to /courier/delivery/:assignmentId
  3. Assert the delivery page loaded (URL contains /courier/delivery/).
    - expect: expect(page.url()).toContain('/courier/delivery/')

#### 6.3. 6c — courier-offer-decline releases the task and hides the card [BLOCKED: needs courier session + offered task]

**File:** `e2e/tests/courier/offer-timer.spec.ts`

**Steps:**
  1. With a courier session and an offered task on the tasks page, assert [data-testid='courier-offer-decline'] is visible.
    - expect: expect(page.locator('[data-testid="courier-offer-decline"]')).toBeVisible()
  2. Click [data-testid='courier-offer-decline'].
    - expect: The task card is removed from the tasks list (optimistic removal)
    - expect: API call POST /api/courier/assignments/:id/reject returns 200
  3. Assert the tasks list shows the empty state or no task card for the declined task.
    - expect: Task card is no longer visible in the list

#### 6.4. 6d — courier-offer-timer auto-declines when it reaches zero [BLOCKED: needs courier session + offered task]

**File:** `e2e/tests/courier/offer-timer.spec.ts`

**Steps:**
  1. SEED OPERATION: Create an assignment offer with a very short offerSeconds window (e.g. 10 seconds). This requires a test fixture or a way to control offerSeconds via the API or test harness.
    - expect: Task card appears with timer set to ~10 seconds
  2. Assert [data-testid='courier-offer-timer'] is visible with data-remaining close to 10.
    - expect: Timer is visible; data-remaining is approximately 10
  3. Wait 12 seconds (timer plus buffer). Assert the task card is no longer visible (auto-decline fired via the onReject callback).
    - expect: Task card disappears from the list after the countdown completes
    - expect: POST /api/courier/assignments/:id/reject was called (check network request or server state)

#### 6.5. 6e — courier-advance-action testid [BLOCKED: testid does not exist in codebase]

**File:** `e2e/tests/courier/offer-timer.spec.ts`

**Steps:**
  1. NOTE: The testid 'courier-advance-action' was specified in the task brief but does NOT currently exist anywhere in the codebase (confirmed by exhaustive grep of /root/dowiz). The DeliveryPage (/courier/delivery/:id) has no data-testid='courier-advance-action'. Existing testids on the DeliveryPage are: 'entry-photo-thumb', 'entry-photo-modal', 'task-tip', 'task-cash-amount', 'message-customer-btn'. The SwipeToComplete component (used for delivery completion) does not carry this testid.
    - expect: BLOCKED: 'courier-advance-action' testid must be added to the courier delivery advance/status control before this test can run
    - expect: Recommended location: the 'picked up from restaurant' or 'mark delivered' action button on DeliveryPage
