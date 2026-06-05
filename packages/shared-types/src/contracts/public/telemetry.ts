import { z } from 'zod';

export const TELEMETRY_ACTIONS = [
  'cart.added','cart.removed','cart.drift_resolved','cart.corrupted',
  'checkout.opened','checkout.submitted','checkout.failed',
  'order.status_viewed','pwa.installed','pwa.install_prompted',
] as const;

export const TelemetryBody = z.object({
  action: z.enum(TELEMETRY_ACTIONS),
  locationId: z.string().uuid().optional(),
  orderId: z.string().uuid().optional(),
  errorCode: z.string().optional(),
  delta: z.number().optional(),
}).passthrough();
export type TelemetryBody = z.infer<typeof TelemetryBody>;

export const TelemetryResponse = z.object({
  accepted: z.literal(true),
}).strict();
export type TelemetryResponse = z.infer<typeof TelemetryResponse>;
