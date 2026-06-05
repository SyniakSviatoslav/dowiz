interface SkeletonProps {
  className?: string;
}

export function Skeleton({ className = '' }: SkeletonProps) {
  return (
    <div className={`animate-pulse bg-brand-border rounded-md ${className}`} />
  );
}

export function SkeletonCard() {
  return (
    <div className="bg-brand-surface rounded-lg p-4 space-y-3">
      <Skeleton className="h-32 w-full rounded-md" />
      <Skeleton className="h-4 w-3/4" />
      <Skeleton className="h-3 w-1/2" />
      <Skeleton className="h-8 w-full" />
    </div>
  );
}
