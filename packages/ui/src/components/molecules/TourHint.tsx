// NOTE: the spotlight/tour-step machinery (TourProvider/useTour/TourContext,
// ~230 lines) was deleted here — it was mounted app-wide (main.tsx) but
// `useTour`/`startTour` had zero callers repo-wide (audit-frontend-2026-07-03.md
// #71): unreachable dead code, not a real feature. HintCard (the only consumer)
// is a plain, self-contained inline card and needs none of that machinery.

export function HintCard({
  title,
  description,
  icon,
  onDismiss,
}: {
  title: string;
  description: string;
  icon?: string;
  onDismiss?: () => void;
}) {
  return (
    <div
      className="flex items-start gap-3 p-3 rounded-lg border slide-in-right"
      style={{
        background: 'var(--brand-surface)',
        borderColor: 'var(--brand-border)',
      }}
    >
      {icon && (
        <div className="w-8 h-8 rounded-full flex items-center justify-center shrink-0" style={{ background: 'var(--brand-primary-light)' }}>
          <i className={icon} style={{ color: 'var(--brand-primary)', fontSize: '0.9rem' }} />
        </div>
      )}
      <div className="flex-1 min-w-0">
        <div className="text-xs font-semibold mb-0.5" style={{ color: 'var(--brand-text)' }}>{title}</div>
        <div className="text-step-2xs" style={{ color: 'var(--brand-text-muted)', lineHeight: 1.4 }}>{description}</div>
      </div>
      {onDismiss && (
        <button
          onClick={onDismiss}
          className="shrink-0 w-5 h-5 flex items-center justify-center rounded-full hover:bg-[var(--brand-surface-raised)] transition-colors"
          aria-label="Dismiss hint"
        >
          <i className="ti ti-x" style={{ fontSize: '0.7rem', color: 'var(--brand-text-muted)' }} />
        </button>
      )}
    </div>
  );
}
