// @ts-nocheck
export function getSettlementPeriodBoundaries(
  referenceDate: Date = new Date(),
  periodType: 'daily' | 'weekly' = 'daily'
): { periodStart: Date; periodEnd: Date } {
  const periodEnd = new Date(referenceDate);
  periodEnd.setUTCHours(0, 0, 0, 0);

  const periodStart = new Date(periodEnd);
  
  if (periodType === 'weekly') {
    // periodStart = previous Monday
    // periodEnd = current Monday
    const day = periodStart.getUTCDay(); // 0 is Sunday, 1 is Monday
    const diff = (day === 0 ? -6 : 1) - day; // Go back to Monday
    periodStart.setUTCDate(periodStart.getUTCDate() + diff);
    periodStart.setUTCDate(periodStart.getUTCDate() - 7);
    
    // Set periodEnd to the Monday after periodStart
    periodEnd.setTime(periodStart.getTime());
    periodEnd.setUTCDate(periodEnd.getUTCDate() + 7);
  } else {
    // Daily
    periodStart.setUTCDate(periodStart.getUTCDate() - 1);
  }

  return { periodStart, periodEnd };
}
