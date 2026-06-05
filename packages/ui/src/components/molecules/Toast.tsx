import { createContext, useContext, useState, useCallback, useEffect, type ReactNode } from 'react';
import { createPortal } from 'react-dom';

type ToastVariant = 'success' | 'error' | 'warning' | 'info';

interface ToastItem {
  id: string;
  message: string;
  variant: ToastVariant;
  exiting: boolean;
}

interface ToastContextValue {
  showToast: (message: string, variant?: ToastVariant) => void;
}

const ToastContext = createContext<ToastContextValue | null>(null);

const TOAST_DURATION = 4000;

const variantConfig: Record<ToastVariant, { bg: string }> = {
  success: { bg: 'var(--color-success)' },
  error: { bg: 'var(--color-danger)' },
  warning: { bg: 'var(--color-warning)' },
  info: { bg: 'var(--color-info)' },
};

function ToastCard({ toast, onDismiss }: { toast: ToastItem; onDismiss: (id: string) => void }) {
  const [progress, setProgress] = useState(100);
  const config = variantConfig[toast.variant];

  useEffect(() => {
    const raf = requestAnimationFrame(() => setProgress(0));
    const dismissTimer = setTimeout(() => {
      setProgress(0);
      onDismiss(toast.id);
    }, TOAST_DURATION);
    return () => {
      cancelAnimationFrame(raf);
      clearTimeout(dismissTimer);
    };
  }, [toast.id, onDismiss]);

  const handleDismiss = () => {
    onDismiss(toast.id);
  };

  return (
    <div
      className={`overflow-hidden rounded-lg shadow-elevation-3 max-w-sm transition-all duration-300 ${
        toast.exiting ? 'translate-x-full opacity-0' : 'translate-x-0 opacity-100 animate-toast-in'
      }`}
      role="alert"
    >
      <div className="text-white" style={{ background: config.bg }}>
        <div className="flex items-center gap-2 px-4 py-3">
          <span className="flex-1 text-sm font-medium">{toast.message}</span>
          <button
            onClick={handleDismiss}
            className="text-white/80 hover:text-white transition-colors shrink-0 text-sm leading-none w-5 h-5 flex items-center justify-center rounded-full hover:bg-white/10"
            aria-label="Dismiss"
          >
            <i className="ti ti-x" style={{ fontSize: '0.75rem' }} />
          </button>
        </div>
      </div>
      <div className="h-1 bg-white/15">
        <div
          className={`h-full bg-white/30 transition-all ease-linear`}
          style={{ width: `${progress}%`, transitionDuration: `${TOAST_DURATION}ms` }}
        />
      </div>
    </div>
  );
}

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<ToastItem[]>([]);

  const showToast = useCallback((message: string, variant: ToastVariant = 'info') => {
    const id = `${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
    setToasts((prev) => [...prev, { id, message, variant, exiting: false }]);
  }, []);

  const removeToast = useCallback((id: string) => {
    setToasts((prev) => prev.map((t) => (t.id === id ? { ...t, exiting: true } : t)));
    setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.id !== id));
    }, 300);
  }, []);

  return (
    <ToastContext.Provider value={{ showToast }}>
      {children}
      {typeof document !== 'undefined' &&
        createPortal(
          <div className="fixed top-4 right-4 z-toast flex flex-col gap-2 pointer-events-none max-w-[calc(100vw-2rem)]">
            {toasts.map((t) => (
              <div key={t.id} className="pointer-events-auto">
                <ToastCard toast={t} onDismiss={removeToast} />
              </div>
            ))}
          </div>,
          document.body,
        )}
    </ToastContext.Provider>
  );
}

export function useToast(): ToastContextValue {
  const ctx = useContext(ToastContext);
  if (!ctx) throw new Error('useToast must be used within ToastProvider');
  return ctx;
}
