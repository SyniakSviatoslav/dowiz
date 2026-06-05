# Status Page Architecture

- **WebSocket Live**: Connects to `wss://.../ws/orders/:id?token=<jwt>`.
- **Reconnect Strategy**: Exponential backoff (`Math.pow(2, attempts) * jitter`). Maximum 30s.
- **Reconcile**: After reconnecting, unconditionally calls `GET /api/orders/:id` to resync truth.
