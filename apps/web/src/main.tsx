import './api/devBootstrap.js';
import { StrictMode, lazy, Suspense, Component } from 'react';
import { createRoot } from 'react-dom/client';
import { BrowserRouter, Routes, Route } from 'react-router-dom';
import { ThemeProvider, TourProvider, I18nProvider } from '@deliveryos/ui';
import './index.css';

class ErrorBoundary extends Component<{ children: React.ReactNode }, { hasError: boolean }> {
  constructor(props: { children: React.ReactNode }) {
    super(props);
    this.state = { hasError: false };
  }
  static getDerivedStateFromError() {
    return { hasError: true };
  }
  componentDidCatch() {}
  render() {
    if (this.state.hasError) {
      return (
        <div style={{
          minHeight: '100vh',
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          gap: '1rem',
          background: 'var(--brand-bg)',
          color: 'var(--brand-text)',
        }}>
          <h1 style={{ fontSize: '1.5rem', fontWeight: 600 }}>Something went wrong</h1>
          <p style={{ color: 'var(--brand-text-muted)' }}>An unexpected error occurred.</p>
          <button
            onClick={() => { this.setState({ hasError: false }); window.location.reload(); }}
            style={{
              padding: '0.5rem 1.5rem',
              borderRadius: 'var(--brand-radius)',
              border: 'none',
              background: 'var(--brand-primary)',
              color: '#fff',
              cursor: 'pointer',
            }}
          >
            Reload
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}

import { Navigate } from 'react-router-dom';
import { LoginPage } from './pages/admin/LoginPage.js';

// Lazy-loaded surfaces
const ClientRoutes = lazy(() => import('./routes/ClientRoutes.js').then(m => ({ default: m.ClientRoutes })));
const AdminRoutes = lazy(() => import('./routes/AdminRoutes.js').then(m => ({ default: m.AdminRoutes })));
const CourierRoutes = lazy(() => import('./routes/CourierRoutes.js').then(m => ({ default: m.CourierRoutes })));

function LoadingFallback() {
  return (
    <div className="min-h-screen flex items-center justify-center bg-brand-bg">
      <div className="animate-spin h-8 w-8 border-2 border-brand-primary border-t-transparent rounded-full" />
    </div>
  );
}

function App() {
  return (
    <StrictMode>
      <BrowserRouter>
        <I18nProvider>
        <ThemeProvider>
          <TourProvider>
          <ErrorBoundary>
            <Suspense fallback={<LoadingFallback />}>
              <Routes>
                <Route path="/" element={<Navigate to="/login" replace />} />
                <Route path="/login" element={<LoginPage />} />
                <Route path="/s/:slug/*" element={<ClientRoutes />} />
                <Route path="/admin/*" element={<AdminRoutes />} />
                <Route path="/courier/*" element={<CourierRoutes />} />
                <Route path="*" element={<NotFound />} />
              </Routes>
            </Suspense>
          </ErrorBoundary>
          </TourProvider>
        </ThemeProvider>
        </I18nProvider>
      </BrowserRouter>
    </StrictMode>
  );
}

function NotFound() {
  return (
    <div className="min-h-screen flex flex-col items-center justify-center gap-4">
      <h1 className="text-6xl font-bold">404</h1>
      <p>Faqja nuk u gjet / Page not found</p>
      <a href="/" className="text-blue-500 hover:underline">Return home</a>
    </div>
  );
}

createRoot(document.getElementById('root')!).render(<App />);
