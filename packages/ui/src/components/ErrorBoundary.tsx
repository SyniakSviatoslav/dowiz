import { Component, type ComponentType, type ReactNode, type ErrorInfo } from 'react';
import { useI18n } from '../lib/I18nProvider.js';

interface ErrorBoundaryProps {
  children: ReactNode;
  fallback?: ReactNode | ((error: Error, reset: () => void) => ReactNode);
  onError?: (error: Error, info: ErrorInfo) => void;
  onReset?: () => void;
  resetKeys?: unknown[];
}

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
}

function DefaultFallback({ error, reset }: { error: Error | null; reset: () => void }) {
  const { t } = useI18n();
  return (
    <div className="flex flex-col items-center justify-center min-h-screen bg-brand-bg text-brand-text p-6 gap-4">
      <div className="w-16 h-16 rounded-full bg-semantic-danger/20 flex items-center justify-center">
        <span className="text-2xl">!</span>
      </div>
      <h2 className="text-lg font-heading font-semibold">{t('common.error', 'Something went wrong')}</h2>
      <p className="text-sm text-brand-text-muted text-center">
        {error?.message || t('common.unexpected_error', 'An unexpected error occurred.')}
      </p>
      <button
        onClick={reset}
        className="px-6 py-2 bg-brand-primary text-white rounded-full font-semibold hover:bg-brand-primary-hover transition-colors"
      >
        {t('common.try_again', 'Try again')}
      </button>
    </div>
  );
}

export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = { hasError: false, error: null };

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    this.props.onError?.(error, info);
  }

  componentDidUpdate(prevProps: ErrorBoundaryProps) {
    if (!this.state.hasError) return;
    if (this.props.resetKeys && prevProps.resetKeys && this.props.resetKeys !== prevProps.resetKeys) {
      this.reset();
    }
  }

  private reset = () => {
    this.setState({ hasError: false, error: null });
    this.props.onReset?.();
  };

  handleReset = () => {
    this.reset();
  };

  render() {
    if (this.state.hasError) {
      const { fallback } = this.props;
      if (typeof fallback === 'function') {
        return (fallback as (error: Error, reset: () => void) => ReactNode)(
          this.state.error!,
          this.handleReset
        );
      }
      if (fallback) return fallback;
      return <DefaultFallback error={this.state.error} reset={this.handleReset} />;
    }
    return this.props.children;
  }
}

export function withErrorBoundary<P extends Record<string, unknown>>(
  Component: ComponentType<P>,
  errorBoundaryProps: Omit<ErrorBoundaryProps, 'children'> = {}
): ComponentType<P> {
  const displayName = Component.displayName || Component.name || 'Component';
  const Wrapped: ComponentType<P> = (props: P) => (
    <ErrorBoundary {...errorBoundaryProps}>
      <Component {...props} />
    </ErrorBoundary>
  );
  Wrapped.displayName = `withErrorBoundary(${displayName})`;
  return Wrapped;
}
