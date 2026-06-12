// @ts-nocheck
import { Pool } from 'pg';
import type { MessageBus } from '@deliveryos/platform';
import { BUS_CHANNELS, QUEUE_NAMES, orderChannel, dashboardChannel, courierChannel, shiftChannel } from '../lib/registry.js';
import { decryptPII } from '../lib/pii-cipher.js';
import { calculateNaiveETASeconds } from '../lib/geo.js';

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

  private maskName(name: string): string {
    if (!name) return 'A***';
    return name.charAt(0) + '***';
  }

  private maskPhone(phone: string): string {
    if (!phone) return '***';
    return '+*** *** ' + phone.substring(phone.length - 4);
  }

  private async fetchCourierDetailsAndOrder(courierId: string, orderId?: string, locationId?: string) {
    const client = await this.pool.connect();
    try {
      // Find active assignment
      let assignmentQuery = `
        SELECT a.order_id, o.delivery_pin_lat, o.delivery_pin_lng, a.status as assignment_status 
        FROM courier_assignments a
        JOIN orders o ON a.order_id = o.id
        WHERE a.courier_id = $1 AND a.status IN ('accepted', 'picked_up', 'delivered')
        ORDER BY a.created_at DESC LIMIT 1
      `;
      let params: any[] = [courierId];

      if (orderId) {
        assignmentQuery = `
          SELECT a.order_id, o.delivery_pin_lat, o.delivery_pin_lng, a.status as assignment_status 
          FROM courier_assignments a
          JOIN orders o ON a.order_id = o.id
          WHERE a.order_id = $1 AND a.courier_id = $2
        `;
        params = [orderId, courierId];
      }

      const assignmentRes = await client.query(assignmentQuery, params);
      if (assignmentRes.rowCount === 0) return null;

      const { order_id, delivery_pin_lat, delivery_pin_lng, assignment_status } = assignmentRes.rows[0];

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
        destination: delivery_pin_lat !== null && delivery_pin_lng !== null ? { lat: Number(delivery_pin_lat), lng: Number(delivery_pin_lng) } : null,
        assignmentStatus: assignment_status
      };
    } finally {
      client.release();
    }
  }

  private mapAssignmentStatusToDisplay(status: string): string {
    switch (status) {
      case 'accepted': return 'heading_to_pickup';
      case 'picked_up': return 'heading_to_destination';
      case 'delivered': return 'delivered';
      default: return 'at_pickup';
    }
  }

  async handlePositionUpdated(msg: { courierId: string; locationId: string; shiftId: string }) {
    const details = await this.fetchCourierDetailsAndOrder(msg.courierId);
    if (!details) return; // Courier not active on any order

    // Dispatch to owner live map
    await this.messageBus.publish(courierChannel(msg.locationId), {
      type: 'courier.position_updated',
      payload: { courierId: msg.courierId, position: details.position }
    });

    // Dispatch to customer WS
    let etaSeconds = null;
    if (details.assignmentStatus === 'picked_up' && details.position && details.destination) {
      etaSeconds = calculateNaiveETASeconds(
        haversineDistanceKm(details.position, details.destination)
      );
    }

    await this.messageBus.publish(orderChannel(details.orderId), {
      type: 'order.courier_updated',
      payload: {
        orderId: details.orderId,
        courierName: details.courierName,
        phoneMasked: details.phoneMasked,
        position: details.position,
        etaSeconds,
        status: this.mapAssignmentStatusToDisplay(details.assignmentStatus)
      }
    });
  }

  async handleAssignmentEvent(msg: { orderId: string; locationId: string; courierId: string, cashCollected?: boolean }, statusOverride: string) {
    const details = await this.fetchCourierDetailsAndOrder(msg.courierId, msg.orderId, msg.locationId);
    if (!details) return;

    let etaSeconds = null;
    if (statusOverride === 'heading_to_destination' && details.position && details.destination) {
      etaSeconds = calculateNaiveETASeconds(
        haversineDistanceKm(details.position, details.destination)
      );
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
        etaSeconds,
        status: statusOverride
      }
    });
  }
}

// Haversine implementation here for worker context
function haversineDistanceKm(coord1: {lat: number, lng: number}, coord2: {lat: number, lng: number}): number {
  const R = 6371; // Earth's radius in km
  const dLat = (coord2.lat - coord1.lat) * (Math.PI / 180);
  const dLng = (coord2.lng - coord1.lng) * (Math.PI / 180);

  const a =
    Math.sin(dLat / 2) * Math.sin(dLat / 2) +
    Math.cos(coord1.lat * (Math.PI / 180)) * Math.cos(coord2.lat * (Math.PI / 180)) *
    Math.sin(dLng / 2) * Math.sin(dLng / 2);

  const c = 2 * Math.atan2(Math.sqrt(a), Math.sqrt(1 - a));
  return R * c;
}
