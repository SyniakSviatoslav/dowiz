# Phase 4 — Owner Actions

## Endpoints

### Confirm Order
`POST /api/owner/locations/:locationId/orders/:orderId/confirm`

- Body: `{}`
- Status guard: only `PENDING → CONFIRMED`
- Rate limit: 30/min per owner
- 200/409/404/429

### Reject Order
`POST /api/owner/locations/:locationId/orders/:orderId/reject`

- Body: `{ reason?: string }`
- Status guard: `PENDING | CONFIRMED | PREPARING | SCHEDULED → REJECTED`
- Rate limit: 30/min per owner
- 200/409/404/429

### Assign Courier (Manual Override)
`POST /api/owner/locations/:locationId/orders/:orderId/assign-courier`

- Body: `{ courierId: uuid }`
- Preconditions: order CONFIRMED or PREPARING, courier active + available
- Creates courier_assignment + shift → `on_delivery` + order → `IN_DELIVERY`
- Rate limit: 10/min per owner
- 200/409/404/429

## Design Principles

- **State-guarded**: All transitions validated against order lifecycle. UI may show buttons, but server enforces.
- **Race-safe**: Conditional `UPDATE WHERE status = X` with `RETURNING id` → 409 if race lost.
- **Tenant-guarded**: 404 (not 403) on cross-tenant attempts.
- **Rate-limited**: Configurable per-action limits prevent abuse.
- **Event broadcast**: Each action publishes to `location:{id}:dashboard` and `order:{id}` channels.

## Rate Limits

| Action | Limit | Time Window |
|--------|-------|-------------|
| Confirm | 30 | 1 minute |
| Reject | 30 | 1 minute |
| Assign Courier | 10 | 1 minute |

## Tenant Isolation

All owner actions check `user.activeLocationId === locationId`. On mismatch → `404 Not found` (no location existence leak).

## Dashboard Events

Each action publishes to `location:{locationId}:dashboard` MessageBus channel:
- Confirm → `{ type: 'order.confirmed', data: { orderId, status, statusUpdatedAt } }`
- Reject → `{ type: 'order.rejected', data: { orderId, status, statusUpdatedAt } }`
- Assign → `{ type: 'courier.assignment_created', data: { orderId, courierId } }`
