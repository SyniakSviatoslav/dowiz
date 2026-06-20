import React from 'react';
import { MenuFirstOnboarding } from '../MenuFirstOnboarding.js';

// Brand-new owner entry (O3), now menu-first: the owner uploads their menu, we
// parse it (zero-dependency heuristic), pre-fill name·phone·link and show the
// items found, then create the storefront seeded with that menu — landing in the
// activation tool with the menu gate already satisfied. A "start without a menu"
// path keeps the manual three-field create. The shared flow lives in
// MenuFirstOnboarding; this authed variant skips the Telegram claim (already
// signed in) and creates+seeds directly. The public, pre-auth front door is /start.
export function OnboardingPage() {
  return <MenuFirstOnboarding mode="authed" />;
}
