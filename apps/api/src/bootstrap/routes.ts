// Core route registration extracted verbatim from server.ts main() (the
// contiguous fastify.register block). This is a mechanical move — same order,
// same options, same flag gate, same dynamic localAuthRoutes import — so the
// boot file's main() shrinks and route wiring lives in one named place. Order is
// load-bearing (Fastify), so the sequence here is unchanged; order-sensitive tail
// registrations (telegram webhook, mock-auth, the spa-proxy catch-all, admin
// routes) stay in main() AFTER this call.
import type { FastifyInstance, FastifyPluginAsync } from 'fastify';
import type { Pool } from 'pg';
import type { Env } from '@deliveryos/config';
import type { MessageBus, PgBossQueueProvider } from '@deliveryos/platform';
import type { MenuParserProvider, StorageProvider, TranslationProvider } from '../ports.js';

import authRoutes from '../routes/auth.js';
import courierRoutes from '../routes/couriers.js';
import orderRoutes from '../routes/orders.js';
import categoryRoutes from '../routes/owner/categories.js';
import productRoutes from '../routes/owner/products.js';
import menuConfirmRoutes from '../routes/owner/menu-confirm.js';
import publicClaimRoutes from '../routes/public/claim.js';
import modifierGroupRoutes from '../routes/owner/modifier-groups.js';
import menuAvailabilityRoutes from '../routes/owner/menu-availability.js';
import locationRoutes from '../routes/owner/locations.js';
import publicMenuRoutes from '../routes/public/menu.js';
import ssrRoutes from '../routes/public/ssr.js';
import ogCardRoutes from '../routes/public/og-card.js';
import brandingPreviewRoutes from '../routes/public/branding-preview.js';
import seoRoutes from '../routes/public/seo.js';
import clientFlowRoutes from '../routes/public/client-flow.js';
import pwaRoutes from '../routes/public/pwa.js';
import vapidRoutes from '../routes/public/vapid.js';
import telemetryRoutes from '../routes/public/telemetry.js';
import accessRequestRoutes from '../routes/public/access-requests.js';
import funnelRoutes from '../routes/public/funnel.js';
import ownerThemeRoutes from '../routes/owner/themes.js';
import publicThemeRoutes from '../routes/public/theme.js';
import ownerNotificationRoutes from '../routes/owner/notifications.js';
import menuImportRoutes from '../routes/owner/menu-import.js';
import menuTranslateRoutes from '../routes/owner/menu-translate.js';
import courierAuthRoutes from '../routes/courier/auth.js';
import courierMeRoutes from '../routes/courier/me.js';
import ownerCourierRoutes from '../routes/owner/couriers.js';
import ownerCourierInvitesRoutes from '../routes/owner/courier-invites.js';
import onboardingRoutes from '../routes/owner/onboarding.js';
import activationRoutes from '../routes/owner/activation.js';
import orderMessageRoutes from '../routes/order-messages.js';
import customerOrderRoutes from '../routes/customer/orders.js';
import ownerSettlementRoutes from '../routes/owner/settlements.js';
import ownerDashboardRoutes from '../routes/owner/dashboard.js';
import courierSettlementRoutes from '../routes/courier/settlements.js';
import courierAssignmentsRoutes from '../routes/courier/assignments.js';
import courierShiftsRoutes from '../routes/courier/shifts.js';
import ownerAlertRoutes from '../routes/owner/alerts.js';
import ownerDwellSettingsRoutes from '../routes/owner/dwell-settings.js';
import customerOtpRoutes from '../routes/customer/otp.js';
import customerTrackRoutes from '../routes/customer/track.js';
import customerPushRoutes from '../routes/customer/push.js';
import ownerPushRoutes from '../routes/owner/push.js';
import ownerOrderMetaRoutes from '../routes/owner/order-meta.js';
import ownerSignalRoutes from '../routes/owner/signals.js';
import ownerGdprRoutes from '../routes/owner/gdpr.js';
import ownerPromotionRoutes from '../routes/owner/promotions.js';
import ownerFallbackRoutes from '../routes/owner/fallback.js';
import ownerRevealContactRoutes from '../routes/owner/reveal-contact.js';
import publicFallbackConfigRoutes from '../routes/public/fallback-config.js';
import publicVoiceConfigRoutes from '../routes/public/voice-config.js';
import ratesRoutes from '../routes/public/rates.js';

export interface CoreRouteDeps {
  pool: Pool;
  messageBus: MessageBus;
  queue: PgBossQueueProvider;
  storage: StorageProvider;
  parsers: Record<string, MenuParserProvider>;
  translation: TranslationProvider;
  /** loadEnv() result — only ACCESS_GATE_PUBLIC_ENABLED is read here. */
  env: Env;
}

/**
 * Registers the core application routes in their original (load-bearing) order.
 * Mirrors server.ts main() lines that ran between authRoutes and
 * publicFallbackConfigRoutes. Must be awaited (it dynamically imports the local
 * auth plugin). The caller registers the order-sensitive tail (telegram webhook,
 * mock-auth, spa-proxy, admin) AFTER this resolves.
 */
export async function registerCoreRoutes(fastify: FastifyInstance, deps: CoreRouteDeps): Promise<void> {
  const { pool, messageBus, queue, storage, parsers, translation, env } = deps;

  // authRoutes define /auth/* paths; mount under /api so they resolve at /api/auth/*.
  fastify.register(authRoutes, { prefix: '/api' });
  // Real email+password login (argon2) + flag-gated dev bypass, both in routes/auth/local.ts.
  // Registered here (prefix /api → /api/auth/local/login).
  const { default: localAuthRoutes } = await import('../routes/auth/local.js');
  fastify.register(localAuthRoutes, { prefix: '/api', db: pool });
  fastify.register(courierRoutes);
  fastify.register(orderRoutes, { prefix: '/api', db: pool, messageBus, queue });
  fastify.register(categoryRoutes);
  fastify.register(productRoutes);
  fastify.register(menuConfirmRoutes);
  // P6 claim phase — public claim accept (verifyAuth) + decline (token-only, no auth).
  fastify.register(publicClaimRoutes, { prefix: '/api', pool });
  fastify.register(modifierGroupRoutes);
  fastify.register(menuAvailabilityRoutes);
  fastify.register(locationRoutes);
  fastify.register(publicMenuRoutes);
  fastify.register(ssrRoutes, { db: pool });
  fastify.register(ogCardRoutes, { db: pool });
  fastify.register(brandingPreviewRoutes);
  fastify.register(seoRoutes, { db: pool });
  fastify.register(clientFlowRoutes, { db: pool });
  fastify.register(pwaRoutes, { db: pool });
  fastify.register(vapidRoutes);
  fastify.register(telemetryRoutes, { db: pool });
  // SENSOR-BUS §1.3: anonymous storefront-funnel ingest. Always mounted; the FUNNEL_INGEST_ENABLED
  // kill-switch is enforced inside (returns a uniform 204 when off).
  fastify.register(funnelRoutes, { db: pool });
  // R3-4 (STOP-1 reachable-surface gate): the access-request capture route is mounted
  // ONLY when the flag is on. While off, POST /api/access-requests 404s via setNotFoundHandler.
  if (env.ACCESS_GATE_PUBLIC_ENABLED === 'true') {
    fastify.register(accessRequestRoutes, { db: pool, queue });
  }
  fastify.register(ownerThemeRoutes, { db: pool, storage });
  fastify.register(publicThemeRoutes, { db: pool });
  fastify.register(ownerNotificationRoutes, { db: pool, queue });
  fastify.register(menuImportRoutes, { prefix: '/api/owner', db: pool, messageBus, parsers, storage, translation });
  fastify.register(menuTranslateRoutes, { prefix: '/api/owner', db: pool, messageBus, translation });
  fastify.register(courierAuthRoutes, { prefix: '/api/courier/auth', db: pool });
  fastify.register(courierMeRoutes, { prefix: '/api/courier', db: pool });
  fastify.register(ownerCourierRoutes, { db: pool });
  fastify.register(ownerCourierInvitesRoutes, { db: pool });
  fastify.register(customerOrderRoutes, { prefix: '/api/customer', db: pool, messageBus });
  fastify.register(ownerSettlementRoutes, { prefix: '/api/owner/locations', db: pool, messageBus });
  fastify.register(ownerDashboardRoutes, { prefix: '/api/owner/locations', db: pool, messageBus, queue });
  fastify.register(ownerAlertRoutes, { prefix: '/api/owner/locations', db: pool, messageBus, queue });
  fastify.register(ownerDwellSettingsRoutes, { prefix: '/api/owner/locations', db: pool, messageBus, queue });
  fastify.register(ownerSignalRoutes, { prefix: '/api/owner/locations', db: pool, messageBus, queue });
  fastify.register(ownerPushRoutes, { prefix: '/api/owner/locations', db: pool, messageBus, queue });
  fastify.register(ownerOrderMetaRoutes, { prefix: '/api/owner/locations', db: pool, messageBus, queue });
  fastify.register(ownerFallbackRoutes, { prefix: '/api/owner/locations', db: pool, messageBus, queue });
  fastify.register(ownerRevealContactRoutes, { prefix: '/api/owner/locations', db: pool, messageBus, queue });
  fastify.register(ownerGdprRoutes, { prefix: '/api/owner/locations', db: pool, messageBus, queue });
  fastify.register(ownerPromotionRoutes, { db: pool });
  fastify.register(onboardingRoutes, { prefix: '/api/owner', db: pool, messageBus, queue });
  fastify.register(activationRoutes, { prefix: '/api/owner', db: pool });
  fastify.register(customerOtpRoutes, { prefix: '/api/customer', db: pool, messageBus });
  fastify.register(customerTrackRoutes, { prefix: '/api/customer', db: pool });
  fastify.register(customerPushRoutes, { prefix: '/api/customer', db: pool, messageBus });
  fastify.register(courierSettlementRoutes, { prefix: '/api/courier', db: pool, messageBus });
  fastify.register(courierAssignmentsRoutes, { prefix: '/api/courier', db: pool, messageBus });
  fastify.register(courierShiftsRoutes, { prefix: '/api/courier', db: pool, messageBus });
  // order-messages.ts declares (fastify: any, opts: any) — register() infers an empty
  // options generic from that; assert the options shape the plugin actually reads.
  fastify.register(orderMessageRoutes as FastifyPluginAsync<{ db: Pool; messageBus: MessageBus }>, { db: pool, messageBus });

  // rates.ts self-casts to bare FastifyPluginAsync (opts generic lost) — assert its real options shape.
  fastify.register(ratesRoutes as FastifyPluginAsync<{ db: Pool }>, { db: pool });
  fastify.register(publicFallbackConfigRoutes, { db: pool });
  fastify.register(publicVoiceConfigRoutes); // voice-control runtime kill-switch (ADR-0015 §9) — no db

}
