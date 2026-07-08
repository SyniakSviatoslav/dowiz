// Bebop token ledger — central, cross-backend token accounting (RESEARCH.md §1.6).
//
// The operator's token-economy rule: usage is tallied into ONE ledger regardless of which CLI spent
// it. Every backend (Claude Code, Hermes, OpenCode, Codex, Aider, Goose, native) reports through this
// single seam. No backend meters itself.

export interface TokenEntry {
  backend: string;
  task: string;
  promptTokens: number;
  completionTokens: number;
  at: number; // caller-supplied timestamp (core never reads a clock)
}

export interface Ledger {
  entries: TokenEntry[];
  totalPrompt: number;
  totalCompletion: number;
}

export function emptyLedger(): Ledger {
  return { entries: [], totalPrompt: 0, totalCompletion: 0 };
}

export function record(ledger: Ledger, e: TokenEntry): Ledger {
  return {
    entries: [...ledger.entries, e],
    totalPrompt: ledger.totalPrompt + e.promptTokens,
    totalCompletion: ledger.totalCompletion + e.completionTokens,
  };
}

export function totalTokens(l: Ledger): number {
  return l.totalPrompt + l.totalCompletion;
}

/** Per-backend breakdown — proves the ledger is unified, not per-tool. */
export function byBackend(l: Ledger): Record<string, number> {
  const out: Record<string, number> = {};
  for (const e of l.entries) {
    const k = e.backend;
    out[k] = (out[k] ?? 0) + e.promptTokens + e.completionTokens;
  }
  return out;
}
