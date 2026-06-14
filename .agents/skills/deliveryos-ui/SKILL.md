# Skill: deliveryos-ui

# DeliveryOS UI Polish Skill

## Goal
Provide the agent with the design system truth, gate requirements, freeze boundaries,
and polish rules for any UI/UX work on DeliveryOS. Loaded automatically on any UI task.

## Design System Truth
- **DESIGN.md**: `docs/design/DESIGN.md` — authoritative brand contract, 9 sections
- **tokens.css**: `packages/ui/src/theme/tokens.css` — single source of all CSS variables
- **Figma-Make-Prompt**: `design.md` — brand identity and visual references

## Acceptance Gates
Both gates must pass before any UI change is considered complete:
1. **Frontend Gate**: `docs/audit/execution-plan.md` — unified design system, grep-clean, all states
2. **Accessibility Gate**: `docs/audit/accessibility-gate.md` — WCAG 2.2 AA, all theme contrast, keyboard/SR

## Two-Speed Rule
- **Inline fixes** (auto): rhythm/spacing, typography hierarchy, state completeness, micro-interactions, AI-slop removal, focus-ring/aria/contrast
- **Flag-only** (manual review): token system changes, third-party design systems, server contracts, pricing/status logic, white-label contrast

## Red Lines (DO NOT)
- Replace existing token system with any foreign design system
- Change server contracts / Zod schemas / business logic
- Use emojis as primary UI elements (use Tabler icons: `ti ti-*`)
- Write hex colors outside CSS variables
- Use `position:fixed` in embed mode
- Store PII in any client storage or event payloads
- Skip component states (default/hover/active/disabled)
- Skip dark mode on any screen
- Auto-fix token values or add new design systems

## Freeze Boundaries
When doing inline fixes, agent is restricted to:
- `packages/ui/src/` — UI components and theme
- `apps/web/src/` — React pages and routes

No edits outside these directories without explicit permission.

## Component Architecture
- `packages/ui/src/components/atoms/` — Button, Icon, Input, Skeleton, StatusBadge
- `packages/ui/src/components/molecules/` — Toast, Tooltip, TourHint, Map*, Drawer, Modal, BottomSheet, ConfirmDialog
- `packages/ui/src/components/layout/` — AdminShell, ClientShell, CourierShell, EmbedShell, ThemeProvider
- `packages/ui/src/components/admin/` — AdminUI (OrderCard, OrderDetail, SwipeToComplete)
- `packages/ui/src/components/client/` — ClientUI (ProductCard, CartFAB, CategoryNav, OTPModal)
- `packages/ui/src/components/courier/` — CourierUI (TaskCard, SwipeToComplete, DeliveryInfo)

## Tour & Hints System
- `TourProvider` wraps the app in `apps/web/src/main.tsx`
- `useTour()` hook returns `{ startTour, isActive }`
- `HintCard` component for contextual in-page tips
- Tour steps use `target` (CSS selector), `title`, `content`, `placement`

Base directory for this skill: file:///C:/Users/Dell5/Documents/dowiz/.agents/skills/deliveryos-ui
