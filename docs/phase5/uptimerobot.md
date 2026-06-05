# UptimeRobot Monitoring Configuration (P31)

## Overview

Three monitors covering critical platform paths. Alerts route to Telegram + email.

## Monitor 1: API Health Check

| Field | Value |
|-------|-------|
| **Type** | HTTP(S) |
| **URL** | `https://api.dowiz.org/health` |
| **Interval** | 60 seconds |
| **Timeout** | 5 seconds |
| **Keyword** | `"status":"healthy"` → assert exists |
| **Alert Contacts** | Telegram group + Email ops@dowiz.org |
| **Regions** | US East, EU West, Asia Pacific |

**Notes:**
- Returns `200` for `healthy` and `degraded` states
- Returns `503` for `unhealthy` (DB down)
- Keyword assert ensures `healthy` — if degraded, keyword fails → alert
- No auth required, no PII in response

## Monitor 2: Public Menu (Tenant)

| Field | Value |
|-------|-------|
| **Type** | HTTP(S) |
| **URL** | `https://margherita.dowiz.org/s/demo` |
| **Interval** | 5 minutes |
| **Timeout** | 10 seconds |
| **Keyword** | `DeliveryOS` → assert exists |
| **Alert Contacts** | Telegram group + Email ops@dowiz.org |
| **Regions** | US East, EU West, Asia Pacific |

**Notes:**
- Verifies SSR rendering is functional
- Slow due to SSR → 10s timeout
- `margherita` is a known demo tenant

## Monitor 3: Landing Page

| Field | Value |
|-------|-------|
| **Type** | HTTP(S) |
| **URL** | `https://www.dowiz.org/` |
| **Interval** | 5 minutes |
| **Timeout** | 10 seconds |
| **Port** | 443 |
| **Alert Contacts** | Telegram group + Email ops@dowiz.org |
| **Regions** | US East, EU West, Asia Pacific |

**Notes:**
- Verifies main landing page is up
- User-facing critical path

## Monitor 4: SSL Certificate

| Field | Value |
|-------|-------|
| **Type** | SSL (Port 443) |
| **Domain** | `*.dowiz.org` |
| **Interval** | 24 hours |
| **Alert on** | 30 days before expiry |
| **Alert Contacts** | Telegram group + Email ops@dowiz.org |

## Alert Contacts

| Channel | Target | Notes |
|---------|--------|-------|
| **Telegram** | `t.me/+...` (Ops group) | Primary, instantaneous |
| **Email** | `ops@dowiz.org` | Secondary, archival |

## Setup Instructions

1. Create UptimeRobot account (ops@dowiz.org)
2. Create Telegram bot integration: [UptimeRobot Telegram Setup](https://uptimerobot.com/integrations/#telegram)
3. Add contacts:
   - Telegram: bot token from `***REDACTED***`
   - Email: ops@dowiz.org
4. Create each monitor per the tables above
5. Verify: manually trigger alert by stopping API → confirm Telegram notification arrives within 60s

## Response SLA

| Severity | Response Time | Channel |
|----------|---------------|---------|
| **P0** — unhealthy (503) | < 5 min | Telegram @ops |
| **P1** — degraded (200 with degraded) | < 15 min | Telegram @ops |
| **P2** — SSL expiry < 14 days | < 24h | Email ops |
