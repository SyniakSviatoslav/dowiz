const variants: Record<string, string> = {
  primary: `bg-brand-primary text-brand-bg font-semibold shadow-[var(--elevation-1)] hover:bg-brand-primary-hover`,
};
const B = `bg-brand-primary text-brand-bg`;
export function D() {
  return <div className="bg-brand-primary text-brand-bg">x</div>;
}
