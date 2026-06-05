# Key Rotation Procedure

## JWT RS256 Keys

### Adding a new signing key
1. Generate new RSA 2048-bit key pair:
   ```bash
   openssl genrsa -out jwt-v2-private.pem 2048
   openssl rsa -in jwt-v2-private.pem -pubout -out jwt-v2-public.pem
   ```
2. Add to env:
   - `JWT_PRIVATE_KEY=v2` (new private key PEM)
   - `JWT_PUBLIC_KEY=v1+v2` (both public keys concatenated or referenced)
   - `JWT_KID=v2`
3. Deploy. New tokens signed with `kid=v2`.
4. Old tokens with `kid=v1` still validate (v1 public key in set).

### Removing an old key
1. Wait until all tokens with `kid=v1` have expired (check `exp` claim).
2. Remove v1 public key from env.
3. Deploy. Tokens with `kid=v1` are now rejected.

### Emergency rotation (compromise)
1. Generate new key pair immediately.
2. Deploy new `JWT_PRIVATE_KEY` + `JWT_PUBLIC_KEY` + `JWT_KID`.
3. All existing sessions invalidated — force re-login.
4. Rotate VAPID, Telegram, R2 keys as well if compromise is broad.

## VAPID Keys (Web Push)
1. Generate: `npx web-push generate-vapid-keys`
2. Update `VAPID_PUBLIC_KEY` + `VAPID_PRIVATE_KEY` in env.
3. Deploy. Old push subscriptions will fail — clients re-subscribe on next interaction.

## Telegram Bot Token
1. Revoke old token in BotFather.
2. Generate new token.
3. Update `TELEGRAM_BOT_TOKEN` in env.
4. Deploy. Users need to re-connect via `/start`.

## R2 / S3 Keys
1. Generate new access key in Cloudflare dashboard.
2. Add to env alongside old key during transition.
3. Remove old key after confirming new key works.

## Supabase Service Key
1. Generate new service key in Supabase dashboard.
2. Update `SUPABASE_SERVICE_KEY` in env.
3. Deploy.

## Verification
```bash
pnpm test:phase5-step3  # JWT rotation test
pnpm verify:secrets     # No secrets in repo
```
