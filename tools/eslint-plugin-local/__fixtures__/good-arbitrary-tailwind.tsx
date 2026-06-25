// GREEN fixture — design-scale classes + token var() instead of arbitrary values.
export function Good() {
  return (
    <div className="p-3 w-24 gap-2" style={{ color: 'var(--brand-primary)' }}>
      on-scale spacing, token colour
    </div>
  );
}
