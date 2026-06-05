import { z } from 'zod';

export const CourierLoginBody = z.object({
  email: z.string().email(),
  password: z.string().min(1),
  location_id: z.string().uuid(),
}).strict();
export type CourierLoginBody = z.infer<typeof CourierLoginBody>;

export const CourierLoginResponse = z.object({
  jwt: z.string(),
  refreshToken: z.string(),
}).strict();
export type CourierLoginResponse = z.infer<typeof CourierLoginResponse>;

export const CourierRefreshBody = z.object({
  refresh_token: z.string().min(1),
}).strict();
export type CourierRefreshBody = z.infer<typeof CourierRefreshBody>;

export const CourierRefreshResponse = z.object({
  jwt: z.string(),
  refreshToken: z.string(),
}).strict();
export type CourierRefreshResponse = z.infer<typeof CourierRefreshResponse>;

export const CourierLogoutBody = z.object({
  refresh_token: z.string().min(1),
}).strict();
export type CourierLogoutBody = z.infer<typeof CourierLogoutBody>;

export const CourierInviteRedeemBody = z.object({
  email: z.string().email(),
  code: z.string().min(1),
  password: z.string().min(12),
  full_name: z.string().min(1),
  phone: z.string().optional(),
}).strict();
export type CourierInviteRedeemBody = z.infer<typeof CourierInviteRedeemBody>;

export const CourierInviteRedeemResponse = z.object({
  jwt: z.string(),
  refreshToken: z.string(),
  courier: z.object({
    id: z.string().uuid(),
    masked_email: z.string(),
    full_name: z.string(),
    locations: z.array(z.object({
      id: z.string().uuid(),
      role: z.string(),
    })),
  }),
}).strict();
export type CourierInviteRedeemResponse = z.infer<typeof CourierInviteRedeemResponse>;
