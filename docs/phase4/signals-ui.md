# Owner Signals UI

## Pages

### `/admin/signals`
- Active signals list (default), with toggle for acknowledged/dismissed.
- Per signal: kind, severity (color-coded), customer name (masked), evidence details (count, age, decay, window).
- Actions: [Acknowledge] [Dismiss].
- Filter by kind, severity.
- WS live updates — new signals appear in real-time.

### `/admin/settings-security`
- OTP toggle: `require_phone_otp` boolean switch.
- Confirmation modal when enabling: "Enabling OTP requires customers to verify their phone before ordering."
- Saved to `locations.require_phone_otp`.

### Dashboard inline banner
- "N active signals" banner at top of dashboard when signals exist.
- Click → navigate to `/admin/signals`.
- WS event `preflight.signal_raised` → banner shows.
- WS events `preflight.signal_acknowledged` / `preflight.signal_dismissed` → banner updates.

## API Endpoints

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/owner/locations/:id/signals` | Owner | List signals with pagination, filter by status/kind |
| GET | `/api/owner/locations/:id/signals/compute` | Owner | Read-only signal computation (what-if) |
| POST | `/api/owner/locations/:id/signals/:id/acknowledge` | Owner | Acknowledge signal, shift decay |
| POST | `/api/owner/locations/:id/signals/:id/dismiss` | Owner | Dismiss signal, mark reviewed |
| POST | `/api/owner/locations/:id/orders/:id/mark-no-show` | Owner | Manual no-show counter increment |

## WS Events

| Event | Payload | Description |
|-------|---------|-------------|
| `preflight.signal_raised` | `{ signalId, customerId, kind, severity }` | New signal detected |
| `preflight.signal_acknowledged` | `{ signalId }` | Owner acknowledged |
| `preflight.signal_dismissed` | `{ signalId }` | Owner dismissed |
