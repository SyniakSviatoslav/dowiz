# Owner Auth — Launch Guide (G1)

## Problem

Google OAuth consent screen is **unverified** for `dowiz.org`. This means:
- Users see "Google hasn't verified this app" warning
- Only **test users** (≤100) can log in via Google OAuth
- Sensitive scopes are blocked

We cannot wait for Google verification. Owner login must work at launch.

## Solution: Dual Path

### Primary: Email + Password (Phase 0 legacy)

The `auth.ts` route already supports email+password login (Phase 0 scaffolding). For launch:

1. **Owner account created manually** via DB seed or supabase dashboard
2. **Owner logs in with email + password** at `/auth/login` (or admin console)
3. After first login, owner can configure Google sign-in as a convenience

This path has **zero dependency on Google OAuth verification**.

### Fallback: Google Test Users (≤100)

If the owner prefers Google sign-in:

1. Add owner's Google email to **OAuth test users** in Google Cloud Console
2. Owner logs in via `/auth/google` — sees unverified warning but proceeds
3. No sensitive scopes used (only `openid email profile`)

## Implementation

### Auth routes needed

The existing `auth.ts` already provides:
- `GET /auth/google` — redirect to Google OAuth
- `GET /auth/google/callback` — OAuth callback
- `POST /auth/exchange` — code exchange for tokens
- `POST /auth/refresh` — token refresh

Missing (for non-Google path):
- `POST /auth/login` — email+password login → JWT tokens
- `POST /auth/signup` — owner registration (if needed)

### For launch, manual account creation

```bash
# Seed an owner account directly
pnpm seed --owner-email=owner@example.com --owner-password=<secure-password>
```

### Token format (shared)

Both paths issue the same token format:
- `access_token`: RS256 JWT, 15m TTL, `{ role: 'owner', userId }`
- `refresh_token`: opaque 32B hex, 7d TTL, family-based rotation

## Gate for Multi-Location

**OAuth verification is a scaling gate.** Before onboarding a second location:
1. Complete Google OAuth verification for `dowiz.org`
2. Remove test-user restriction
3. Remove unverified-app warning
4. Keep email+password as a secondary path

## Risks

- **Password management**: No self-service password reset yet. Owner must contact dev to reset.
- **Credential storage**: Password hash stored in `users.password_hash` column (bcrypt). Ensure column exists in production DB.
- **Session revocation**: No per-user session management UI. Full revocation requires DB query.

## Verification

```bash
# Test non-Google login (requires running server)
curl -X POST http://localhost:8080/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"email": "owner@example.com", "password": "..."}'
# Expect: { access_token, refresh_token }
```
