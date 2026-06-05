# Notification Architecture

The Notification Seam in DeliveryOS acts as an operations layer ensuring owners receive business-critical alerts without blocking the hot path of customer requests.

## 1. Zero-PII Policy & Claim Check
The worker payload enqueueing into `pg-boss` strictly adheres to a zero-PII policy.
Payload structure: `{ targetId, eventType, orderId, locationId, attempt }`.

Instead of packing customer details into the job (which would violate PII constraints for a long-lived queue), the `notify.dispatch` worker performs a **Claim Check**:
It reads the `orderId` and fetches the required details directly from the `orders` table under the scope of the given `locationId`, guaranteeing tenant isolation.

## 2. Notification Dispatcher
`NotificationDispatcher` is a Dependency Injection registry. Adapters implement the `NotificationProvider` interface. The dispatcher routes jobs to the appropriate adapter (`telegram`, `push`, etc.).

## 3. Dead-Channel Degradation
If a channel provider becomes dead (e.g., Telegram returning `401 Unauthorized` or `403 Forbidden` because the user blocked the bot), the target is immediately marked as `status = 'disabled'`.
Temporary failures (like `502 Bad Gateway` or `429 Too Many Requests`) are retried up to 5 times using an exponential backoff with jitter. If the retries are exhausted, the channel is disabled.

## 4. Escalation (order.pending_aging)
A background cron job runs every 5 minutes scanning for `orders` where `status = 'PENDING'` that have exceeded the `PENDING_AGING_THRESHOLD_MS`. It de-duplicates using `location_alerts` and dispatches `order.pending_aging` to notify the owner of the rotting order.
