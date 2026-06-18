import './api/devBootstrap.js';
import { StrictMode, lazy, Suspense } from 'react';
import { createRoot } from 'react-dom/client';
import { BrowserRouter, Routes, Route, useLocation } from 'react-router-dom';
import { MotionConfig, AnimatePresence, motion } from 'framer-motion';
import { ThemeProvider, TourProvider, I18nProvider, CurrencyProvider, ErrorBoundary } from '@deliveryos/ui';
import './index.css';

import { Navigate } from 'react-router-dom';
import { LoginPage } from './pages/admin/LoginPage.js';
import { AuthCallback } from './pages/admin/AuthCallback.js';

// Lazy-loaded surfaces
const ClientRoutes = lazy(() => import('./routes/ClientRoutes.js').then(m => ({ default: m.ClientRoutes })));
const AdminRoutes = lazy(() => import('./routes/AdminRoutes.js').then(m => ({ default: m.AdminRoutes })));
const CourierRoutes = lazy(() => import('./routes/CourierRoutes.js').then(m => ({ default: m.CourierRoutes })));
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
        animate={{ opacity: 1, y: 0, scale: 1 }}
        exit={{ opacity: 0, y: -8, scale: 0.99 }}
        transition={{ duration: 0.24, ease: [0.16, 1, 0.3, 1] }}
      >
        <Suspense fallback={<LoadingFallback />}>
          <Routes location={location}>
            <Route path="/" element={<Navigate to="/login" replace />} />
            <Route path="/login" element={<LoginPage />} />
            <Route path="/auth/callback" element={<AuthCallback />} />
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
    <div className="min-h-screen flex flex-col items-center justify-center gap-4">
      <h1 className="text-6xl font-bold">404</h1>
      <p>Faqja nuk u gjet / Page not found</p>
      <a href="/" className="text-[var(--color-info)] hover:underline">Return home</a>
    </div>
  );
}

createRoot(document.getElementById('root')!).render(<App />);
