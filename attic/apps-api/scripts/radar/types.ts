export type FlowStatus = 'UNKNOWN' | 'OK' | 'DIVERGENCE' | 'BLOCKED';
export type Severity = '🔴' | '🟠' | '🟡' | '🔵' | '⚪';

export type SurfaceType = 'db' | 'queue' | 'ws' | 'telegram' | 'push' | 'audit' | 'http';

export interface ExpectedEffect {
  surface: SurfaceType;
  description: string;
  source: string; // file:line where the contract is defined
}

export interface FlowEntry {
  id: string;
  description: string;
  trigger: string;
  expectedEffects: ExpectedEffect[];
}

export interface ProbeResult {
  flowId: string;
  status: FlowStatus;
  durationMs: number;
  actual: Record<string, any>;
  divergence?: {
    expected: string;
    actual: string;
    evidence: string;
    severity: Severity;
    rootCauseHypothesis: string;
  }[];
  blockedReason?: string;
}

export interface RadarReport {
  timestamp: string;
  target: string;
  total: number;
  ok: number;
  divergences: number;
  blocked: number;
  results: ProbeResult[];
  clusterByRoot?: Map<string, ProbeResult[]>;
}

export function formatReport(report: RadarReport): string {
  const lines: string[] = [];
  lines.push(`# Radar Report — ${report.timestamp}`);
  lines.push(`Target: ${report.target}`);
  lines.push('');
  lines.push(`## Summary`);
  lines.push(`- Total flows: ${report.total}`);
  lines.push(`- ✅ OK: ${report.ok}`);
  lines.push(`- 🔴 Divergences: ${report.divergences}`);
  lines.push(`- ⚪ Blocked: ${report.blocked}`);
  lines.push('');

  const divergences = report.results.filter(r => r.status === 'DIVERGENCE');
  if (divergences.length > 0) {
    lines.push('## 🔴 Divergences');
    for (const d of divergences) {
      lines.push(`### ${d.flowId}`);
      for (const div of d.divergence || []) {
        lines.push(`- ${div.severity} **Expected:** ${div.expected}`);
        lines.push(`  **Actual:** ${div.actual}`);
        lines.push(`  **Evidence:** ${div.evidence}`);
        lines.push(`  **Hypothesis:** ${div.rootCauseHypothesis}`);
      }
    }
    lines.push('');
  }

  const blocked = report.results.filter(r => r.status === 'BLOCKED');
  if (blocked.length > 0) {
    lines.push('## ⚪ Blocked');
    for (const b of blocked) {
      lines.push(`- ${b.flowId}: ${b.blockedReason}`);
    }
    lines.push('');
  }

  lines.push('## Full Results');
  for (const r of report.results) {
    const icon = r.status === 'OK' ? '✅' : r.status === 'DIVERGENCE' ? '🔴' : '⚪';
    lines.push(`| ${icon} ${r.flowId} | ${r.status} | ${r.durationMs}ms |`);
  }

  return lines.join('\n');
}
