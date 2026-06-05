import { z } from 'zod';

export const DwellThresholds = z.object({
  pending_s: z.number().int().min(10).max(3600),
  confirmed_s: z.number().int().min(10).max(3600),
  preparing_s: z.number().int().min(10).max(7200),
  en_route_s: z.number().int().min(10).max(7200),
}).strict();
export type DwellThresholds = z.infer<typeof DwellThresholds>;

export const DwellSettingsResponse = z.object({
  dwellThresholds: DwellThresholds,
}).strict();
export type DwellSettingsResponse = z.infer<typeof DwellSettingsResponse>;

export const UpdateDwellSettingsBody = z.object({
  dwellThresholds: DwellThresholds,
}).strict();
export type UpdateDwellSettingsBody = z.infer<typeof UpdateDwellSettingsBody>;

export const RetentionSettings = z.object({
  retentionDays: z.number().int(),
}).strict();
export type RetentionSettings = z.infer<typeof RetentionSettings>;

export const UpdateRetentionBody = z.object({
  retentionDays: z.number().int().min(30).max(2555),
}).strict();
export type UpdateRetentionBody = z.infer<typeof UpdateRetentionBody>;

export const FallbackSettingsResponse = z.object({
  phone: z.string().nullable(),
  showPhoneOnError: z.boolean(),
  showPhoneOnOffline: z.boolean(),
  wsRetryMax: z.number().int().optional(),
  wsRetryBaseMs: z.number().int().optional(),
}).strict();
export type FallbackSettingsResponse = z.infer<typeof FallbackSettingsResponse>;

export const UpdateFallbackSettingsBody = z.object({
  phone: z.string().max(50).optional(),
  showPhoneOnError: z.boolean(),
  showPhoneOnOffline: z.boolean(),
  wsRetryMax: z.number().int().min(1).max(30).optional(),
  wsRetryBaseMs: z.number().int().min(500).max(10000).optional(),
}).strict();
export type UpdateFallbackSettingsBody = z.infer<typeof UpdateFallbackSettingsBody>;
