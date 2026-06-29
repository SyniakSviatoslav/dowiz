import { WebSocketServer, WebSocket } from 'ws';
import type { FastifyInstance } from 'fastify';
import { verifyAuthToken } from '@deliveryos/platform';
import { AuthToken } from '@deliveryos/shared-types';
import type { MessageBus } from '@deliveryos/platform';
import { courierRoomVerdict, courierReadVerdict } from './lib/courier-room-authz.js';
import { createCourierRelayGuard } from './lib/courier-relay-guard.js';

interface RoomMember {
  ws: WebSocket;
  user: AuthToken;
}

export function setupWebSocket(fastify: FastifyInstance, messageBus: MessageBus) {
  const wss = new WebSocketServer({ server: fastify.server });

  const rooms = new Map<string, Set<RoomMember>>();
  const userBySocket = new Map<WebSocket, AuthToken>();
  // P1-WSDUP: each room registers exactly one messageBus handler. We MUST keep the
  // exact handler reference so we can unsubscribe when the room is torn down —
  // otherwise re-creating a room (member rejoins after the room was GC'd) stacks a
  // second messageBus subscription on the same channel, and every event is then
  // delivered N times (the "Calling N handlers" leak).
  const roomHandlers = new Map<string, (msg: unknown) => void>();

  function deleteRoom(room: string) {
    rooms.delete(room);
    const handler = roomHandlers.get(room);
    if (handler) {
      roomHandlers.delete(room);
      messageBus.unsubscribe(room, handler);
    }
  }

  async function subscribeToRoom(room: string, member: RoomMember) {
    if (!rooms.has(room)) {
      rooms.set(room, new Set());
      const handler = (msg: unknown) => {
        const members = rooms.get(room);
        if (!members) return;
        const payload = JSON.stringify({ room, data: msg });
        // ADR-0013: this fan-out is role-AGNOSTIC (sends to EVERY member), so it is the exact site the
        // C1 reassign leak rides. Route ALL members through the guard — couriers in an order: room are
        // revalidated; non-couriers and non-order rooms relay directly inside the guard.
        const orderId = room.startsWith('order:') ? (room.split(':')[1] ?? null) : null;
        for (const m of members) {
          relayGuard.relay(orderId, m, payload);
        }
      };
      roomHandlers.set(room, handler);
      await messageBus.subscribe(room, handler);
      console.log('[WS] Created room:', room);
    }
    rooms.get(room)!.add(member);
    console.log('[WS] Member joined room:', room, 'total:', rooms.get(room)!.size);
  }

  // ADR-0013 fan-out revalidation guard. Re-derives each courier member's live binding before every
  // `order:<O>` frame so an involuntarily-reassigned courier (the C1 leak) stops receiving within
  // ≤TTL (DB up) / ≤ceiling (DB down). The SHARED chokepoint for all three raw courier-send sites —
  // a raw `member.ws.send` over a courier-joinable room outside this guard is the drift the ESLint
  // rule bans (Breaker NEW-E). Evict = drop from the room + `binding_revoked` (NOT socket-close).
  const relayGuard = createCourierRelayGuard({
    check: (orderId, sub, loc) => courierReadVerdict(fastify.db, sub, loc, orderId),
    evict: (orderId, member, reason) => {
      const room = `order:${orderId}`;
      const members = rooms.get(room);
      if (members) {
        members.delete(member as RoomMember);
        if (members.size === 0) deleteRoom(room);
      }
      if (member.ws.readyState === WebSocket.OPEN) {
        // The guard's sanctioned revocation notice — a single binding_revoked, NOT a frame fan-out
        // (the rule guards the relay sites; eviction is the controlled exit from them).
        // eslint-disable-next-line local/no-raw-courier-ws-send
        member.ws.send(JSON.stringify({ type: 'error', error: reason }));
      }
      console.warn('[WS] courier binding revoked, evicted from', room, (member.user as any).sub);
    },
  });

  // Heartbeat: detect and clean zombie connections every 30s
  const heartbeat = setInterval(() => {
    for (const ws of wss.clients) {
      if ((ws as any).isAlive === false) {
        console.warn('[WS] Zombie connection terminated');
        ws.terminate();
        continue;
      }
      (ws as any).isAlive = false;
      ws.ping();
    }
  }, 30000);

  // Periodic cleanup of empty rooms + relay-guard heap hygiene (drop expired ALLOW entries).
  const roomCleanup = setInterval(() => {
    for (const [room, members] of rooms) {
      if (members.size === 0) {
        deleteRoom(room);
        console.log('[WS] Cleaned up empty room:', room);
      }
    }
    relayGuard.sweep();
  }, 60000);

  // Per-tenant authorization for owner subscriptions. An authenticated owner must
  // only stream rooms for locations they are a member of — otherwise any owner
  // could subscribe to another tenant's `location:*`/`order:*` channel and watch
  // their live order feed. `fastify.db` is the operational Pg pool (decorated on
  // the instance at startup).
  async function ownerCanAccessRoom(userId: string, room: string): Promise<boolean> {
    try {
      if (room.startsWith('location:')) {
        const locId = room.split(':')[1];
        if (!locId) return false;
        const r = await fastify.db.query(
          `SELECT 1 FROM memberships WHERE user_id = $1 AND location_id = $2 AND role = 'owner' AND status = 'active' LIMIT 1`, // P-d (ADR-0004)
          [userId, locId],
        );
        return (r.rowCount ?? 0) > 0;
      }
      if (room.startsWith('order:')) {
        const orderId = room.split(':')[1];
        if (!orderId) return false;
        const r = await fastify.db.query(
          `SELECT 1 FROM orders o
             JOIN memberships m ON m.location_id = o.location_id
            WHERE o.id = $1 AND m.user_id = $2 AND m.role = 'owner' LIMIT 1`,
          [orderId, userId],
        );
        return (r.rowCount ?? 0) > 0;
      }
      return false;
    } catch (err: any) {
      console.error('[WS] owner room authz query failed:', err?.message);
      return false;
    }
  }

  // ADR-0013 (courier-realtime-authz): courier room-authz lives in the shared lib
  // (apps/api/src/lib/courier-room-authz.ts) so the WS gate and the order-messages REST routes
  // can't drift. `courierCanAccessRoom(fastify.db, sub, activeLocationId, room)` denies `location:*`
  // and any order: room the courier holds no live binding for; fail-closed; NOBYPASSRLS-sound.

  wss.on('connection', (ws, req) => {
    (ws as any).isAlive = true; // seed before the first heartbeat tick
    ws.on('pong', () => { (ws as any).isAlive = true; });

    let isAuthenticated = false;
    let user: AuthToken | null = null;
    let authPromise: Promise<void> | null = null;
    const clientIp = req.headers['x-forwarded-for'] || req.socket.remoteAddress;

    const url = new URL(req.url || '', 'http://localhost');
    const urlToken = url.searchParams.get('token');
    if (urlToken) {
      authPromise = verifyAuthToken(urlToken).then(tokenUser => {
        user = tokenUser;
        isAuthenticated = true;
        clearTimeout(authTimeout);
        userBySocket.set(ws, user);
        ws.send(JSON.stringify({ type: 'auth_success', role: user.role }));
        console.log('[WS] Client authenticated via URL token:', user.role, user.sub, 'ip:', clientIp);
      }).catch((err) => {
        console.warn('[WS] URL token auth failed:', err?.message);
      });
    }

    const authTimeout = setTimeout(() => {
      if (!isAuthenticated) {
        console.warn('[WS] Auth timeout for client ip:', clientIp);
        ws.close(1008, 'Authentication timeout');
      }
    }, 5000);

    ws.on('message', async (data) => {
      try {
        const msg = JSON.parse(data.toString());

        if (authPromise) {
          await authPromise;
          authPromise = null;
        }

        if (!isAuthenticated) {
          if (msg.type === 'auth' && msg.token) {
            user = await verifyAuthToken(msg.token);
            isAuthenticated = true;
            clearTimeout(authTimeout);
            userBySocket.set(ws, user);
            ws.send(JSON.stringify({ type: 'auth_success', role: user.role }));
            console.log('[WS] Client authenticated via message:', user.role, user.sub);
          } else {
            ws.close(1008, 'Invalid auth format');
          }
          return;
        }

        if (msg.type === 'subscribe' && msg.room) {
          const { room } = msg;
          const member: RoomMember = { ws, user: user! };

          if (user!.role === 'customer') {
            const orderRoom = `order:${user!.orderId}`;
            if (room !== orderRoom) {
              ws.send(JSON.stringify({ type: 'error', error: 'Forbidden room' }));
              return;
            }
          } else if (user!.role === 'owner') {
            if (!room.startsWith('location:') && !room.startsWith('order:')) {
              ws.send(JSON.stringify({ type: 'error', error: 'Invalid room' }));
              return;
            }
            const ownerId = (user as any).userId ?? user!.sub;
            if (!ownerId || !(await ownerCanAccessRoom(ownerId, room))) {
              ws.send(JSON.stringify({ type: 'error', error: 'Forbidden room' }));
              return;
            }
          } else if (user!.role === 'courier') {
            if (room.startsWith('courier:')) {
              // A courier may only watch their OWN task room (courier:<sub>).
              if (room !== `courier:${user!.sub}`) {
                ws.send(JSON.stringify({ type: 'error', error: 'Forbidden room' }));
                return;
              }
            } else if (room.startsWith('order:')) {
              // ADR-0013: binding-scoped — must hold a live courier_assignments row for THIS order.
              // Tri-state (Breaker NEW-A): a DB blip is UNAVAILABLE → a RETRYABLE soft error that keeps
              // the socket open, NEVER a permanent Forbidden / ws.close — so a pool storm can't fleet-deny.
              const verdict = await courierRoomVerdict(fastify.db, user!.sub, user!.activeLocationId, room);
              if (verdict === 'UNAVAILABLE') {
                ws.send(JSON.stringify({ type: 'error', error: 'Service temporarily unavailable', retryable: true }));
                return;
              }
              if (verdict !== 'ALLOW') {
                ws.send(JSON.stringify({ type: 'error', error: 'Forbidden room' }));
                return;
              }
            } else {
              // `location:*` is the owner dashboard feed; couriers have no legitimate location/other room.
              ws.send(JSON.stringify({ type: 'error', error: 'Forbidden room' }));
              return;
            }
          }

          await subscribeToRoom(room, member);
          ws.send(JSON.stringify({ type: 'subscribed', room }));
          return;
        }

        if (msg.type === 'unsubscribe' && msg.room) {
          const members = rooms.get(msg.room);
          if (members) {
            for (const m of members) {
              if (m.ws === ws) {
                members.delete(m);
                console.log('[WS] Member left room:', msg.room);
                break;
              }
            }
            if (members.size === 0) {
              deleteRoom(msg.room);
              console.log('[WS] Room deleted (empty):', msg.room);
            }
          }
          return;
        }

        if (msg.type === 'client_location' && user!.role === 'customer') {
          const { lat, lng } = msg.payload || {};
          if (typeof lat === 'number' && typeof lng === 'number' &&
              lat >= -90 && lat <= 90 && lng >= -180 && lng <= 180) {
            const orderRoom = `order:${user!.orderId}`;
            const members = rooms.get(orderRoom);
            if (members) {
              const relay = JSON.stringify({
                type: 'client_location',
                payload: { lat, lng, timestamp: Date.now() }
              });
              // ADR-0013: customer GPS goes to courier members ONLY, each binding-revalidated by the guard.
              for (const m of members) {
                if (m.user.role === 'courier') relayGuard.relay(user!.orderId ?? null, m, relay);
              }
            }
          }
          return;
        }

        if (msg.type === 'client_location_stop' && user!.role === 'customer') {
          const orderRoom = `order:${user!.orderId}`;
          const members = rooms.get(orderRoom);
          if (members) {
            const relay = JSON.stringify({ type: 'client_location_stop' });
            // ADR-0013: same guarded fan-out — a revoked courier must not even learn tracking stopped.
            for (const m of members) {
              if (m.user.role === 'courier') relayGuard.relay(user!.orderId ?? null, m, relay);
            }
          }
          return;
        }

        console.warn('[WS] Unknown message type from:', user?.role, msg.type);
      } catch (err) {
        console.error('[WS] Message handling error:', err);
        ws.close(1008, 'Invalid message');
      }
    });

    ws.on('close', (code, reason) => {
      userBySocket.delete(ws);
      if (user) {
        // P1-WSDUP: on disconnect, drop the member and tear the room down (incl.
        // messageBus.unsubscribe) the moment it empties, rather than waiting for the
        // 60s GC. A reconnect then re-creates exactly one subscription.
        for (const [room, members] of rooms) {
          for (const m of members) {
            if (m.ws === ws) {
              members.delete(m);
              break;
            }
          }
          if (members.size === 0) {
            deleteRoom(room);
          }
        }
        console.log('[WS] Client disconnected:', user.role, user.sub, 'code:', code, 'ip:', clientIp);
      }
    });

    ws.on('error', (err) => {
      console.error('[WS] Socket error:', err?.message, 'ip:', clientIp);
    });
  });

  wss.on('close', () => {
    clearInterval(heartbeat);
    clearInterval(roomCleanup);
    console.log('[WS] Server closed');
  });

  fastify.wss = wss;
}
