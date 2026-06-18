import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { serveSpaShell } from '../../lib/spa-shell.js';

// Cart, checkout and order-status are the React SPA (apps/web). These routes
// serve the SPA shell (with its CSP + per-location frame-ancestors) so a full
// page load / refresh / deep link renders the SPA — keeping the whole /s/:slug/*
// flow on one cart (dos_cart_<slug>), no SSR↔SPA crossing. They are noindex +
// robots-disallowed (seo.ts), so no SSR-for-bots branch is needed here.
export default (async function clientFlowRoutes(fastify: any, opts: any) {
  const { db } = opts as any;

  const serve = (request: any, reply: any) => serveSpaShell(reply, db, request.params.slug);

  fastify.get('/s/:slug/cart', serve);
  fastify.get('/s/:slug/checkout', serve);
  fastify.get('/s/:slug/order/:id', serve);        // SPA route (singular)
  fastify.get('/s/:slug/orders/:orderId', serve);  // legacy deep links / back-compat
}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
