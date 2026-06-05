import { Component, type ReactNode, type ErrorInfo } from 'react';

interface OfflineBannerProps {
  show?: boolean;
}

export function OfflineBanner({ show }: OfflineBannerProps) {
  if (!show) return null;
  return (
    <div className="fixed bottom-0 left-0 right-0 z-toast bg-semantic-warning text-white text-center py-2 px-4 text-sm font-medium">
      Nuk jeni të lidhur me internetin
    </div>
  );
}

interface ErrorBoundaryProps {
  children: ReactNode;
  fallback?: ReactNode;
}

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = { hasError: false, error: null };

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error('ErrorBoundary caught:', error, info);
  }

  render() {
    if (this.state.hasError) {
      if (this.props.fallback) return this.props.fallback;
      return (
        <div className="flex flex-col items-center justify-center min-h-screen bg-brand-bg text-brand-text p-6 gap-4">
          <div className="w-16 h-16 rounded-full bg-semantic-danger/20 flex items-center justify-center">
            <span className="text-2xl">!</span>
          </div>
          <h2 className="text-lg font-heading font-semibold">Diçka shkoi keq</h2>
          <p className="text-sm text-brand-text-muted text-center">
            {this.state.error?.message || 'Një gabim i papritur ndodhi.'}
          </p>
          <button
            onClick={() => window.location.reload()}
            className="px-6 py-2 bg-brand-primary text-white rounded-full font-semibold hover:bg-brand-primary-hover transition-colors"
          >
            Provo përsëri
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}
