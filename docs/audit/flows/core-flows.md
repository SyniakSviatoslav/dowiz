# Flow: Customer Order (menu → cart → checkout → status)

```mermaid
sequenceDiagram
    actor C as Client
    participant B as Browser (localStorage)
    participant API as Fastify API
    participant DB as Postgres (RLS)
    participant Q as pg-boss
    participant WS as WebSocket Server
    
    Note over C,B: ── Menu Browse ──
    C->>API: GET /public/locations/:slug/menu
    API->>DB: SELECT categories, products (public RLS)
    API-->>C: menu JSON (categories + products + prices)

    Note over C,B: ── Cart (localStorage) ──
    C->>B: addItem({productId, quantity, options})
    B->>B: CartProvider.setItems (CART_SCHEMA_VERSION=1)
    B->>B: localStorage.setItem(dos_cart_<slug>, {version:1, items})

    Note over C,WS: ── Checkout ──
    C->>API: POST /customer/otp/send {phone, order_intent}
    API->>DB: INSERT phone_otp (code_hash, expires_at)
    API-->>C: {otp_token}

    C->>API: POST /customer/otp/verify {phone, code, otp_token}
    API->>DB: SELECT code_hash, check expires/attempts
    API-->>C: {verified: true}

    C->>API: POST /orders {items, fulfillment, X-Idempotency-Key}
    Note over API: Zod .strict() + server price lookup
    API->>DB: BEGIN tx
    API->>DB: INSERT orders (status=PENDING, server-calc total)
    API->>DB: INSERT order_items (price_snapshot from products)
    API->>DB: INSERT idempotency_keys (key, location_id, request_hash)
    API->>Q: enqueue order.timeout {orderId, locationId} (outbox)
    API->>DB: COMMIT tx
    API-->>C: 201 + customer-JWT {orderId, locationId}
    API->>WS: publish location:{id}:dashboard (new order)

    Note over C,WS: ── Order Status (WS live) ──
    C->>API: GET /orders/:id
    C->>WS: connect + subscribe order:{id}
    WS-->>C: status updates, courier position, ETA
```

---

## Flow: Order Lifecycle (State Machine)

```mermaid
stateDiagram-v2
    [*] --> PENDING: POST /orders
    PENDING --> CONFIRMED: Owner confirm
    PENDING --> REJECTED: Owner reject
    PENDING --> CANCELLED: timeout (pg-boss durable)
    
    CONFIRMED --> PREPARING: Owner/Courier transition
    PREPARING --> READY: Owner mark ready
    READY --> ASSIGNED: Owner assign courier
    ASSIGNED --> IN_DELIVERY: Courier picked up
    IN_DELIVERY --> DELIVERED: Courier delivered
    
    REJECTED --> [*]
    CANCELLED --> [*]
    DELIVERED --> [*]
    
    note right of PENDING
        timeout_at = now + config
        pg-boss order.timeout enqueued
        claim-check: only {orderId, locationId}
    end note
    
    note right of CONFIRMED
        requireRole('owner')
        assertTransition('CONFIRMED')
        UPDATE WHERE status='PENDING' → 0 rows = 409
    end note
```

---

## Flow: Courier Delivery (assign → pickup → deliver)

```mermaid
sequenceDiagram
    actor O as Owner
    actor CR as Courier
    participant API as Fastify API
    participant DB as Postgres
    participant WS as WebSocket
    participant MB as MessageBus
    
    Note over O,DB: ── Assign ──
    O->>API: POST /owner/locations/:id/orders/:id/assign-courier {courierId}
    API->>DB: UPDATE orders SET courier_id=$1, status='ASSIGNED'
    API->>DB: INSERT courier_assignments
    API->>MB: publish courier.assigned
    MB->>WS: forward to courier:{id} room
    WS-->>CR: new assignment notification

    Note over CR,DB: ── Pickup ──
    CR->>API: POST /courier/assignments/:id/picked-up
    API->>DB: UPDATE orders SET status='IN_DELIVERY'
    API->>DB: UPDATE courier_assignments SET status='picked_up'
    API->>MB: publish order.status (IN_DELIVERY)
    MB->>WS: forward to order:{id} room
    WS-->>C: status update + courier info

    Note over CR,DB: ── GPS Stream ──
    loop Every position update
        CR->>WS: send({type:'location_update', payload:{lat,lng,heading,speed}})
        WS->>API: POST /courier/shifts/ping {lat, lng, accuracy_meters}
        API->>DB: INSERT courier_positions
        API->>MB: publish courier.position_updated
        MB->>WS: forward to order:{id} + location:{id}:couriers
    end

    Note over CR,DB: ── Deliver ──
    CR->>API: POST /courier/assignments/:id/delivered {cash_collected, cash_amount}
    API->>DB: UPDATE orders SET status='DELIVERED', payment_outcome
    API->>DB: UPDATE courier_assignments SET status='delivered'
    API->>MB: publish order.status (DELIVERED)
    WS-->>C: delivery complete
```

---

## Flow: Onboarding (8-step wizard)

```mermaid
sequenceDiagram
    actor O as Owner
    participant Web as React (Dowiz)
    participant API as Fastify API
    participant DB as Postgres

    Note over O,DB: ── Step 1: Restaurant + Slug ──
    O->>Web: Enter name, phone, slug
    Web->>API: POST /owner/onboarding/start {name, phone, slug}
    API->>DB: INSERT organizations + locations + memberships (1 tx)
    API->>DB: slug UNIQUE check + reserved blocklist
    API-->>Web: {locationId, slug}

    Note over O,DB: ── Step 2: Menu (import-first) ──
    O->>Web: Upload CSV / add manual / use demo
    Web->>API: POST /locations/:id/menu/import/preview (multipart)
    API->>DB: INSERT import_sessions (draft_json)
    API-->>Web: preview (categories + products)
    O->>Web: Review + confirm
    Web->>API: POST /locations/:id/menu/import/commit
    API->>DB: INSERT categories + products + bump menu_version

    Note over O,DB: ── Step 3: Location Pin + Radius ──
    O->>Web: Place pin on map, set radius
    Web->>API: PATCH /owner/locations/:id {lat, lng, delivery_radius_km}
    API->>DB: UPDATE locations

    Note over O,DB: ── Step 4: Courier (skippable) ──
    O->>Web: Choose: skip / invite / owner-as-courier
    Web->>API: POST /owner/courier-invites (if invite)
    API->>DB: INSERT courier_invites (code_hash, expires_at)

    Note over O,DB: ── Step 5: Branding (skippable) ──
    O->>Web: Pick colors + logo
    Web->>API: PUT /owner/locations/:id/theme
    API->>DB: UPSERT location_themes

    Note over O,DB: ── Step 6-7: Preview + Share ──
    O->>Web: Preview iframe + copy link
    Web->>Web: Show embed code + share link

    Note over O,DB: ── Step 8: Test Order + Go Live ──
    O->>Web: Place test order (is_test=true)
    Web->>API: POST /orders {is_test: true, ...}
    API->>DB: INSERT orders (is_test=true — excluded from analytics)
    O->>Web: Confirm publish
    Web->>API: POST /owner/onboarding/:id/complete
    API->>DB: UPDATE locations SET status='open'
```

---

## Flow: Durable Timeout (Order Cancellation)

```mermaid
sequenceDiagram
    participant API as Fastify API
    participant DB as Postgres
    participant Q as pg-boss Queue
    participant W as Worker Process

    Note over API,DB: ── Order Creation (Outbox Pattern) ──
    API->>DB: BEGIN tx
    API->>DB: INSERT orders (status=PENDING, timeout_at=now()+15min)
    API->>Q: enqueue order.timeout {orderId, locationId} WITH db: tx
    API->>DB: COMMIT tx
    Note over API,DB: Job committed atomically with order — no lost jobs

    Note over W,DB: ── Timeout Execution ──
    Q->>W: deliver order.timeout {orderId, locationId}
    W->>DB: SELECT * FROM orders WHERE id=$1 AND location_id=$2
    Note over W: Check: status == PENDING?
    alt status == PENDING
        W->>DB: UPDATE orders SET status='CANCELLED' WHERE id=$1 AND status='PENDING'
        Note over W: Status-guarded: 0 rows affected = already transitioned
        W->>DB: record cancellation as no-show signal (optional)
    else status != PENDING
        Note over W: Already transitioned — no-op (idempotent)
    end

    Note over W: Claim-check: payload only {orderId, locationId}
    Note over W: Re-fetches order inside job — no stale data
    Note over W: busy_mode: timeout window doubles during busy mode
```
