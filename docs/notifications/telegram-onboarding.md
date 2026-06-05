# Telegram Onboarding

We use a one-time deep-link setup flow for maximum convenience and security.

1. **Token Generation:** The owner navigates to the admin panel and clicks "Connect Telegram". The API creates a single-use UUID (`connect_token`) with a 10-minute TTL.
2. **Deep-Link:** The admin UI displays a QR code or deep link: `https://t.me/dowiz_bot?start={connect_token}`.
3. **Bot Interaction:** The user starts the bot. The internal Telegram polling worker receives the `/start {token}` command.
4. **Validation:** The token is validated against the `telegram_connect_tokens` table. If valid, the target `owner_notification_targets` is upserted with `status = 'active'`, and the token is immediately marked as `used_at = now()`.

This process eliminates the need to manually figure out `chat_id`s, while preventing abuse or spoofing.
