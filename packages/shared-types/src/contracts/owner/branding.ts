import { z } from 'zod';

export const ALLOWED_FONTS = [
  "'DM Sans', sans-serif",
  "'Inter', sans-serif",
  "'DM Serif Display', serif",
  "'Cormorant Garamond', serif",
  "'Playfair Display', serif",
] as const;

export const UpdateThemeBody = z.object({
  primary_color: z.string().regex(/^#[0-9a-fA-F]{6}$/).nullable().optional(),
  secondary_color: z.string().regex(/^#[0-9a-fA-F]{6}$/).nullable().optional(),
  font_family: z.enum(ALLOWED_FONTS).nullable().optional(),
  bg_color: z.string().regex(/^#[0-9a-fA-F]{6}$/).nullable().optional(),
  text_color: z.string().regex(/^#[0-9a-fA-F]{6}$/).nullable().optional(),
  frame_ancestors: z.array(z.string()).optional(),
}).strict();
export type UpdateThemeBody = z.infer<typeof UpdateThemeBody>;

export const ThemeResponse = z.object({
  id: z.string().uuid(),
  locationId: z.string().uuid(),
  primaryColor: z.string().nullable(),
  secondaryColor: z.string().nullable(),
  fontFamily: z.string().nullable(),
  bgColor: z.string().nullable(),
  textColor: z.string().nullable(),
  logoUrl: z.string().nullable(),
  frameAncestors: z.array(z.string()).nullable(),
  googleRating: z.number().nullable().optional(),
  googleReviewCount: z.number().int().nullable().optional(),
  googleMapsUrl: z.string().nullable().optional(),
}).strict();
export type ThemeResponse = z.infer<typeof ThemeResponse>;

export const ThemeFullResponse = z.object({
  theme: ThemeResponse,
  cssHash: z.string().nullable(),
  version: z.number().int(),
}).strict();
export type ThemeFullResponse = z.infer<typeof ThemeFullResponse>;

export const UpdateThemeResponse = z.object({
  cssHash: z.string(),
  version: z.number().int(),
  warnings: z.array(z.string()).optional(),
}).strict();
export type UpdateThemeResponse = z.infer<typeof UpdateThemeResponse>;

export const LogoUploadResponse = z.object({
  logo_url: z.string(),
}).strict();
export type LogoUploadResponse = z.infer<typeof LogoUploadResponse>;
