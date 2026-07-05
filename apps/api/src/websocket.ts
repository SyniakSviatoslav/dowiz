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

// ─────────────────────────────────────────────────────────────────────────────
// security-hardening-2026-07 · #4 (WS owner revocation) + #5 (JWT-in-URL deprecation)
// Exported (not just closure-local) so the behavioral red→green tests can exercise the owner
// fan-out revalidation + the auth-log redaction WITHOUT a live socket/DB.
// ─────────────────────────────────────────────────────────────────────────────

export type OwnerRoomVerdict = 'ALLOW' | 'DENY' | 'UNAVAILABLE';

interface OwnerAuthzDb {
  query: (sql: string, params: unknown[]) => Promise<{ rowCount: number | null }>;
}

/**
 * #4 — Tri-state owner room authorization. The SINGLE predicate behind BOTH the WS subscribe gate
 * and the fan-out re-authz, so they cannot drift. Mirrors the ADR-0013 courier tri-state:
 *  - a clean 0-row result is DENY (a real negative — fail closed / evict);
 *  - a query failure is UNAVAILABLE (transient — withhold at the fan-out, NEVER evict a live owner
 *    on a pool blip; a throw must not fleet-deny owners).
 * `status = 'active'` is required in BOTH branches (ADR-0004 revocation). The order: branch
 * previously OMITTED it, so a revoked owner still passed subscribe — that is the #4 subscribe gap.
 */
export async function ownerRoomVerdict(db: OwnerAuthzDb, userId: string, room: string): Promise<OwnerRoomVerdict> {
  if (!userId) return 'DENY';
  let sql: string;
  let params: unknown[];
  if (room.startsWith('location:')) {
    const locId = room.split(':')[1];
    if (!locId) return 'DENY';
    sql = `SELECT 1 FROM memberships WHERE user_id = $1 AND location_id = $2 AND role = 'owner' AND status = 'active' LIMIT 1`;
    params = [userId, locId];
  } else if (room.startsWith('order:')) {
    const orderId = room.split(':')[1];
    if (!orderId) return 'DENY';
    // ADR-0004: JOIN active owner membership on the ORDER's location — no baked-claim trust, and a
    // multi-location owner reading their own order at any of their locations is authorized.
    sql = `SELECT 1 FROM orders o
             JOIN memberships m ON m.location_id = o.location_id
            WHERE o.id = $1 AND m.user_id = $2 AND m.role = 'owner' AND m.status = 'active' LIMIT 1`;
    params = [orderId, userId];
  } else {
    return 'DENY';
  }
  try {
    const r = await db.query(sql, params);
    return (r.rowCount ?? 0) > 0 ? 'ALLOW' : 'DENY';
  } catch (err: any) {
    console.error('[WS] owner room authz query failed:', err?.message);
    return 'UNAVAILABLE';
  }
}

const OWNER_WS_OPEN = 1; // ws.OPEN

export interface OwnerRelayMember {
  ws: { readyState: number; send: (data: string) => void };
  user: { role: string; sub: string; userId?: string };
}

export interface OwnerRelayGuardOptions {
  /** Tri-state live membership read (ownerRoomVerdict) — never throws. */
  check: (room: string, ownerId: string) => Promise<OwnerRoomVerdict>;
  /** Remove the member from the room + notify (`membership_revoked`). */
  evict: (room: string, member: OwnerRelayMember, reason: string) => void;
  now?: () => number;
  ttlMs?: number;      // absolute ALLOW lifetime, no refresh-on-access (~10s)
  maxEntries?: number; // LRU bound on the ALLOW cache
}

/**
 * #4 — fan-out re-authz for OWNERS. Mirrors the ADR-0013 courier relay guard
 * (createCourierRelayGuard), which re-validated ONLY couriers on the broadcast path — so a persistent
 * owner socket kept STREAMING order_update frames after revocation until it disconnected (subscribe
 * gated only NEW subscribes). Before every order:/location: frame we re-derive the owner's LIVE active
 * membership from a short-TTL cache; DENY (revoked/downgraded) evicts the socket from the room and
 * sends a single `membership_revoked` notice.
 *
 * Invariants (mirroring the courier guard):
 *  - Absolute TTL, NO refresh-on-access → a reconnect-flap cannot keep a revoked entry warm.
 *  - Withhold-then-revalidate → a frame reaches an owner ONLY on a cached fresh ALLOW, never
 *    relay-then-check.
 *  - DENY → evict + membership_revoked. UNAVAILABLE (transient DB failure) → withhold only, do NOT
 *    evict (a pool blip must not bounce a live owner; withholding already prevents any leak while the
 *    read is in doubt — owners have no GPS ceiling, so no ceiling machinery is needed).
 *
 * Residual (OR-9, stated honestly): a revoked owner stops receiving frames within ≤TTL, NOT literally
 * zero — identical to the courier guarantee. A true zero-window needs a push-based membership_revoked
 * socket-drop at the instant memberships.status flips; that is OR-9, out of scope here.
 */
export function createOwnerRelayGuard(opts: OwnerRelayGuardOptions) {
  const now = opts.now ?? Date.now;
  const ttlMs = opts.ttlMs ?? 10_000;
  const maxEntries = opts.maxEntries ?? 50_000;
  // Insertion-ordered Map = cheap LRU; key -> absolute expiry. Never re-inserted on READ.
  const allow = new Map<string, number>();
  const inflight = new Set<string>();

  const ownerIdOf = (m: OwnerRelayMember) => m.user.userId ?? m.user.sub;
  const keyOf = (room: string, ownerId: string) => `${room} ${ownerId}`;

  function setAllow(key: string) {
    allow.delete(key); // move-to-end ONLY on write (a fresh ALLOW), never on read
    allow.set(key, now() + ttlMs);
    if (allow.size > maxEntries) {
      const oldest = allow.keys().next().value;
      if (oldest !== undefined) allow.delete(oldest);
    }
  }

  function revalidate(room: string, member: OwnerRelayMember) {
    const ownerId = ownerIdOf(member);
    const key = keyOf(room, ownerId);
    if (inflight.has(key)) return; // dedup concurrent re-reads for the same (room, owner)
    inflight.add(key);
    Promise.resolve(opts.check(room, ownerId))
      .then((verdict) => {
        if (verdict === 'ALLOW') {
          setAllow(key);
        } else if (verdict === 'DENY') {
          allow.delete(key);
          opts.evict(room, member, 'membership_revoked');
        }
        // UNAVAILABLE → withhold only (already done by the caller); do NOT evict a live owner.
      })
      .catch(() => { /* check is contractually non-throwing; swallow to be safe */ })
      .finally(() => { inflight.delete(key); });
  }

  /** Gate ONE frame to ONE owner member. Relays on a fresh ALLOW, else withholds + async re-read. */
  function relay(room: string, member: OwnerRelayMember, payload: string): 'relayed' | 'withheld' | 'skipped' {
    if (member.ws.readyState !== OWNER_WS_OPEN) return 'skipped';
    const ownerId = ownerIdOf(member);
    if (!ownerId) { revalidate(room, member); return 'withheld'; }
    const key = keyOf(room, ownerId);
    const exp = allow.get(key);
    if (exp !== undefined && now() < exp) {
      // eslint-disable-next-line local/no-raw-courier-ws-send -- this guard IS the sanctioned owner fan-out chokepoint (mirrors createCourierRelayGuard); the send only fires on a fresh ALLOW
      member.ws.send(payload);
      return 'relayed';
    }
    if (exp !== undefined) allow.delete(key); // stale
    revalidate(room, member);
    return 'withheld';
  }

  /** Heap hygiene — drop expired ALLOW entries. Eviction is frame-driven (see revalidate). */
  function sweep() {
    const t = now();
    for (const [key, exp] of allow) {
      if (t >= exp) allow.delete(key);
    }
  }

  return {
    relay,
    sweep,
    _stats: () => ({ allow: allow.size, inflight: inflight.size }),
  };
}

/**
 * #5 (JWT-in-URL deprecation window). Every ?token= handshake is recorded so access-logs can drive
 * usage → zero BEFORE the URL path is removed (cached PWA/service-worker clients still connect with
 * it — hard-removing now would WS-lock them out at auth_timeout). role only (to target the client
 * migration); NEVER the token value, NEVER sub.
 */
export function logTokenDeprecation(role: string | undefined, ip: unknown): void {
  console.warn('[WS] DEPRECATED ?token= auth used — migrate to message-auth (removal pending usage→zero); role:', role ?? 'unknown', 'ip:', ip);
}

/**
 * Redacted auth-success log line (the earlier-audit LOW). Never emits the raw token and never emits
 * sub (identity) — only the transport channel + client ip.
 */
export function logAuthSuccess(via: 'url' | 'message', ip: unknown): void {
  console.log('[WS] client authenticated —', via, 'ip:', ip);
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
          // #4: owners are re-validated on the broadcast path (createCourierRelayGuard only
          // re-validated couriers, so a revoked owner kept streaming until disconnect). Couriers +
          // customers keep the existing relayGuard path (courier tri-state / customer scoping intact).
          if (m.user.role === 'owner') {
            ownerRelayGuard.relay(room, m, payload);
          } else {
            relayGuard.relay(orderId, m, payload);
          }
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

  // #4 owner fan-out re-authz guard — the owner counterpart to relayGuard. Re-derives each owner
  // member's LIVE active membership before every order:/location: frame and evicts on revocation
  // (drop from room + `membership_revoked`), so a revoked owner stops receiving frames within ≤TTL
  // (OR-9: not literally zero — same guarantee couriers have).
  const ownerRelayGuard = createOwnerRelayGuard({
    check: (room, ownerId) => ownerRoomVerdict(fastify.db, ownerId, room),
    evict: (room, member, reason) => {
      const members = rooms.get(room);
      if (members) {
        members.delete(member as unknown as RoomMember);
        if (members.size === 0) deleteRoom(room);
      }
      if (member.ws.readyState === WebSocket.OPEN) {
        // eslint-disable-next-line local/no-raw-courier-ws-send -- sanctioned owner revocation notice (mirrors the courier evict); the guard is the fan-out chokepoint, not a raw relay
        member.ws.send(JSON.stringify({ type: 'error', error: reason }));
      }
      console.warn('[WS] owner membership revoked, evicted from', room, (member.user as any).sub);
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
    ownerRelayGuard.sweep();
  }, 60000);

  // Per-tenant authorization for owner subscriptions. An authenticated owner must
  // only stream rooms for locations they are a member of — otherwise any owner
  // could subscribe to another tenant's `location:*`/`order:*` channel and watch
  // their live order feed. `fastify.db` is the operational Pg pool (decorated on
  // the instance at startup).
  async function ownerCanAccessRoom(userId: string, room: string): Promise<boolean> {
    // Subscribe gate: reuse the SINGLE tri-state predicate (ownerRoomVerdict) so the subscribe check
    // and the fan-out re-authz can't drift. #4 fix: the order: branch now carries status = 'active'
    // (ADR-0004) — it previously omitted it, so a revoked owner still passed subscribe. Admission
    // fails closed on both DENY and UNAVAILABLE (only a live ALLOW admits).
    return (await ownerRoomVerdict(fastify.db, userId, room)) === 'ALLOW';
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
        // #5: DUAL-ACCEPT deprecation window — keep accepting ?token= (cached PWA/SW clients) but
        // record every use so usage can be driven to zero before the URL path is removed. Redacted
        // success line (no token, no sub).
        logTokenDeprecation(user.role, clientIp);
        logAuthSuccess('url', clientIp);
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
            // #5: redacted success line (no token, no sub) — the preferred message-auth path.
            logAuthSuccess('message', clientIp);
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
