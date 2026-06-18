# Client Checkout Flow — DeliveryOS

## Application Overview

DeliveryOS is a multi-tenant food-delivery SaaS. The public client ordering journey lives entirely under the SSR route /s/:slug. The funnel is: browse menu → add item to cart → open cart drawer → proceed to checkout → fill contact/address/payment details → place order → land on order-status page. The slug used throughout this plan is "test-slug" (with the ?dev=true flag), which is the seeded restaurant slug already wired to the dev mock-auth endpoint and consistently used across the existing client test suite. In dev mode the app bypasses the real API call on order failure and navigates to /s/test-slug/order/o_mock_123. All routes follow the pattern /s/:slug (menu), /s/:slug/checkout, and /s/:slug/order/:id.

## Test Scenarios

### 1. Happy Path — Browse to Order Placed

**Seed:** `e2e/tests/seed.spec.ts`

#### 1.1. Full funnel: browse menu → add item → open cart → checkout → order placed

**File:** `specs/client-checkout.spec.ts`

**Steps:**
  1. Navigate to https://dowiz.fly.dev/s/test-slug?dev=true and wait for product cards to load.
    - expect: URL contains /s/test-slug
    - expect: At least one article.product-card element is visible
    - expect: The hero section h1 contains the restaurant name (e.g. 'Dubin')
    - expect: The sticky category nav renders at least 2 tab buttons
    - expect: The cart FAB (#cartFabBtn) is NOT visible (cart is empty)
  2. Click the 'Add' button (aria-label='Add') on the first visible product card.
    - expect: The cart FAB (#cartFabBtn) becomes visible within 5 seconds
    - expect: The FAB label shows '1' (one item in cart)
  3. Click the cart FAB (#cartFabBtn) to open the cart drawer.
    - expect: A heading containing 'Your Cart' (or 'Shporta') is visible
    - expect: The added item name appears in the drawer
    - expect: A total amount line is visible in the drawer
    - expect: A 'Checkout' button is visible in the drawer
  4. Click the 'Checkout' button inside the cart drawer.
    - expect: The URL changes to match /checkout (i.e. /s/test-slug/checkout)
    - expect: The page heading 'Checkout' (or localised equivalent) is visible
    - expect: A 'Contact Info' section is visible with Name and Phone inputs
    - expect: A delivery-type tab bar is visible with 'Delivery', 'Pickup', and 'Scheduled' tabs
    - expect: The 'Delivery' tab is selected by default (aria-selected=true)
    - expect: An address input field is visible
    - expect: Entrance and Apartment inputs are visible
    - expect: A 'How to find you' textarea (notes) is visible
    - expect: Dropoff instruction buttons are visible (e.g. 'Leave at door', 'Call on arrival')
    - expect: A 'Cash' payment section is visible with a cash-amount number input (id=cash-amount)
    - expect: An order summary section shows Subtotal, Delivery fee (200 ALL), and Total in ALL currency
    - expect: A sticky 'Place order' button (data-testid=order-confirm-button) is visible at the bottom
  5. Fill in the Name input (placeholder 'Your name') with 'Test Customer'.
    - expect: The Name input displays 'Test Customer'
  6. Fill in the Phone input (data-testid=checkout-phone) with '+355691234567'.
    - expect: The Phone input displays '+355691234567'
    - expect: No phone-error alert is visible
  7. Fill in the address input with 'Rruga e Durrësit, Tirana'.
    - expect: The address input displays the entered value
  8. Fill in the Entrance input (data-testid=checkout-entrance) with '2'.
    - expect: The Entrance input displays '2'
  9. Fill in the Apartment input (data-testid=checkout-apartment) with '5'.
    - expect: The Apartment input displays '5'
  10. Fill in the 'How to find you' textarea with 'Blue gate, third floor, ring the bell'.
    - expect: The textarea displays the entered text
  11. Click the 'Place order' sticky button (data-testid=order-confirm-button).
    - expect: The button briefly shows a spinner and 'Placing order...' text while the request is in flight
    - expect: A full-screen confirmation overlay appears containing 'Order placed!' (or localised variant) with a checkmark SVG animation
    - expect: Within ~1.5 seconds the URL changes to /s/test-slug/order/<orderId> (or /s/test-slug/order/o_mock_123 in dev mode)
    - expect: The order-status page body is visible and contains text matching one of: PENDING, CONFIRMED, Preparing, placed, confirmed (case-insensitive)

### 2. Edge Case — Empty Cart Cannot Checkout

**Seed:** `e2e/tests/seed.spec.ts`

#### 2.1. Navigating directly to /checkout with an empty cart shows 'Cart is empty' state

**File:** `specs/client-checkout-edge.spec.ts`

**Steps:**
  1. Navigate directly to https://dowiz.fly.dev/s/test-slug/checkout?dev=true without adding any items to the cart first (fresh session, no localStorage cart).
    - expect: The checkout form is NOT rendered
    - expect: A message containing 'Cart is empty' (or localised variant, key cart.empty) is visible
    - expect: A 'Back' button is visible below the empty-cart message
    - expect: The sticky 'Place order' button is NOT present in the DOM
  2. Click the 'Back' button shown on the empty-cart screen.
    - expect: The URL changes back to /s/test-slug (the menu page)
    - expect: Product cards (article.product-card) are visible

### 3. Edge Case — Required Field Validation Blocks Submission

**Seed:** `e2e/tests/seed.spec.ts`

#### 3.1. Submitting checkout form with invalid phone shows inline error and prevents order placement

**File:** `specs/client-checkout-edge.spec.ts`

**Steps:**
  1. Navigate to https://dowiz.fly.dev/s/test-slug?dev=true and add one item to the cart.
    - expect: The cart FAB (#cartFabBtn) is visible showing '1'
  2. Click the cart FAB, then click 'Checkout' to navigate to /s/test-slug/checkout.
    - expect: URL matches /checkout
    - expect: The checkout form is visible with Contact Info section
  3. Fill the Name input with 'Test Customer', leave the Phone input (data-testid=checkout-phone) empty, fill address with 'Test St', entrance with '1', apartment with '1', and the notes textarea with 'near the red door'. Then click the 'Place order' button (data-testid=order-confirm-button).
    - expect: Order is NOT placed — no confirmation overlay appears
    - expect: A phone-error message is visible (role=alert) containing text matching 'valid phone' or '+355' hint
    - expect: The URL remains at /checkout (no navigation away)
  4. Now type an invalid phone number 'abc123' into the Phone input and click 'Place order' again.
    - expect: The phone-error alert remains visible (pattern validation enforces E.164 format)
    - expect: No order is placed, URL stays at /checkout
  5. Clear the Phone input and type a valid E.164 number '+355691234567', then click 'Place order'.
    - expect: The phone-error alert is no longer visible
    - expect: The form proceeds to placement (spinner visible on button), confirming the validation was the only blocker
