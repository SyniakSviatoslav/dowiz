import { StrictMode, lazy, Suspense } from 'react';
import { createRoot } from 'react-dom/client';
import { BrowserRouter, Routes, Route, useLocation, Navigate } from 'react-router-dom';
import { MotionConfig, AnimatePresence, motion } from 'framer-motion';
import { ThemeProvider, TourProvider, I18nProvider, CurrencyProvider, ErrorBoundary, t, ease, duration } from '@deliveryos/ui';
// Self-hosted Tabler icon webfont (was a jsdelivr CDN <link> in index.html). Vite bundles the CSS
// + hashes/emits the woff2, so icons render offline / on blocked networks — no third-party CDN.
import '@tabler/icons-webfont/dist/tabler-icons.min.css';
import './index.css';
// App-wide PWA install nudge (Android/Chromium banner + iOS "Add to Home Screen"
// hint). Self-hides when already installed or after dismissal — see component.
import { InstallPrompt } from './components/pwa/InstallPrompt.js';

// Lazy-loaded surfaces (UI-PERF roadmap 2.2 — route-based code splitting).
// EVERY page is lazy so the entry chunk carries only the shell (router, theme,
// i18n, ErrorBoundary): a storefront customer landing on /s/:slug must never
// download admin/courier code, and owner/courier surfaces must not pay for the
// storefront. All lazy mounts sit inside the AnimatedRoutes <Suspense> and the
// app-level <ErrorBoundary> (chunk-load failures surface there, not as a blank page).
const LoginPage = lazy(() => import('./pages/admin/LoginPage.js').then(m => ({ default: m.LoginPage })));
const StartPage = lazy(() => import('./pages/MenuFirstOnboarding.js').then(m => ({ default: m.StartPage })));
const AuthCallback = lazy(() => import('./pages/admin/AuthCallback.js').then(m => ({ default: m.AuthCallback })));
const PrivacyPage = lazy(() => import('./pages/PrivacyPage.js').then(m => ({ default: m.PrivacyPage })));
const ClientRoutes = lazy(() => import('./routes/ClientRoutes.js').then(m => ({ default: m.ClientRoutes })));
const AdminRoutes = lazy(() => import('./routes/AdminRoutes.js').then(m => ({ default: m.AdminRoutes })));
const CourierRoutes = lazy(() => import('./routes/CourierRoutes.js').then(m => ({ default: m.CourierRoutes })));
const ClaimPage = lazy(() => import('./pages/ClaimPage.js').then(m => ({ default: m.ClaimPage })));
const CourierInvitePage = lazy(() => import('./pages/courier/CourierInvitePage.js').then(m => ({ default: m.CourierInvitePage })));

function LoadingFallback() {
  return (
    <div className="min-h-screen flex items-center justify-center bg-brand-bg">
      <div className="animate-spin h-8 w-8 border-2 border-brand-primary border-t-transparent rounded-full" />
    </div>
  );
}

function AnimatedRoutes() {
  const location = useLocation();
  return (
    <AnimatePresence mode="wait">
      <motion.div
        key={location.pathname}
        initial={{ opacity: 0, y: 8, scale: 0.99 }}
        animate={{ opacity: 1, y: 0, scale: 1, transition: { duration: duration.base, ease: ease.out } }}
        exit={{ opacity: 0, y: -8, scale: 0.99, transition: { duration: duration.fast, ease: ease.soft } }}
      >
        <Suspense fallback={<LoadingFallback />}>
          <Routes location={location}>
            <Route path="/" element={<Navigate to="/start" replace />} />
            <Route path="/start" element={<StartPage />} />
            <Route path="/login" element={<LoginPage />} />
            <Route path="/privacy" element={<PrivacyPage />} />
            <Route path="/auth/callback" element={<AuthCallback />} />
            <Route path="/claim" element={<ClaimPage />} />
            <Route path="/s/:slug/*" element={<ClientRoutes />} />
            <Route path="/branding-preview/:slug/*" element={<ClientRoutes />} />
            <Route path="/admin/*" element={<AdminRoutes />} />
            <Route path="/courier/*" element={<CourierRoutes />} />
            <Route path="/courier-invite/:inviteId" element={<CourierInvitePage />} />
            <Route path="*" element={<NotFound />} />
          </Routes>
        </Suspense>
      </motion.div>
    </AnimatePresence>
  );
}

function App() {
  return (
    <StrictMode>
      <BrowserRouter>
        <MotionConfig reducedMotion="user">
        <I18nProvider>
        <CurrencyProvider>
        <ThemeProvider>
          <TourProvider>
          <ErrorBoundary>
            <AnimatedRoutes />
          </ErrorBoundary>
          <InstallPrompt />
          </TourProvider>
        </ThemeProvider>
        </CurrencyProvider>
        </I18nProvider>
        </MotionConfig>
      </BrowserRouter>
    </StrictMode>
  );
}

function NotFound() {
  return (
    <div data-skin="paper" className="min-h-screen flex flex-col items-center justify-center gap-4 px-6 text-center" style={{ background: 'var(--brand-bg)', color: 'var(--brand-text)' }}>
      <div className="w-16 h-16 rounded-2xl flex items-center justify-center mb-1" style={{ background: 'var(--brand-surface-raised)', color: 'var(--brand-primary)' }}>
        <i className="ti ti-map-search text-3xl" aria-hidden="true" />
      </div>
      <h1 className="text-5xl font-bold" style={{ fontFamily: 'var(--brand-font-heading)' }}>404</h1>
      <p className="text-step-base max-w-xs" style={{ color: 'var(--brand-text-muted)' }}>{t('error.page_not_found', 'This page took a wrong turn — it doesn’t exist.')}</p>
      <a href="/" className="mt-2 inline-flex items-center gap-2 px-5 py-2.5 rounded-xl text-sm font-semibold transition-all active:scale-95 min-h-11" style={{ background: 'var(--brand-primary)', color: 'var(--color-on-primary)' }}>
        <i className="ti ti-arrow-left text-base" aria-hidden="true" />
        {t('error.return_home', 'Return home')}
      </a>
    </div>
  );
}

// Dev-only fetch mock + demo/mock-courier data. Conditionally imported so the
// mock bootstrap and mockData payload are tree-shaken out of production builds
// (import.meta.env.DEV is statically false in prod → the import is dropped).
if (import.meta.env.DEV) {
  await import('./api/devBootstrap.js');
}

createRoot(document.getElementById('root')!).render(<App />);
