// Test fixture for no-direct-websocket rule.
// ANTI-PATTERN: a frontend component constructing a raw WebSocket bypasses the
// shared client that owns reconnect-jitter + ordered-frame handling.
// Recurrent class: out-of-order WS frames + reconnect bugs.

export function LiveBadge({ url }: { url: string }) {
  // ANTI-PATTERN: should be flagged — use the shared WS client instead
  const ws = new WebSocket(url);
  ws.onmessage = () => {};
  return null;
}
