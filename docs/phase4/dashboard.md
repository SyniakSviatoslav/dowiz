# Phase 4 — Live Dashboard

## Architecture

The Live Dashboard (E24) provides real-time order tracking for restaurant owners via a WebSocket-backed kanban interface.

### Components

```
┌─────────────────────────────────────────────────────────┐
│                    Browser (Owner)                       │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │  Dashboard    │  │  Orders      │  │  Active Del.  │   │
│  │  (kanban)     │  │  (table)     │  │  (list)       │   │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘   │
│         │                 │                 │            │
│    ┌────┴─────────────────┴─────────────────┴────┐       │
│    │           dashboard.js (WS client)            │       │
│    └────────────────────┬──────────────────────────┘       │
└─────────────────────────┼──────────────────────────────────┘
                          │
         ┌────────────────┼────────────────┐
         │   WS (push)    │   REST (snap)  │
         ▼                ▼                ▼
┌─────────────────────────────────────────────────────────┐
│                    API Server (Fastify)                   │
│  ┌────────────────┐  ┌────────────────────────────────┐   │
│  │  WebSocket     │  │  /api/owner/locations/:id/     │   │
│  │  / → ws auth   │  │  dashboard/snapshot            │   │
│  │  → subscribe   │  │  /orders/:id/confirm           │   │
│  └───────┬────────┘  │  /orders/:id/reject            │   │
│          │            │  /orders/:id/assign-courier    │   │
│          │            └────────────────────────────────┘   │
│     ┌────┴─────┐                                          │
│     │MessageBus│                                          │
│     │(Pg NOTIFY)│                                         │
│     └────┬─────┘                                          │
└──────────┼────────────────────────────────────────────────┘
           │
     ┌─────┴─────┐
     │PostgreSQL  │
     │(LISTEN/    │
     │ NOTIFY)    │
     └───────────┘
```

### Data Flow

1. **Initial load**: HTML → GET `/api/owner/locations/:id/dashboard/snapshot` → render kanban
2. **Live updates**: WS subscribe to `location:{id}:dashboard` room → events via MessageBus
3. **Actions**: POST confirm/reject → DB update → MessageBus publish → all WS clients receive event
4. **Reconnect**: WS disconnect → exponential backoff → snapshot fetch → incremental WS

## WS Contract

### Connection
- URL: `ws://{host}/` (same origin, WS upgrade from HTTP)
- Auth: `{ type: 'auth', token: '<owner-jwt>' }`
- Subscribe: `{ type: 'subscribe', room: 'location:{locationId}:dashboard' }`

### Events (server → client)

| Type | Payload | Trigger |
|------|---------|---------|
| `order.created` | `{ orderId, status, total, currency, createdAt, ... }` | New order placed |
| `order.confirmed` | `{ orderId, status, statusUpdatedAt }` | Owner confirmed |
| `order.rejected` | `{ orderId, status, statusUpdatedAt }` | Owner rejected |
| `order.preparing` | `{ orderId, status, statusUpdatedAt }` | Status → PREPARING |
| `order.in_delivery` | `{ orderId, status, statusUpdatedAt }` | Status → IN_DELIVERY |
| `order.delivered` | `{ orderId, status, statusUpdatedAt }` | Status → DELIVERED |
| `order.cancelled` | `{ orderId, status, statusUpdatedAt }` | Status → CANCELLED |
| `courier.assignment_created` | `{ orderId, courierId }` | Courier assigned |

### PII Masking
All customer names/phones are masked before broadcast. No addresses, no coordinates, no raw emails.

## Snapshot Endpoint

`GET /api/owner/locations/:locationId/dashboard/snapshot`

Query params: `status=PENDING,CONFIRMED&limit=100&cursor=...`

Response includes:
- `serverTime` — UTC ISO 8601
- `counts` — per-status order counts
- `orders[]` — masked PII, dwellSeconds (computed server-side)
- `activeDeliveries[]` — courier info (masked), distance, ETA
- `nextCursor` — for pagination

## Reconnect Flow

1. WS disconnected → banner "Reconnecting..."
2. Backoff: 1s, 2s, 4s, 8s, 16s, max 30s (jitter ±50%)
3. On reconnect: `GET snapshot` (full resync) → WS incremental
4. After 6 failed attempts → banner "Connection lost" + manual reload
