import { createContext, useContext, useState, useCallback, useRef, useEffect, type ReactNode } from 'react';
import { createPortal } from 'react-dom';

interface TourStep {
  target: string;
  title: string;
  content: string;
  placement?: 'top' | 'bottom' | 'left' | 'right';
}

interface TourContextValue {
  startTour: (steps: TourStep[]) => void;
  isActive: boolean;
}

const TourContext = createContext<TourContextValue>({ startTour: () => {}, isActive: false });

export function useTour() {
  return useContext(TourContext);
}

function calculatePosition(targetEl: HTMLElement, tooltipEl: HTMLElement, placement: TourStep['placement']) {
  const targetRect = targetEl.getBoundingClientRect();
  const tooltipRect = tooltipEl.getBoundingClientRect();
  const gap = 12;

  let top = 0;
  let left = 0;

  switch (placement) {
    case 'bottom':
      top = targetRect.bottom + gap;
      left = targetRect.left + targetRect.width / 2 - tooltipRect.width / 2;
      break;
    case 'top':
      top = targetRect.top - tooltipRect.height - gap;
      left = targetRect.left + targetRect.width / 2 - tooltipRect.width / 2;
      break;
    case 'left':
      top = targetRect.top + targetRect.height / 2 - tooltipRect.height / 2;
      left = targetRect.left - tooltipRect.width - gap;
      break;
    case 'right':
    default:
      top = targetRect.top + targetRect.height / 2 - tooltipRect.height / 2;
      left = targetRect.right + gap;
      break;
  }

  left = Math.max(16, Math.min(left, window.innerWidth - tooltipRect.width - 16));
  top = Math.max(16, Math.min(top, window.innerHeight - tooltipRect.height - 16));

  return { top, left };
}

export function TourProvider({ children }: { children: ReactNode }) {
  const [steps, setSteps] = useState<TourStep[]>([]);
  const [currentStep, setCurrentStep] = useState(-1);
  const [position, setPosition] = useState({ top: 0, left: 0 });
  const [visible, setVisible] = useState(false);
  const tooltipRef = useRef<HTMLDivElement>(null);
  const spotlightRef = useRef<HTMLDivElement>(null);

  const startTour = useCallback((tourSteps: TourStep[]) => {
    setSteps(tourSteps);
    setCurrentStep(0);
    setVisible(true);
    const firstStep = tourSteps[0];
    if (firstStep) {
      const el = document.querySelector(firstStep.target) as HTMLElement;
      if (el) {
        el.scrollIntoView({ behavior: 'smooth', block: 'center' });
      }
    }
  }, []);

  const updatePosition = useCallback(() => {
    if (currentStep < 0 || currentStep >= steps.length) return;
    const step = steps[currentStep];
    if (!step) return;
    const targetEl = document.querySelector(step.target) as HTMLElement;
    const tooltipEl = tooltipRef.current;
    if (!targetEl || !tooltipEl) return;

    const pos = calculatePosition(targetEl, tooltipEl, step.placement || 'bottom');
    setPosition(pos);

    const targetRect = targetEl.getBoundingClientRect();
    if (spotlightRef.current) {
      spotlightRef.current.style.top = `${targetRect.top - 4}px`;
      spotlightRef.current.style.left = `${targetRect.left - 4}px`;
      spotlightRef.current.style.width = `${targetRect.width + 8}px`;
      spotlightRef.current.style.height = `${targetRect.height + 8}px`;
      spotlightRef.current.style.borderRadius = '8px';
    }
  }, [currentStep, steps]);

  useEffect(() => {
    if (currentStep >= 0) {
      updatePosition();
      window.addEventListener('resize', updatePosition);
      window.addEventListener('scroll', updatePosition);
      return () => {
        window.removeEventListener('resize', updatePosition);
        window.removeEventListener('scroll', updatePosition);
      };
    }
  }, [currentStep, updatePosition]);

  const goNext = () => {
    if (currentStep < steps.length - 1) {
      const next = currentStep + 1;
      setCurrentStep(next);
      const nextStep = steps[next];
      if (nextStep) {
        const el = document.querySelector(nextStep.target) as HTMLElement;
        if (el) el.scrollIntoView({ behavior: 'smooth', block: 'center' });
      }
    } else {
      dismiss();
    }
  };

  const goPrev = () => {
    if (currentStep > 0) {
      const prev = currentStep - 1;
      setCurrentStep(prev);
      const prevStep = steps[prev];
      if (prevStep) {
        const el = document.querySelector(prevStep.target) as HTMLElement;
        if (el) el.scrollIntoView({ behavior: 'smooth', block: 'center' });
      }
    }
  };

  const dismiss = () => {
    setVisible(false);
    setTimeout(() => {
      setCurrentStep(-1);
      setSteps([]);
    }, 300);
  };

  const onKeyDown = useCallback((e: KeyboardEvent) => {
    if (e.key === 'Escape') dismiss();
    if (e.key === 'ArrowRight') goNext();
    if (e.key === 'ArrowLeft') goPrev();
  }, [currentStep, steps]);

  useEffect(() => {
    if (currentStep >= 0) {
      document.addEventListener('keydown', onKeyDown);
      return () => document.removeEventListener('keydown', onKeyDown);
    }
  }, [onKeyDown]);

  if (currentStep < 0 || !visible) {
    return <TourContext.Provider value={{ startTour, isActive: false }}>{children}</TourContext.Provider>;
  }

  const step = steps[currentStep];
  if (!step) return <TourContext.Provider value={{ startTour, isActive: false }}>{children}</TourContext.Provider>;
  const isFirst = currentStep === 0;
  const isLast = currentStep === steps.length - 1;

  return (
    <TourContext.Provider value={{ startTour, isActive: true }}>
      {children}
      {typeof document !== 'undefined' && createPortal(
        <>
          <div
            ref={spotlightRef}
            className="fixed z-[350] pointer-events-none transition-all duration-300 ease-out"
            style={{
              boxShadow: '0 0 0 9999px rgba(0,0,0,0.55)',
            }}
          />
          <div
            ref={tooltipRef}
            role="dialog"
            aria-label={`Tour step ${currentStep + 1} of ${steps.length}: ${step.title}`}
            className="fixed z-[450] w-72 p-4 rounded-xl shadow-elevation-4 fade-in"
            style={{
              top: position.top,
              left: position.left,
              background: 'var(--brand-surface)',
              border: '1px solid var(--brand-border)',
              color: 'var(--brand-text)',
            }}
          >
            <div className="flex items-center justify-between mb-2">
              <span className="text-step-2xs font-medium uppercase tracking-wide" style={{ color: 'var(--brand-primary)' }}>
                Step {currentStep + 1}/{steps.length}
              </span>
              <button
                onClick={dismiss}
                className="w-6 h-6 flex items-center justify-center rounded-full hover:bg-[var(--brand-surface-raised)] transition-colors"
                style={{ color: 'var(--brand-text-muted)' }}
                aria-label="Close tour"
              >
                <i className="ti ti-x" style={{ fontSize: '0.85rem' }} />
              </button>
            </div>
            <h4 className="text-sm font-semibold mb-1" style={{ fontFamily: 'var(--brand-font-heading)' }}>{step.title}</h4>
            <p className="text-xs mb-4" style={{ color: 'var(--brand-text-muted)', lineHeight: 1.5 }}>{step.content}</p>
            <div className="flex items-center gap-2">
              {!isFirst && (
                <button
                  onClick={goPrev}
                  className="px-3 py-1.5 text-xs font-medium rounded-md border transition-colors hover:bg-[var(--brand-surface-raised)]"
                  style={{ borderColor: 'var(--brand-border)', color: 'var(--brand-text-muted)' }}
                >
                  <i className="ti ti-arrow-left" /> Back
                </button>
              )}
              <button
                onClick={goNext}
                className="ml-auto px-4 py-1.5 text-xs font-semibold rounded-full text-white transition-all duration-200 hover:opacity-90 active:scale-[0.97]"
                style={{ background: 'var(--brand-primary)' }}
              >
                {isLast ? 'Finish' : 'Next'} {!isLast && <i className="ti ti-arrow-right" style={{ fontSize: '0.75rem' }} />}
              </button>
            </div>
          </div>
        </>,
        document.body,
      )}
    </TourContext.Provider>
  );
}

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
