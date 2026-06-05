// @ts-nocheck
import { WebSocketServer, WebSocket } from 'ws';
import type { FastifyInstance } from 'fastify';
import { verifyAuthToken } from '@deliveryos/platform';
import { AuthToken } from '@deliveryos/shared-types';
import type { MessageBus } from '@deliveryos/platform';

interface RoomMember {
  ws: WebSocket;
  user: AuthToken;
}

export function setupWebSocket(fastify: FastifyInstance, messageBus: MessageBus) {
  const wss = new WebSocketServer({ server: fastify.server });

  const rooms = new Map<string, Set<RoomMember>>();
  const userBySocket = new Map<WebSocket, AuthToken>();

  async function subscribeToRoom(room: string, member: RoomMember) {
    if (!rooms.has(room)) {
      rooms.set(room, new Set());

      await messageBus.subscribe(room, (msg: unknown) => {
        const members = rooms.get(room);
        if (!members) return;
        const payload = JSON.stringify({ room, data: msg });
        for (const m of members) {
          if (m.ws.readyState === WebSocket.OPEN) {
            m.ws.send(payload);
          }
        }
      });
    }
    rooms.get(room)!.add(member);
  }

  wss.on('connection', (ws) => {
    let isAuthenticated = false;
    let user: AuthToken | null = null;

    const authTimeout = setTimeout(() => {
      if (!isAuthenticated) {
        ws.close(1008, 'Authentication timeout');
      }
    }, 5000);

    ws.on('message', async (data) => {
      try {
        const msg = JSON.parse(data.toString());

        if (!isAuthenticated) {
          if (msg.type === 'auth' && msg.token) {
            user = await verifyAuthToken(msg.token);
            isAuthenticated = true;
            clearTimeout(authTimeout);
            userBySocket.set(ws, user);
            ws.send(JSON.stringify({ type: 'auth_success', role: user.role }));
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
          } else if (user!.role === 'owner' || user!.role === 'courier') {
            if (!room.startsWith('location:') && !room.startsWith('order:')) {
              ws.send(JSON.stringify({ type: 'error', error: 'Invalid room' }));
              return;
            }
          }

          await subscribeToRoom(room, member);
          ws.send(JSON.stringify({ type: 'subscribed', room }));
        }

        if (msg.type === 'unsubscribe' && msg.room) {
          const member: RoomMember = { ws, user: user! };
          const members = rooms.get(msg.room);
          if (members) {
            members.delete(member);
          }
        }

      } catch (err) {
        fastify.log.error(err);
        ws.close(1008, 'Invalid message');
      }
    });

    ws.on('close', () => {
      userBySocket.delete(ws);
      if (user) {
        for (const [, members] of rooms) {
          for (const m of members) {
            if (m.ws === ws) {
              members.delete(m);
            }
          }
        }
      }
    });
  });

  fastify.wss = wss;
}
