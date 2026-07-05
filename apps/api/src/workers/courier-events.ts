// @ts-nocheck
import { Pool } from 'pg';
import type { MessageBus } from '@deliveryos/platform';
import { BUS_CHANNELS, QUEUE_NAMES, orderChannel, dashboardChannel, courierChannel, shiftChannel } from '../lib/registry.js';
import { decryptPII } from '../lib/pii-cipher.js';
import { getRoutingService, saveRoute, loadRoute, claimOnce, shouldReroute } from '../lib/routing.js';

export class CourierEventsWorker {
  constructor(
    private pool: Pool,
    private messageBus: MessageBus
  ) {}

  async start() {
    this.messageBus.subscribe(BUS_CHANNELS.COURIER_POSITION_UPDATED, async (msg) => this.handlePositionUpdated(msg));
    this.messageBus.subscribe(BUS_CHANNELS.ORDER_COURIER_ACCEPTED, async (msg) => this.handleAssignmentEvent(msg, 'heading_to_pickup'));
    this.messageBus.subscribe(BUS_CHANNELS.ORDER_PICKED_UP, async (msg) => this.handleAssignmentEvent(msg, 'heading_to_destination'));
    this.messageBus.subscribe(BUS_CHANNELS.ORDER_DELIVERED, async (msg) => this.handleAssignmentEvent(msg, 'delivered'));
  }

  // B3: every DB read/write here is single-tenant — each inbound bus message carries a
  // locationId (the courier-domain tenant). Pin app.current_tenant for the duration of a
  // transaction so the Phase-1 policies on orders / courier_assignments / courier_positions /
  // order_routes are satisfied once dowiz_app loses BYPASSRLS. (couriers has no RLS.)
  // Transaction-local GUC → released on COMMIT/ROLLBACK; the pooled connection returns clean.
  private async withTenant<T>(locationId: string, fn: (client: any) => Promise<T>): Promise<T> {
    const client = await this.pool.connect();
    try {
      await client.query('BEGIN');
      await client.query(`SELECT set_config('app.current_tenant', $1, true)`, [locationId]);
      const out = await fn(client);
      await client.query('COMMIT');
      return out;
    } catch (err) {
      await client.query('ROLLBACK').catch(() => {});
      throw err;
    } finally {
      client.release();
    }
  }

  private maskName(name: string): string {
    if (!name) return 'A***';
    return name.charAt(0) + '***';
  }

  private maskPhone(phone: string): string {
    if (!phone) return '***';
    return '+*** *** ' + phone.substring(phone.length - 4);
  }

  private async fetchCourierDetailsAndOrder(courierId: string, locationId: string, orderId?: string) {
    return this.withTenant(locationId, async (client: any) => {
      // Find active assignment
      let assignmentQuery = `
        SELECT a.order_id, o.delivery_lat, o.delivery_lng, a.status as assignment_status
        FROM courier_assignments a
        JOIN orders o ON a.order_id = o.id
        WHERE a.courier_id = $1 AND a.status IN ('accepted', 'picked_up', 'delivered')
        ORDER BY a.created_at DESC LIMIT 1
      `;
      let params: any[] = [courierId];

      if (orderId) {
        assignmentQuery = `
          SELECT a.order_id, o.delivery_lat, o.delivery_lng, a.status as assignment_status
          FROM courier_assignments a
          JOIN orders o ON a.order_id = o.id
          WHERE a.order_id = $1 AND a.courier_id = $2
        `;
        params = [orderId, courierId];
      }

      const assignmentRes = await client.query(assignmentQuery, params);
      if (assignmentRes.rowCount === 0) return null;

      const { order_id, delivery_lat, delivery_lng, assignment_status } = assignmentRes.rows[0];

      // Find courier PII and latest position
      const courierRes = await client.query(`
        SELECT c.full_name_encrypted, c.phone_encrypted, cp.lat, cp.lng 
        FROM couriers c
        LEFT JOIN courier_positions cp ON cp.courier_id = c.id
        WHERE c.id = $1
        ORDER BY cp.recorded_at DESC LIMIT 1
      `, [courierId]);

      if (courierRes.rowCount === 0) return null;

      const { full_name_encrypted, phone_encrypted, lat, lng } = courierRes.rows[0];

      const name = decryptPII(full_name_encrypted);
      const phone = decryptPII(phone_encrypted);

      return {
        orderId: order_id,
        courierName: this.maskName(name),
        phoneMasked: this.maskPhone(phone),
        position: lat !== null && lng !== null ? { lat: Number(lat), lng: Number(lng) } : null,
        destination: delivery_lat !== null && delivery_lng !== null ? { lat: Number(delivery_lat), lng: Number(delivery_lng) } : null,
        assignmentStatus: assignment_status
      };
    });
  }

  // Latest known position for a courier, independent of any order assignment —
  // used to keep idle (on-shift but unassigned) couriers visible on the owner map.
  private async fetchLatestPosition(courierId: string, locationId: string): Promise<{ lat: number; lng: number } | null> {
    return this.withTenant(locationId, async (client: any) => {
      const res = await client.query(
        `SELECT lat, lng FROM courier_positions WHERE courier_id = $1 ORDER BY recorded_at DESC LIMIT 1`,
        [courierId],
      );
      if (!res.rowCount) return null;
      const { lat, lng } = res.rows[0];
      return lat !== null && lng !== null ? { lat: Number(lat), lng: Number(lng) } : null;
    });
  }

  private mapAssignmentStatusToDisplay(status: string): string {
    switch (status) {
      case 'accepted': return 'heading_to_pickup';
      case 'picked_up': return 'heading_to_destination';
      case 'delivered': return 'delivered';
      default: return 'at_pickup';
    }
  }

  // Compute one road route (per-leg) and push it to the order channel exactly once
  // across the N instances (claimOnce). Authoritative copy stored in Redis for
  // reconnecting clients (served by the status endpoint). Routing is advisory: any
  // failure degrades to haversine inside the provider, so this never throws.
  private async publishRouteOnce(
    orderId: string,
    locationId: string,
    from: { lat: number; lng: number },
    to: { lat: number; lng: number },
    claimKey: string,
    claimTtlS: number,
  ) {
    try {
      if (!(await claimOnce(claimKey, claimTtlS))) return; // another instance owns it
      const route = await getRoutingService().getLegRoute(from, to);
      await saveRoute(orderId, route);

      // Durable copy so the planned route survives the Redis TTL / a flush. Advisory:
      // a failure here must never break the live route push below.
      try {
        // order_routes isolate policy keys on app.current_tenant — pin it for this insert.
        await this.withTenant(locationId, (client: any) =>
          client.query(
            `INSERT INTO order_routes (order_id, location_id, polyline, distance_meters, duration_seconds, updated_at)
           VALUES ($1, $2, $3, $4, $5, now())
           ON CONFLICT (order_id) DO UPDATE
             SET polyline = EXCLUDED.polyline, distance_meters = EXCLUDED.distance_meters,
                 duration_seconds = EXCLUDED.duration_seconds, updated_at = now()`,
            [orderId, locationId, JSON.stringify(route.polyline), route.distance_m, route.duration_s],
          ),
        );
      } catch (err) {
        console.error('[CourierEvents] persist order_routes failed (advisory):', err);
      }
      await this.messageBus.publish(orderChannel(orderId), {
        type: 'order.route',
        payload: {
          orderId,
          polyline: route.polyline,
          durationSeconds: route.duration_s,
          distanceMeters: route.distance_m,
        },
      });
    } catch (err) {
      console.error('[CourierEvents] publishRouteOnce failed (advisory):', err);
    }
  }

  async handlePositionUpdated(msg: { courierId: string; locationId: string; shiftId: string }) {
    // Always surface the courier on the owner live map — even when idle (no active
    // order). The dashboard tracks every on-shift courier, not only those mid-delivery.
    const livePosition = await this.fetchLatestPosition(msg.courierId, msg.locationId);
    if (livePosition) {
      await this.messageBus.publish(courierChannel(msg.locationId), {
        type: 'courier.position_updated',
        payload: { courierId: msg.courierId, position: livePosition },
      });
    }

    // The customer-facing fan-out below only applies while the courier is actively
    // on an order.
    const details = await this.fetchCourierDetailsAndOrder(msg.courierId, msg.locationId);
    if (!details) return;

    // Dispatch to customer WS — D1: do NOT push a single-number ETA to the customer; the honest
    // range comes from the order-status endpoint's `etaRange` (etaService). Live position/route stay.
    if (details.assignmentStatus === 'picked_up' && details.position && details.destination) {
      // Per-leg routing — NOT per-ping. Only (re)compute when there's no stored
      // route yet, or the live position has strayed past the threshold.
      const existing = await loadRoute(details.orderId);
      if (!existing) {
        await this.publishRouteOnce(details.orderId, msg.locationId, details.position, details.destination, `route:init:${details.orderId}`, 300);
      } else if (shouldReroute(existing.polyline, details.position)) {
        await this.publishRouteOnce(details.orderId, msg.locationId, details.position, details.destination, `route:re:${details.orderId}`, 30);
      }
    }

    await this.messageBus.publish(orderChannel(details.orderId), {
      type: 'order.courier_updated',
      payload: {
        orderId: details.orderId,
        courierName: details.courierName,
        phoneMasked: details.phoneMasked,
        position: details.position,
        status: this.mapAssignmentStatusToDisplay(details.assignmentStatus)
      }
    });
  }

  async handleAssignmentEvent(msg: { orderId: string; locationId: string; courierId: string, cashCollected?: boolean }, statusOverride: string) {
    const details = await this.fetchCourierDetailsAndOrder(msg.courierId, msg.locationId, msg.orderId);
    if (!details) return;

    if (statusOverride === 'heading_to_destination' && details.position && details.destination) {
      // The single per-delivery route() call: courier just picked up → heading to
      // the customer. Pushed once to order:{id}; reconnecting clients read it back
      // from the status endpoint. (D1: no single-number ETA pushed; range is on the status endpoint.)
      await this.publishRouteOnce(msg.orderId, msg.locationId, details.position, details.destination, `route:init:${msg.orderId}`, 300);
    }

    // Owner live map assignment status update
    await this.messageBus.publish(courierChannel(msg.locationId), {
      type: 'courier.assignment_status_changed',
      payload: { courierId: msg.courierId, orderId: msg.orderId, status: statusOverride }
    });

    // Customer payload
    await this.messageBus.publish(orderChannel(msg.orderId), {
      type: 'order.courier_updated',
      payload: {
        orderId: msg.orderId,
        courierName: details.courierName,
        phoneMasked: details.phoneMasked,
        position: details.position,
        status: statusOverride
      }
    });
  }
}
