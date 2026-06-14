import { z } from 'zod';

export type Severity = 'BLOCKER' | 'MAJOR' | 'MINOR';
export type FindingLayer = 'EDGE' | 'APP' | 'DATA' | 'CONTRACT';
export type FindingStatus = 'NEW' | 'KNOWN' | 'REGRESSED' | 'RESOLVED';

export interface Finding {
  id: string;
  layer: FindingLayer;
  severity: Severity;
  target: string;
  expected: string;
  actual: string;
  evidence: string;
  status: FindingStatus;
  first_seen: string;
  last_seen: string;
  spec_ref: string;
}

export interface AuditRun {
  run_id: string;
  env: string;
  base_url: string;
  verdict: 'GO' | 'NO-GO';
  trigger: 'watchdog' | 'post_deploy' | 'nightly' | 'manual';
  timestamp: string;
  findings: Finding[];
  health_check: Record<string, unknown>;
  summary: {
    total_checks: number;
    green: number;
    red: number;
    flaky: number;
    blocked: number;
    blockers: number;
    majors: number;
    minors: number;
  };
}

export const FindingSchema = z.object({
  id: z.string(),
  layer: z.enum(['EDGE', 'APP', 'DATA', 'CONTRACT']),
  severity: z.enum(['BLOCKER', 'MAJOR', 'MINOR']),
  target: z.string(),
  expected: z.string(),
  actual: z.string(),
  evidence: z.string(),
  status: z.enum(['NEW', 'KNOWN', 'REGRESSED', 'RESOLVED']),
  first_seen: z.string(),
  last_seen: z.string(),
  spec_ref: z.string(),
});

export const AuditRunSchema = z.object({
  run_id: z.string(),
  env: z.string(),
  base_url: z.string(),
  verdict: z.enum(['GO', 'NO-GO']),
  trigger: z.enum(['watchdog', 'post_deploy', 'nightly', 'manual']),
  timestamp: z.string(),
  findings: z.array(FindingSchema),
  health_check: z.record(z.unknown()),
  summary: z.object({
    total_checks: z.number(),
    green: z.number(),
    red: z.number(),
    flaky: z.number(),
    blocked: z.number(),
    blockers: z.number(),
    majors: z.number(),
    minors: z.number(),
  }),
});

export function makeFindingId(layer: string, target: string, spec_ref: string): string {
  const key = `${layer}:${target}:${spec_ref}`;
  return `F-${simpleHash(key)}`;
}

function simpleHash(str: string): string {
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    const char = str.charCodeAt(i);
    hash = ((hash << 5) - hash) + char;
    hash = hash & hash;
  }
  return Math.abs(hash).toString(16).substring(0, 8);
}
