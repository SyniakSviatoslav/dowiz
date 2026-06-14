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
