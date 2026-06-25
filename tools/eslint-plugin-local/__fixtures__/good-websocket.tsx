// Test fixture for no-direct-websocket rule (valid code — must NOT be flagged).
// CORRECT: a frontend component subscribes via the shared WS client; it never
// constructs a raw WebSocket itself.
import { useWebSocket } from '../../../apps/web/src/lib/useWebSocket';

export function LiveBadge({ orderId }: { orderId: string }) {
  const { lastMessage } = useWebSocket(`/orders/${orderId}`);
  return null;
}
