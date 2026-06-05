# Operators Runbook

## Dead-Channel Recovery
If a user temporarily deletes their chat or the bot fails to contact them 5 times in a row, the target `status` is set to `disabled`.
**To Recover:** In the admin panel UI, click "Re-enable" to update the target `status` back to `active`.

## Scaling Workers
The Telegram Long Polling (`getUpdates`) worker operates sequentially by design (offset-based). To scale polling horizontally, we would need to switch to Webhooks (Phase 4).
However, the `pg-boss` workers processing `notify.dispatch` can scale horizontally freely because they operate via standard job queues.

## Push Tokens Key Rotation
Push tokens are AES-GCM encrypted using the environment variable `NOTIFICATION_ENCRYPTION_KEY`.
Rotating this key requires reading all entries using the old key, decrypting them, encrypting with the new key, and saving them. (Planned for Phase 4).
