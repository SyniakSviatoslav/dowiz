import { useState, useCallback, useEffect, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { useI18n } from '../../lib/I18nProvider.js';

interface ConfirmDialogProps {
  open: boolean;
  title: string;
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  onConfirm: () => void | Promise<void>;
  onClose: () => void;
  variant?: 'danger' | 'default';
}

export function ConfirmDialog({
  open,
  title,
  message,
  confirmLabel,
  cancelLabel,
  onConfirm,
  onClose,
  variant = 'default',
}: ConfirmDialogProps) {
  const { t } = useI18n();
  const resolvedConfirm = confirmLabel ?? t('common.confirm');
  const resolvedCancel = cancelLabel ?? t('common.cancel');
  const [visible, setVisible] = useState(false);
  const [loading, setLoading] = useState(false);

  const handleConfirm = async () => {
    if (loading) return;
    setLoading(true);
    try {
      await onConfirm();
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (open) {
      requestAnimationFrame(() => requestAnimationFrame(() => setVisible(true)));
      document.body.style.overflow = 'hidden';
    } else {
      setVisible(false);
    }
    return () => {
      document.body.style.overflow = '';
    };
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  }, [open, onClose]);

  if (!open) return null;

  return createPortal(
    <div className="fixed inset-0 z-modal-backdrop flex items-center justify-center p-4">
      <div
        className={`absolute inset-0 bg-black/60 backdrop-blur-sm transition-opacity duration-[var(--motion-base)] ease-[var(--ease-soft)] ${
          visible ? 'opacity-100' : 'opacity-0'
        }`}
        role="button"
        tabIndex={0}
        aria-label={t('common.close', 'Close')}
        onClick={onClose}
        onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onClose(); } }}
      />
      <div
        className={`relative z-modal bg-[var(--brand-surface)] rounded-[var(--brand-radius)] max-w-sm w-full p-6 transform-gpu transition-[transform,opacity] duration-[var(--motion-base)] ease-[var(--ease-out)] motion-reduce:transition-opacity ${
          visible ? 'scale-100 opacity-100' : 'scale-95 opacity-0 motion-reduce:scale-100'
        }`}
        role="alertdialog"
        aria-modal="true"
        aria-label={title}
        style={{ boxShadow: 'var(--elev-4)' }}
      >
        <button
          onClick={onClose}
          className="absolute top-3 right-3 w-8 h-8 flex items-center justify-center rounded-full hover:bg-[var(--brand-surface-raised)] text-[var(--brand-text-muted)] hover:text-[var(--brand-text)] transition-colors duration-[var(--motion-fast)] ease-[var(--ease-soft)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--brand-surface)]"
          aria-label={t('common.close', 'Close')}
        >
          &#x2715;
        </button>
        <h3 className="text-lg font-heading font-semibold text-[var(--brand-text)] mb-2 pr-8">{title}</h3>
        <p className="text-sm text-[var(--brand-text)] mb-6 leading-relaxed">{message}</p>
        <div className="flex gap-3 justify-end">
          <button
            onClick={onClose}
            className="px-5 py-2 text-sm font-semibold text-[var(--brand-text)] rounded-full border border-[var(--brand-border)] hover:bg-[var(--brand-surface-raised)] transition-colors duration-[var(--motion-fast)] ease-[var(--ease-soft)] active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--brand-surface)]"
          >
            {resolvedCancel}
          </button>
          <button
            onClick={handleConfirm}
            disabled={loading}
            className={`px-5 py-2 text-sm font-semibold text-white rounded-full transition-[background-color,transform,opacity] duration-[var(--motion-fast)] ease-[var(--ease-soft)] active:scale-[0.98] disabled:opacity-50 disabled:cursor-not-allowed focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--brand-surface)] ${
              variant === 'danger'
                ? 'hover:opacity-90 focus-visible:ring-[var(--color-danger)]'
                : 'bg-[var(--brand-primary)] hover:bg-[var(--brand-primary-hover)] focus-visible:ring-[var(--brand-primary)]'
            }`}
            style={variant === 'danger' ? { background: 'var(--color-danger)' } : undefined}
          >
            {loading ? '...' : resolvedConfirm}
          </button>
        </div>
      </div>
    </div>,
    document.body,
  );
}

interface ConfirmOptions {
  title: string;
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  variant?: 'danger' | 'default';
}

export function useConfirm(): {
  confirm: (opts: ConfirmOptions) => Promise<boolean>;
  dialog: ReactNode;
} {
  const [state, setState] = useState<{
    key: number;
    open: boolean;
    options: ConfirmOptions;
    resolve: ((value: boolean) => void) | null;
  }>({
    key: 0,
    open: false,
    options: { title: '', message: '' },
    resolve: null,
  });

  const confirm = useCallback((opts: ConfirmOptions): Promise<boolean> => {
    return new Promise((resolve) => {
      setState({ key: Date.now(), open: true, options: opts, resolve });
    });
  }, []);

  const handleClose = useCallback(() => {
    state.resolve?.(false);
    setState((s) => ({ ...s, open: false, resolve: null }));
  }, [state.resolve]);

  const handleConfirm = useCallback(() => {
    state.resolve?.(true);
    setState((s) => ({ ...s, open: false, resolve: null }));
  }, [state.resolve]);

  const dialog = (
    <ConfirmDialog
      key={state.key}
      open={state.open}
      title={state.options.title}
      message={state.options.message}
      confirmLabel={state.options.confirmLabel}
      cancelLabel={state.options.cancelLabel}
      variant={state.options.variant}
      onConfirm={handleConfirm}
      onClose={handleClose}
    />
  );

  return { confirm, dialog };
}
