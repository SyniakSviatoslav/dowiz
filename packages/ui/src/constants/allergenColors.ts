export const ALLERGEN_COLORS: Record<string, { bg: string; text: string }> = {
  gluten: { bg: 'rgba(234,179,8,0.12)', text: '#a16207' },
  dairy: { bg: 'rgba(59,130,246,0.12)', text: '#1d4ed8' },
  eggs: { bg: 'rgba(234,179,8,0.12)', text: '#a16207' },
  soy: { bg: 'rgba(34,197,94,0.12)', text: '#15803d' },
  nuts: { bg: 'rgba(249,115,22,0.12)', text: '#c2410c' },
  peanuts: { bg: 'rgba(249,115,22,0.12)', text: '#c2410c' },
  shellfish: { bg: 'rgba(239,68,68,0.12)', text: '#b91c1c' },
  fish: { bg: 'rgba(6,182,212,0.12)', text: '#0e7490' },
  sesame: { bg: 'rgba(168,85,247,0.12)', text: '#7e22ce' },
};

export function getAllergenStyle(allergen: string) {
  const key = allergen.toLowerCase();
  return ALLERGEN_COLORS[key] || { bg: 'rgba(107,114,128,0.12)', text: '#374151' };
}
