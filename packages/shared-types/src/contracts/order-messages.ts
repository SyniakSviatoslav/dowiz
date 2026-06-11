import { z } from 'zod';

export const MessageSender = z.enum(['courier', 'customer', 'owner']);
export type MessageSender = z.infer<typeof MessageSender>;

export const MessagePresetKey = z.enum([
  'cu_on_my_way', 'cu_eta', 'cu_arrived', 'cu_at_entrance', 'cu_cant_find',
  'cu_waiting', 'cu_left_at_door', 'cu_prepare_cash', 'cu_please_call_me',
  'cc_coming_out', 'cc_wait', 'cc_leave_at_door', 'cc_im_at', 'cc_meet_outside',
  'ow_accepted_preparing', 'ow_delay', 'ow_substitution', 'ow_high_load',
  'co_cancel_request', 'co_when_ready',
]);
export type MessagePresetKey = z.infer<typeof MessagePresetKey>;

export const OrderStatusForMsg = z.enum([
  'PENDING', 'CONFIRMED', 'PREPARING', 'READY', 'IN_DELIVERY', 'DELIVERED',
  'REJECTED', 'CANCELLED', 'SCHEDULED', 'PICKED_UP',
]);
export type OrderStatusForMsg = z.infer<typeof OrderStatusForMsg>;

export const TERMINAL_STATUSES: ReadonlySet<OrderStatusForMsg> = new Set(['DELIVERED', 'REJECTED', 'CANCELLED']);

export const CuEtaParams = z.object({ minutes: z.union([z.literal(5), z.literal(10), z.literal(15)]) }).strict();
export const CcWaitParams = z.object({ minutes: z.union([z.literal(2), z.literal(5)]) }).strict();
export const CcImAtParams = z.object({ location: z.enum(['entrance', 'gate', 'reception']) }).strict();
export const OwDelayParams = z.object({ minutes: z.union([z.literal(15), z.literal(30)]) }).strict();
export const OwSubstitutionParams = z.object({ action: z.enum(['replace_similar', 'remove_refund', 'cancel']) }).strict();
export const CuPrepareCashParams = z.object({ amount: z.number().int().positive() }).strict();
export const EmptyParams = z.object({}).strict();

export interface PresetDef {
  key: MessagePresetKey;
  sender: MessageSender;
  recipient: MessageSender;
  allowedStates: OrderStatusForMsg[];
  paramsSchema: z.ZodTypeAny;
  requiresCourier?: boolean;
  requiresDropoff?: boolean;
  requiresCash?: boolean;
}

export const PRESET_REGISTRY: Record<string, PresetDef> = {
  cu_on_my_way:     { key: 'cu_on_my_way',     sender: 'courier', recipient: 'customer', allowedStates: ['IN_DELIVERY'],                    paramsSchema: EmptyParams },
  cu_eta:           { key: 'cu_eta',            sender: 'courier', recipient: 'customer', allowedStates: ['IN_DELIVERY'],                    paramsSchema: CuEtaParams },
  cu_arrived:       { key: 'cu_arrived',        sender: 'courier', recipient: 'customer', allowedStates: ['IN_DELIVERY'],                    paramsSchema: EmptyParams },
  cu_at_entrance:   { key: 'cu_at_entrance',    sender: 'courier', recipient: 'customer', allowedStates: ['IN_DELIVERY'],                    paramsSchema: EmptyParams },
  cu_cant_find:     { key: 'cu_cant_find',      sender: 'courier', recipient: 'customer', allowedStates: ['IN_DELIVERY'],                    paramsSchema: EmptyParams },
  cu_waiting:       { key: 'cu_waiting',        sender: 'courier', recipient: 'customer', allowedStates: ['IN_DELIVERY'],                    paramsSchema: EmptyParams },
  cu_left_at_door:  { key: 'cu_left_at_door',   sender: 'courier', recipient: 'customer', allowedStates: ['IN_DELIVERY'], paramsSchema: EmptyParams, requiresDropoff: true },
  cu_prepare_cash:  { key: 'cu_prepare_cash',   sender: 'courier', recipient: 'customer', allowedStates: ['IN_DELIVERY'], paramsSchema: CuPrepareCashParams, requiresCash: true },
  cu_please_call_me:{ key: 'cu_please_call_me', sender: 'courier', recipient: 'customer', allowedStates: ['PENDING','CONFIRMED','PREPARING','READY','IN_DELIVERY'], paramsSchema: EmptyParams },

  cc_coming_out:    { key: 'cc_coming_out',    sender: 'customer', recipient: 'courier', allowedStates: ['IN_DELIVERY'],                    paramsSchema: EmptyParams },
  cc_wait:          { key: 'cc_wait',           sender: 'customer', recipient: 'courier', allowedStates: ['IN_DELIVERY'],                    paramsSchema: CcWaitParams },
  cc_leave_at_door: { key: 'cc_leave_at_door', sender: 'customer', recipient: 'courier', allowedStates: ['IN_DELIVERY'],                    paramsSchema: EmptyParams },
  cc_im_at:         { key: 'cc_im_at',          sender: 'customer', recipient: 'courier', allowedStates: ['IN_DELIVERY'],                    paramsSchema: CcImAtParams },
  cc_meet_outside:  { key: 'cc_meet_outside',  sender: 'customer', recipient: 'courier', allowedStates: ['IN_DELIVERY'],                    paramsSchema: EmptyParams },

  ow_accepted_preparing: { key: 'ow_accepted_preparing', sender: 'owner', recipient: 'customer', allowedStates: ['PENDING', 'PREPARING'],      paramsSchema: EmptyParams },
  ow_delay:              { key: 'ow_delay',              sender: 'owner', recipient: 'customer', allowedStates: ['CONFIRMED', 'PREPARING'],    paramsSchema: OwDelayParams },
  ow_substitution:       { key: 'ow_substitution',       sender: 'owner', recipient: 'customer', allowedStates: ['PENDING', 'PREPARING'],      paramsSchema: OwSubstitutionParams },
  ow_high_load:          { key: 'ow_high_load',          sender: 'owner', recipient: 'customer', allowedStates: ['PENDING', 'CONFIRMED', 'PREPARING'], paramsSchema: EmptyParams },

  co_cancel_request: { key: 'co_cancel_request', sender: 'customer', recipient: 'owner', allowedStates: ['PENDING', 'CONFIRMED'],             paramsSchema: EmptyParams },
  co_when_ready:     { key: 'co_when_ready',     sender: 'customer', recipient: 'owner', allowedStates: ['CONFIRMED', 'PREPARING'],            paramsSchema: EmptyParams },
};

export function getPresetForRole(role: MessageSender): PresetDef[] {
  return Object.values(PRESET_REGISTRY).filter(p => p.sender === role);
}

export function validatePresetAllowed(preset: PresetDef, role: MessageSender, status: OrderStatusForMsg): string | null {
  if (preset.sender !== role) return 'Sender role mismatch';
  if (TERMINAL_STATUSES.has(status)) return 'Order is in terminal status';
  if (!preset.allowedStates.includes(status)) return `Preset ${preset.key} not allowed in status ${status}`;
  return null;
}

export const SendMessageRequest = z.object({
  order_id: z.string().uuid(),
  preset_key: MessagePresetKey,
  params: z.record(z.unknown()).default({}),
}).strict();

export const MessageRecord = z.object({
  id: z.string().uuid(),
  order_id: z.string().uuid(),
  location_id: z.string().uuid(),
  sender: MessageSender,
  preset_key: MessagePresetKey,
  params: z.record(z.unknown()),
  body: z.null(),
  read_at: z.string().datetime().nullable(),
  created_at: z.string().datetime(),
});

export type SendMessageRequest = z.infer<typeof SendMessageRequest>;
export type MessageRecord = z.infer<typeof MessageRecord>;
