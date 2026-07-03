import { useEffect, useCallback, useState, useRef, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { useIsMobile } from '../../hooks/use-breakpoint.js';

interface ResponsiveDialogProps {
  open: boolean;
  onClose: () => void;
  title?: string;
  children: ReactNode;
  className?: string;
}

const FOCUSABLE_SELECTOR =
  'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])';

// S5 fix (repo-wide: zero focus traps existed despite aria-modal="true" on every
// dialog). This is the ONE modal-shell primitive — every composer (OTPModal,
// ConfirmDialog, and page modals migrated onto ResponsiveDialog) gets a real trap,
// initial focus and focus restoration for free instead of re-solving it per-site.
export function ResponsiveDialog({ open, onClose, title, children, className = '' }: ResponsiveDialogProps) {
  const isMobile = useIsMobile();
  // Ease-out enter without global keyframes; reduced-motion collapses the
  // --motion-* tokens to 0ms so this becomes instant.
  const [entered, setEntered] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const previouslyFocusedRef = useRef<HTMLElement | null>(null);

  const getFocusable = useCallback((): HTMLElement[] => {
    const root = containerRef.current;
    if (!root) return [];
    return Array.from(root.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)).filter(
      (el) => !el.hasAttribute('disabled') && el.getAttribute('aria-hidden') !== 'true',
    );
  }, []);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
        return;
      }
      if (e.key !== 'Tab') return;
      const focusable = getFocusable();
      if (focusable.length === 0) {
        // Nothing to tab to inside the dialog — keep focus from escaping to the page.
        e.preventDefault();
        containerRef.current?.focus();
        return;
      }
      const first = focusable[0]!;
      const last = focusable[focusable.length - 1]!;
      const active = document.activeElement as HTMLElement | null;
      const activeInside = !!active && containerRef.current?.contains(active);
      if (e.shiftKey) {
        if (!activeInside || active === first) {
          e.preventDefault();
          last.focus();
        }
      } else if (!activeInside || active === last) {
        e.preventDefault();
        first.focus();
      }
    },
    [onClose, getFocusable],
  );

  useEffect(() => {
    if (open) {
      previouslyFocusedRef.current = document.activeElement as HTMLElement | null;
      document.addEventListener('keydown', handleKeyDown);
      document.body.style.overflow = 'hidden';
      const raf = requestAnimationFrame(() => {
        setEntered(true);
        // Initial focus inside the dialog. A composer that needs a specific field
        // focused (e.g. OTPModal's code input) simply focuses it itself afterward —
        // this is only the default so a dialog with no such logic isn't left with
        // focus stranded on the page underneath.
        const focusable = getFocusable();
        (focusable[0] ?? containerRef.current)?.focus();
      });
      return () => {
        cancelAnimationFrame(raf);
        document.removeEventListener('keydown', handleKeyDown);
        document.body.style.overflow = '';
        setEntered(false);
        // Restore focus to whatever triggered the dialog — without this, closing
        // (Escape, backdrop, or an in-dialog action) drops focus to <body>.
        previouslyFocusedRef.current?.focus?.();
      };
    }
    return () => {
      document.removeEventListener('keydown', handleKeyDown);
      document.body.style.overflow = '';
    };
  }, [open, handleKeyDown, getFocusable]);

  if (!open) return null;

  const closeButton = (
    <button
      onClick={onClose}
      className="w-8 h-8 flex items-center justify-center rounded-full hover:bg-[var(--brand-surface-raised)] text-[var(--brand-text-muted)] transition-[background-color,transform] duration-[var(--motion-fast)] ease-[var(--ease-soft)] active:scale-95 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2"
      aria-label="Close"
    >
      <i className="ti ti-x" />
    </button>
  );

  const backdrop = (
    <div
      className="absolute inset-0 bg-black/50 backdrop-blur-sm transition-opacity ease-[var(--ease-out)]"
      style={{ opacity: entered ? 1 : 0, transitionDuration: 'var(--motion-base)' }}
      role="button"
      tabIndex={0}
      aria-label="Close"
      onClick={onClose}
      onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onClose(); } }}
    />
  );

  if (isMobile) {
    return createPortal(
      <div className="fixed inset-0 z-modal-backdrop flex items-end justify-center">
        {backdrop}
        <div
          ref={containerRef}
          tabIndex={-1}
          className={`relative z-modal w-full max-h-[85vh] bg-[var(--brand-bg)] flex flex-col transition-transform ease-[var(--ease-out)] outline-none ${className}`}
          style={{
            borderTopLeftRadius: 'var(--brand-radius)',
            borderTopRightRadius: 'var(--brand-radius)',
            boxShadow: 'var(--elev-4)',
            transform: entered ? 'translateY(0)' : 'translateY(100%)',
            transitionDuration: 'var(--motion-base)',
          }}
          role="dialog"
          aria-modal="true"
          aria-label={title}
        >
          <div className="flex items-center justify-center pt-3 pb-1 shrink-0">
            <div className="w-10 h-1.5 rounded-full bg-[var(--brand-border)]" aria-hidden="true" />
          </div>
          {title && (
            <div className="flex items-center justify-between px-5 pt-2 pb-3 shrink-0">
              <h2 className="text-lg font-heading font-semibold text-[var(--brand-text)]">{title}</h2>
              {closeButton}
            </div>
          )}
          <div
            className="overflow-y-auto px-5 pb-6 sheet-content"
            style={{ paddingBottom: 'max(1.5rem, env(safe-area-inset-bottom))' }}
          >
            {children}
          </div>
        </div>
      </div>,
      document.body,
    );
  }

  return createPortal(
    <div className="fixed inset-0 z-modal-backdrop flex items-center justify-center p-4">
      {backdrop}
      <div
        ref={containerRef}
        tabIndex={-1}
        role="dialog"
        aria-modal="true"
        aria-label={title}
        className={`relative z-modal bg-[var(--brand-bg)] max-w-md w-full max-h-[85vh] overflow-y-auto transition-[opacity,transform] ease-[var(--ease-out)] outline-none ${className}`}
        style={{
          borderRadius: 'var(--brand-radius)',
          boxShadow: 'var(--elev-4)',
          opacity: entered ? 1 : 0,
          transform: entered ? 'translateY(0) scale(1)' : 'translateY(8px) scale(0.98)',
          transitionDuration: 'var(--motion-base)',
        }}
      >
        {title && (
          <div className="flex items-center justify-between px-5 pt-5 pb-3">
            <h2 className="text-lg font-heading font-semibold text-[var(--brand-text)]">{title}</h2>
            {closeButton}
          </div>
        )}
        <div className="px-5 pb-5">{children}</div>
      </div>
    </div>,
    document.body,
  );
}
